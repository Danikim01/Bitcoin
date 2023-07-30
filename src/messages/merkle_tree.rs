use crate::utility::double_hash;
use bitcoin_hashes::sha256;
use std::io::Error;

/// Enum that represents the direction of a node in a merkle tree
#[derive(Clone, Debug)]
pub enum Direction {
    Left,
    Right,
}

/// Struct that represents a merkle proof
#[derive(Clone, Debug)]
pub struct MerkleProof {
    pub proof: Vec<(sha256::Hash, Direction)>,
}

impl MerkleProof {
    fn find_root(&self, hash: sha256::Hash, index: usize) -> sha256::Hash {
        if index > self.proof.len() - 1 {
            return hash;
        }

        let (hash2, direction2) = self.proof[index].clone();

        match direction2 {
            Direction::Right => {
                let combined_hash = double_hash(&[&hash[..], &hash2[..]].concat());
                self.find_root(combined_hash, index + 1)
            }
            Direction::Left => {
                let combined_hash = double_hash(&[&hash2[..], &hash[..]].concat());
                self.find_root(combined_hash, index + 1)
            }
        }
    }

    pub fn generate_merkle_root(&self) -> sha256::Hash {
        if self.proof.is_empty() {
            return double_hash(b"foo");
        }

        let (h, _) = self.proof[0].clone();
        self.find_root(h, 1)
    }
}

/// Struct that represents a merkle tree (as array of arrays of hashes)
#[derive(Debug, PartialEq)]
pub struct MerkleTree {
    pub tree: Vec<Vec<sha256::Hash>>,
}

// Doc: https://developer.bitcoin.org/reference/block_chain.html#merkle-trees
// Reference: https://medium.com/coinmonks/merkle-tree-a-simple-explanation-and-implementation-48903442bc08
impl MerkleTree {
    fn get_leaf_node_direction(&self, hash_leaf: sha256::Hash) -> Direction {
        let mut hash_index = 0;
        for hash in self.tree[0].iter() {
            if hash == &hash_leaf {
                break;
            }
            hash_index += 1;
        }

        if hash_index % 2 == 0 {
            return Direction::Left;
        }
        Direction::Right
        // this function does not contemplate the chance that the hash is not in the tree
        // yet it is not needed as the generated proof will fail later on
    }

    fn ensure_even(hashes: Vec<sha256::Hash>) -> Vec<sha256::Hash> {
        if hashes.len() % 2 != 0 {
            let mut new_hashes = hashes.clone();
            let last_hash = hashes[hashes.len() - 1];
            new_hashes.push(last_hash);
            return new_hashes;
        }
        hashes
    }

    /// Returns the root of the merkle tree
    pub fn get_root(&self) -> sha256::Hash {
        if self.tree.is_empty() {
            return double_hash(b"foo");
        }
        self.tree[self.tree.len() - 1][0]
    }

    pub fn _merkle_root_from_hashes(hashes: Vec<sha256::Hash>) -> Result<sha256::Hash, Error> {
        if hashes.is_empty() {
            return Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot create a Merkle tree from an empty list of hashes.",
            ));
        }

        // Merkle root found
        if hashes.len() == 1 {
            return Ok(hashes[0]);
        }

        let hashes = Self::ensure_even(hashes);
        let mut combined_hashes: Vec<sha256::Hash> = Vec::new();
        for i in (0..hashes.len()).step_by(2) {
            let left_hash = hashes[i];
            let right_hash = hashes[i + 1];
            let combined_hash = double_hash(&[&left_hash[..], &right_hash[..]].concat());
            combined_hashes.push(combined_hash);
        }

        Self::_merkle_root_from_hashes(combined_hashes)
    }

    /// Generates a merkle tree from a list of hashes
    pub fn generate_from_hashes(hashes: Vec<sha256::Hash>) -> Self {
        if hashes.is_empty() {
            return Self { tree: Vec::new() };
        }

        let mut tree: Vec<Vec<sha256::Hash>> = Vec::new();
        tree.push(hashes.clone());
        fn generate(hashes: Vec<sha256::Hash>, tree: &mut Vec<Vec<sha256::Hash>>) {
            // Merkle root found
            if hashes.len() == 1 {
                return;
            }

            let hashes = MerkleTree::ensure_even(hashes);
            let mut combined_hashes: Vec<sha256::Hash> = Vec::new();
            for i in (0..hashes.len()).step_by(2) {
                let left_hash = hashes[i];
                let right_hash = hashes[i + 1];
                let combined_hash = double_hash(&[&left_hash[..], &right_hash[..]].concat());
                combined_hashes.push(combined_hash);
            }
            tree.push(combined_hashes.clone());
            generate(combined_hashes, tree);
        }
        generate(hashes, &mut tree);
        Self { tree }
    }

    /// Generates a Merkle proof for a given hash
    pub fn generate_proof(&self, hash: sha256::Hash) -> Result<MerkleProof, Error> {
        let mut proof: Vec<(sha256::Hash, Direction)> = Vec::new();
        proof.push((hash, self.get_leaf_node_direction(hash)));

        let mut hash_index = 0;
        for h in self.tree[0].iter() {
            if h == &hash {
                break;
            }
            hash_index += 1;
        }

        for level in 0..(self.tree.len() - 1) {
            let is_left_child = hash_index % 2 == 0;
            let sibling_direction = if is_left_child {
                Direction::Right
            } else {
                Direction::Left
            };

            let mut sibling_index = if is_left_child {
                hash_index + 1
            } else {
                hash_index - 1
            };

            // This means that the hash is the last one in the tree
            if sibling_index >= self.tree[level].len() {
                sibling_index = self.tree[level].len() - 1;
            }

            let sibling_node: (sha256::Hash, Direction) =
                (self.tree[level][sibling_index], sibling_direction);
            proof.push(sibling_node);
            hash_index /= 2 // shouldn't need to floor as it's a usize
        }

        Ok(MerkleProof { proof })
    }
}

