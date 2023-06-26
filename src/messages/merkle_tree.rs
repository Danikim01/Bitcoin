use crate::utility::double_hash;
use bitcoin_hashes::sha256;
use std::io::Error;

/// Enum that represents the direction of a node in a merkle tree
#[derive(Clone)]
pub enum Direction {
    _Left,
    _Right,
}

/// Struct that represents a merkle proof
#[derive(Clone)]
pub struct MerkleProof {
    pub proof: Vec<(sha256::Hash, Direction)>,
}

impl MerkleProof {
    fn _find_root(&self, hash: sha256::Hash, index: usize) -> sha256::Hash {
        if index > self.proof.len() - 1 {
            return hash;
        }

        let (hash2, direction2) = self.proof[index].clone();

        match direction2 {
            Direction::_Right => {
                let combined_hash = double_hash(&[&hash[..], &hash2[..]].concat());
                self._find_root(combined_hash, index + 1)
            }
            Direction::_Left => {
                let combined_hash = double_hash(&[&hash2[..], &hash[..]].concat());
                self._find_root(combined_hash, index + 1)
            }
        }
    }

    pub fn _generate_merkle_root(&self) -> sha256::Hash {
        if self.proof.is_empty() {
            return double_hash(b"foo");
        }

        let (h, _) = self.proof[0].clone();
        self._find_root(h, 1)
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
    fn _get_leaf_node_direction(&self, hash_leaf: sha256::Hash) -> Direction {
        let mut hash_index = 0;
        for hash in self.tree[0].iter() {
            if hash == &hash_leaf {
                break;
            }
            hash_index += 1;
        }

        if hash_index % 2 == 0 {
            return Direction::_Left;
        }
        Direction::_Right
        // this function does not contemplate the chance that the hash is not in the tree
        // yet it is not needed as the generated proof will fail later on
    }

    fn _ensure_even(hashes: Vec<sha256::Hash>) -> Vec<sha256::Hash> {
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

        let hashes = Self::_ensure_even(hashes);
        let mut combined_hashes: Vec<sha256::Hash> = Vec::new();
        for i in (0..hashes.len()).step_by(2) {
            let _left_hash = hashes[i];
            let _right_hash = hashes[i + 1];
            let combined_hash = double_hash(&[&_left_hash[..], &_right_hash[..]].concat());
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

            let hashes = MerkleTree::_ensure_even(hashes);
            let mut combined_hashes: Vec<sha256::Hash> = Vec::new();
            for i in (0..hashes.len()).step_by(2) {
                let _left_hash = hashes[i];
                let _right_hash = hashes[i + 1];
                let combined_hash = double_hash(&[&_left_hash[..], &_right_hash[..]].concat());
                combined_hashes.push(combined_hash);
            }
            tree.push(combined_hashes.clone());
            generate(combined_hashes, tree);
        }
        generate(hashes, &mut tree);
        Self { tree }
    }

    /// Generates a Merkle proof for a given hash
    pub fn _generate_proof(&self, hash: sha256::Hash) -> Result<MerkleProof, Error> {
        let mut proof: Vec<(sha256::Hash, Direction)> = Vec::new();
        proof.push((hash, self._get_leaf_node_direction(hash)));

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
                Direction::_Right
            } else {
                Direction::_Left
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
            let proof = actual_tree._generate_proof(transaction).unwrap();
            let merkle_root = proof._generate_merkle_root();
            assert_eq!(merkle_root, abcd_hash);
        }

        // alien transaction should fail to generate correct proof
        let alien_transaction = double_hash(b"alien");
        let alien_proof = actual_tree._generate_proof(alien_transaction).unwrap();
        let bad_merkle_root = alien_proof._generate_merkle_root();
        assert_ne!(bad_merkle_root, abcd_hash);
    }
}