#[cfg(test)]
mod tests {
    use bitcoin_hashes::Hash;

    use crate::{raw_transaction::RawTransaction, utility::decode_hex};

    use super::*;

    #[test]
    fn test_merkle_root_from_no_elements() {
        let empty_hashes: Vec<sha256::Hash> = Vec::new();
        assert!(MerkleTree::_merkle_root_from_hashes(empty_hashes).is_err());
    }

    #[test]
    fn test_merkle_root_from_one_element() {
        // If a block only has a coinbase transaction, the coinbase TXID is used as the merkle root hash.
        // "Transactions"
        let a = b"a";

        // Transactions hashes
        let a_hash = double_hash(a);

        let txid_hashes = vec![a_hash];

        // Actual merkle root hash
        let actual_hash = MerkleTree::_merkle_root_from_hashes(txid_hashes).unwrap();

        assert_eq!(actual_hash, a_hash);
    }

    #[test]
    fn test_merkle_root_from_two_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);

        let txid_hashes = vec![a_hash, b_hash];

        // Expected merkle root
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());

        // Actual merkle root
        let actual_hash = MerkleTree::_merkle_root_from_hashes(txid_hashes).unwrap();
        assert_eq!(actual_hash, ab_hash);
    }

    #[test]
    fn test_merkle_root_from_three_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);
        let c_hash = double_hash(c);
        let txid_hashes = vec![a_hash, b_hash, c_hash];

        // Expected merkle root
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());
        let cc_hash = double_hash(&[&c_hash[..], &c_hash[..]].concat());
        let abcc_hash = double_hash(&[&ab_hash[..], &cc_hash[..]].concat());

        // Actual merkle root
        let actual_hash = MerkleTree::_merkle_root_from_hashes(txid_hashes).unwrap();
        assert_eq!(actual_hash, abcc_hash);
    }

    #[test]
    fn test_merkle_root_from_four_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";
        let d = b"d";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);
        let c_hash = double_hash(c);
        let d_hash = double_hash(d);

        let txid_hashes = vec![a_hash, b_hash, c_hash, d_hash];

        // Expected merkle root
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());
        let cd_hash = double_hash(&[&c_hash[..], &d_hash[..]].concat());
        let abcd_hash = double_hash(&[&ab_hash[..], &cd_hash[..]].concat());

        // Actual merkle root
        let actual_hash = MerkleTree::_merkle_root_from_hashes(txid_hashes).unwrap();
        assert_eq!(actual_hash, abcd_hash);
    }

    #[test]
    fn test_merkle_root_from_nine_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";
        let d = b"d";
        let e = b"e";
        let f = b"f";
        let g = b"g";
        let h = b"h";
        let i = b"i";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);
        let c_hash = double_hash(c);
        let d_hash = double_hash(d);
        let e_hash = double_hash(e);
        let f_hash = double_hash(f);
        let g_hash = double_hash(g);
        let h_hash = double_hash(h);
        let i_hash = double_hash(i);

        let txid_hashes = vec![
            a_hash, b_hash, c_hash, d_hash, e_hash, f_hash, g_hash, h_hash, i_hash,
        ];

        // Expected merkle root
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());
        let cd_hash = double_hash(&[&c_hash[..], &d_hash[..]].concat());
        let ef_hash = double_hash(&[&e_hash[..], &f_hash[..]].concat());
        let gh_hash = double_hash(&[&g_hash[..], &h_hash[..]].concat());
        let ii_hash = double_hash(&[&i_hash[..], &i_hash[..]].concat());

        let abcd_hash = double_hash(&[&ab_hash[..], &cd_hash[..]].concat());
        let efgh_hash = double_hash(&[&ef_hash[..], &gh_hash[..]].concat());
        let iiii_hash = double_hash(&[&ii_hash[..], &ii_hash[..]].concat());

        let abcdefgh_hash = double_hash(&[&abcd_hash[..], &efgh_hash[..]].concat());
        let iiiiii_hash = double_hash(&[&iiii_hash[..], &iiii_hash[..]].concat());

        let abcdefghii_hash = double_hash(&[&abcdefgh_hash[..], &iiiiii_hash[..]].concat());

        // Actual merkle root
        let actual_hash = MerkleTree::_merkle_root_from_hashes(txid_hashes).unwrap();
        assert_eq!(actual_hash, abcdefghii_hash);
    }

    #[test]
    fn test_merkle_tree_from_one_element() {
        // If a block only has a coinbase transaction, the coinbase TXID is used as the merkle root hash.
        // "Transactions"
        let a = b"a";

        // Transactions hashes
        let a_hash = double_hash(a);

        let txid_hashes = vec![a_hash];

        // Expected merkle tree
        let expected_tree = vec![vec![a_hash]];

        // Actual merkle tree
        let actual_tree = MerkleTree::generate_from_hashes(txid_hashes);

        assert_eq!(actual_tree.tree, expected_tree);
    }

    #[test]
    fn test_merkle_tree_from_two_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());

        let txid_hashes = vec![a_hash, b_hash];

        // Expected merkle tree
        let expected_tree = vec![vec![a_hash, b_hash], vec![ab_hash]];

        // Actual merkle tree
        let actual_tree = MerkleTree::generate_from_hashes(txid_hashes);

        assert_eq!(actual_tree.tree, expected_tree);
    }

    #[test]
    fn test_merkle_tree_from_three_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);
        let c_hash = double_hash(c);
        let txid_hashes = vec![a_hash, b_hash, c_hash];

        // Expected merkle tree
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());
        let cc_hash = double_hash(&[&c_hash[..], &c_hash[..]].concat());
        let abcc_hash = double_hash(&[&ab_hash[..], &cc_hash[..]].concat());
        let expected_tree = vec![
            vec![a_hash, b_hash, c_hash], // Leaf nodes
            vec![ab_hash, cc_hash],       // Level 1
            vec![abcc_hash],              // merkle root
        ];

        // Generate merkle tree
        let actual_tree = MerkleTree::generate_from_hashes(txid_hashes);

        assert_eq!(actual_tree.tree, expected_tree);
    }

    #[test]
    fn test_generate_merkle_root_from_proof() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";
        let d = b"d";

        // Transactions hashes
        let a_hash = double_hash(a);
        let b_hash = double_hash(b);
        let c_hash = double_hash(c);
        let d_hash = double_hash(d);
        let ab_hash = double_hash(&[&a_hash[..], &b_hash[..]].concat());
        let cd_hash = double_hash(&[&c_hash[..], &d_hash[..]].concat());
        let abcd_hash = double_hash(&[&ab_hash[..], &cd_hash[..]].concat()); // Expected merkle root

        let txid_hashes = vec![a_hash, b_hash, c_hash, d_hash];
        let actual_tree = MerkleTree::generate_from_hashes(txid_hashes.clone());

        // iterate all elements in the tree and validate their proof
        for transaction in txid_hashes {
            let proof = actual_tree.generate_proof(transaction).unwrap();
            let merkle_root = proof.generate_merkle_root();
            assert_eq!(merkle_root, abcd_hash);
        }

        // alien transaction should fail to generate correct proof
        let alien_transaction = double_hash(b"alien");
        let alien_proof = actual_tree.generate_proof(alien_transaction).unwrap();
        let bad_merkle_root = alien_proof.generate_merkle_root();
        assert_ne!(bad_merkle_root, abcd_hash);
    }

    #[test]
    fn test_merkle_tree_from_raw_transactions() {
        let tx1_bytes = decode_hex("020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff2303aba925044428c1644d65726d6169646572204654572101000023f5cb010000000000ffffffff02ce80250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed8e2fa0dcf35a1c3853030613ab9fe45ff77255df36484815fd2899d8675e3d180120000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let mut cursor = std::io::Cursor::new(&tx1_bytes[..]);
        let tx1 = RawTransaction::from_bytes(&mut cursor).unwrap();
        let tx1_hash = double_hash(&tx1.serialize());

        let tx2_bytes = decode_hex("02000000000101fad25ca83a41395a00dec1a6bc20ee52ec413984358157d697fc09d53091c2e50100000017160014038e5730357e5631b6a5626df15a244ab0a7d9e8fdffffff0260b0d7c50e0000001600143c898dff9dd73d780d846a61a65a7cbfa871a81d30420500000000001600144cf6537ae378d52ab13c4fe5a0d52808dbfc75ef02473044022011fc8d6b5b350ae40b44093e4ca7aa0e19a60fb835362da365c86636df6d1e3902205278495b8c7cf237bf12561665b6858715c57e1bdb65a0525c018ee054d3960d012103cc957cab76d1677ae3547e7654096f392d3b3784acb29075830fdd72d1361a0baaa92500").unwrap();
        let mut cursor = std::io::Cursor::new(&tx2_bytes[..]);
        let tx2 = RawTransaction::from_bytes(&mut cursor).unwrap();
        let tx2_hash = double_hash(&tx2.serialize());

        let hash_tx_vec = vec![tx1_hash, tx2_hash];
        let merkle_tree = MerkleTree::generate_from_hashes(hash_tx_vec);

        let merkle_root = merkle_tree.get_root();
        let mut expected =
            decode_hex("a3b3097e67e3d002c36400e685575f41bb0a3215b7ca92f0a79b8a4f5d38075f").unwrap();
        expected.reverse();
        let expected_sha256 = sha256::Hash::from_slice(&expected).unwrap();
        assert_eq!(expected_sha256, merkle_root);
    }

    fn reverse_hex_str(hex: &str) -> String {
        let mut reversed_hex = String::new();
        let chars = hex.chars().collect::<Vec<char>>();
        for i in (0..chars.len()).step_by(2) {
            let mut byte = String::new();
            byte.push(chars[i]);
            byte.push(chars[i + 1]);
            reversed_hex = format!("{}{}", byte, reversed_hex);
        }
        reversed_hex
    }

    #[test]
    fn test_merkle_root_valid_poi() {
        let tx1_bytes = decode_hex("020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff2303aba925044428c1644d65726d6169646572204654572101000023f5cb010000000000ffffffff02ce80250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed8e2fa0dcf35a1c3853030613ab9fe45ff77255df36484815fd2899d8675e3d180120000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let mut cursor = std::io::Cursor::new(&tx1_bytes[..]);
        let tx1 = RawTransaction::from_bytes(&mut cursor).unwrap();
        let tx1_hash = double_hash(&tx1.serialize());

        let tx2_bytes = decode_hex("02000000000101fad25ca83a41395a00dec1a6bc20ee52ec413984358157d697fc09d53091c2e50100000017160014038e5730357e5631b6a5626df15a244ab0a7d9e8fdffffff0260b0d7c50e0000001600143c898dff9dd73d780d846a61a65a7cbfa871a81d30420500000000001600144cf6537ae378d52ab13c4fe5a0d52808dbfc75ef02473044022011fc8d6b5b350ae40b44093e4ca7aa0e19a60fb835362da365c86636df6d1e3902205278495b8c7cf237bf12561665b6858715c57e1bdb65a0525c018ee054d3960d012103cc957cab76d1677ae3547e7654096f392d3b3784acb29075830fdd72d1361a0baaa92500").unwrap();
        let mut cursor = std::io::Cursor::new(&tx2_bytes[..]);
        let tx2 = RawTransaction::from_bytes(&mut cursor).unwrap();
        let tx2_hash = double_hash(&tx2.serialize());

        let hash_tx_vec = vec![tx1_hash, tx2_hash];
        let merkle_tree = MerkleTree::generate_from_hashes(hash_tx_vec);

        let mut expected =
            decode_hex("a3b3097e67e3d002c36400e685575f41bb0a3215b7ca92f0a79b8a4f5d38075f").unwrap();
        expected.reverse();
        let expected_sha256 = sha256::Hash::from_slice(&expected).unwrap();

        let tx1_hash_str =
            reverse_hex_str("3412733ebdff59c8b28fed2b18b2a4fd60332fb08a5ca4b2ecfaea4e241fc081");
        let tx1_hash = tx1_hash_str.parse().unwrap();
        let proof_from_tx1 = merkle_tree.generate_proof(tx1_hash).unwrap();
        let merkle_root_from_tx1 = proof_from_tx1.generate_merkle_root();
        assert_eq!(expected_sha256, merkle_root_from_tx1);

        let tx2_hash_str =
            reverse_hex_str("3412733ebdff59c8b28fed2b18b2a4fd60332fb08a5ca4b2ecfaea4e241fc081");
        let tx2_hash = tx2_hash_str.parse().unwrap();
        let proof_from_tx2 = merkle_tree.generate_proof(tx2_hash).unwrap();
        let merkle_root_from_tx2 = proof_from_tx2.generate_merkle_root();
        assert_eq!(expected_sha256, merkle_root_from_tx2);
    }
}
