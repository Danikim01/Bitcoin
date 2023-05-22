use bitcoin_hashes::{sha256, Hash};

#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub hash: sha256::Hash,
    _left: Option<Box<MerkleNode>>,
    _right: Option<Box<MerkleNode>>,
}

#[derive(Debug)]
pub struct MerkleTree {
    pub root: Option<Box<MerkleNode>>,
}

// Doc: https://developer.bitcoin.org/reference/block_chain.html#merkle-trees
// Guide: https://www.derpturkey.com/merkle-tree-construction-and-proof-of-inclusion/
impl MerkleTree {
    pub fn from_hashes(hashes: Vec<sha256::Hash>) -> Self {
        // Map all hashes to MerkleNodes
        let mut children = hashes
            .into_iter()
            .map(|hash| {
                Box::new(MerkleNode {
                    hash,
                    _left: None,
                    _right: None,
                })
            })
            .collect::<Vec<_>>();

        // iterate children until there is only one left, which is the root
        while children.len() > 1 {
            let mut parents: Vec<Box<MerkleNode>> = Vec::new();
            // iterate in pairs
            let childs_iter = children.chunks(2);
            for pair in childs_iter {
            // while let Some(pair) = childs_iter.next() {
                // Case only one child in the pair
                if pair.len() == 1 {
                    let mut hash =
                        sha256::Hash::hash(&[&pair[0].hash[..], &pair[0].hash[..]].concat());
                    hash = sha256::Hash::hash(&hash[..]);
                    parents.push(Box::new(MerkleNode {
                        hash,
                        _left: Some(pair[0].clone()),
                        _right: None,
                    }));
                } else {
                    let mut hash =
                        sha256::Hash::hash(&[&pair[0].hash[..], &pair[1].hash[..]].concat());
                    hash = sha256::Hash::hash(&hash[..]);
                    parents.push(Box::new(MerkleNode {
                        hash,
                        _left: Some(pair[0].clone()),
                        _right: Some(pair[1].clone()),
                    }));
                }
            }
            children = parents.clone();
        }

        // return tree with root
        if let Some(root) = children.pop() {
            return Self { root: Some(root) };
        }

        Self { root: None } // something wrong happened
    }

    pub fn _get_root_hash(&self) -> Option<sha256::Hash> {
        if let Some(root) = &self.root {
            return Some(root.hash);
        }
        None
    }
    

    pub fn _validate_inclusion_recursive(node: &MerkleNode, hash: sha256::Hash) -> bool {
        if node.hash == hash {
            return true;
        }
        if let Some(left) = &node._left {
            if Self::_validate_inclusion_recursive(left, hash) {
                return true;
            }
        }
        if let Some(right) = &node._right {
            if Self::_validate_inclusion_recursive(right, hash) {
                return true;
            }
        }
        false
    }

    pub fn _validate_inclusion(&self, hash: sha256::Hash) -> bool {
        if let Some(root) = self.root.as_ref() {
            return Self::_validate_inclusion_recursive(root, hash);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_one_element() {
        // If a block only has a coinbase transaction, the coinbase TXID is used as the merkle root hash.
        // "Transactions"
        let a = b"a";

        // Transactions hashes
        let mut a_hash = sha256::Hash::hash(a);
        a_hash = sha256::Hash::hash(&a_hash[..]); // this is the expected merkle root hash

        let txid_hashes = vec![a_hash];

        // Merkle tree
        let merkle_tree = MerkleTree::from_hashes(txid_hashes);

        // Actual merkle root hash
        let actual_hash = merkle_tree._get_root_hash().unwrap();
        println!("\nExpected: {:?}\n\n Actual: {:?}\n", a_hash, actual_hash);
        assert_eq!(actual_hash, a_hash);
    }

    #[test]
    fn test_merkle_tree_two_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";

        // Transactions hashes
        let mut a_hash = sha256::Hash::hash(a);
        a_hash = sha256::Hash::hash(&a_hash[..]);
        let mut b_hash = sha256::Hash::hash(b);
        b_hash = sha256::Hash::hash(&b_hash[..]);

        let txid_hashes = vec![a_hash, b_hash];
        let merkle_tree = MerkleTree::from_hashes(txid_hashes);

        // Expected merkle root
        let mut ab_hash = sha256::Hash::hash(&[&a_hash[..], &b_hash[..]].concat());
        ab_hash = sha256::Hash::hash(&ab_hash[..]);

        // Actual merkle root
        let actual_hash = merkle_tree._get_root_hash().unwrap();
        println!("\nExpected: {:?}\n\n Actual: {:?}\n", ab_hash, actual_hash);
        assert_eq!(actual_hash, ab_hash);
    }

    #[test]
    fn test_merkle_tree_three_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";

        // Transactions hashes
        let mut a_hash = sha256::Hash::hash(a);
        a_hash = sha256::Hash::hash(&a_hash[..]);
        let mut b_hash = sha256::Hash::hash(b);
        b_hash = sha256::Hash::hash(&b_hash[..]);
        let mut c_hash = sha256::Hash::hash(c);
        c_hash = sha256::Hash::hash(&c_hash[..]);

        let txid_hashes = vec![a_hash, b_hash, c_hash];

        // Merkle tree
        let merkle_tree = MerkleTree::from_hashes(txid_hashes);

        // Expected merkle root
        let mut ab_hash = sha256::Hash::hash(&[&a_hash[..], &b_hash[..]].concat());
        ab_hash = sha256::Hash::hash(&ab_hash[..]);
        let mut cc_hash = sha256::Hash::hash(&[&c_hash[..], &c_hash[..]].concat());
        cc_hash = sha256::Hash::hash(&cc_hash[..]);
        let mut abcc_hash = sha256::Hash::hash(&[&ab_hash[..], &cc_hash[..]].concat());
        abcc_hash = sha256::Hash::hash(&abcc_hash[..]);

        // Actual merkle root
        let actual_hash = merkle_tree._get_root_hash().unwrap();
        assert_eq!(actual_hash, abcc_hash);
    }

    #[test]
    fn test_merkle_tree_four_elements() {
        // "Transactions"
        let a = b"a";
        let b = b"b";
        let c = b"c";
        let d = b"d";

        // Transactions hashes
        let mut a_hash = sha256::Hash::hash(a);
        a_hash = sha256::Hash::hash(&a_hash[..]);
        let mut b_hash = sha256::Hash::hash(b);
        b_hash = sha256::Hash::hash(&b_hash[..]);
        let mut c_hash = sha256::Hash::hash(c);
        c_hash = sha256::Hash::hash(&c_hash[..]);
        let mut d_hash = sha256::Hash::hash(d);
        d_hash = sha256::Hash::hash(&d_hash[..]);

        let txid_hashes = vec![a_hash, b_hash, c_hash, d_hash];

        // Merkle tree
        let merkle_tree = MerkleTree::from_hashes(txid_hashes);

        // Expected merkle root
        let mut ab_hash = sha256::Hash::hash(&[&a_hash[..], &b_hash[..]].concat());
        ab_hash = sha256::Hash::hash(&ab_hash[..]);
        let mut cd_hash = sha256::Hash::hash(&[&c_hash[..], &d_hash[..]].concat());
        cd_hash = sha256::Hash::hash(&cd_hash[..]);
        let mut abcd_hash = sha256::Hash::hash(&[&ab_hash[..], &cd_hash[..]].concat());
        abcd_hash = sha256::Hash::hash(&abcd_hash[..]);

        // Actual merkle root
        let actual_hash = merkle_tree._get_root_hash().unwrap();
        assert_eq!(actual_hash, abcd_hash);
    }

    #[test]
    fn test_merkle_root_nine_elements() {
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
        let mut a_hash = sha256::Hash::hash(a);
        a_hash = sha256::Hash::hash(&a_hash[..]);
        let mut b_hash = sha256::Hash::hash(b);
        b_hash = sha256::Hash::hash(&b_hash[..]);
        let mut c_hash = sha256::Hash::hash(c);
        c_hash = sha256::Hash::hash(&c_hash[..]);
        let mut d_hash = sha256::Hash::hash(d);
        d_hash = sha256::Hash::hash(&d_hash[..]);
        let mut e_hash = sha256::Hash::hash(e);
        e_hash = sha256::Hash::hash(&e_hash[..]);
        let mut f_hash = sha256::Hash::hash(f);
        f_hash = sha256::Hash::hash(&f_hash[..]);
        let mut g_hash = sha256::Hash::hash(g);
        g_hash = sha256::Hash::hash(&g_hash[..]);
        let mut h_hash = sha256::Hash::hash(h);
        h_hash = sha256::Hash::hash(&h_hash[..]);
        let mut i_hash = sha256::Hash::hash(i);
        i_hash = sha256::Hash::hash(&i_hash[..]);

        let txid_hashes = vec![
            a_hash, b_hash, c_hash, d_hash, e_hash, f_hash, g_hash, h_hash, i_hash,
        ];

        // Merkle tree
        let merkle_tree = MerkleTree::from_hashes(txid_hashes);

        // Expected merkle root
        let mut ab_hash = sha256::Hash::hash(&[&a_hash[..], &b_hash[..]].concat());
        ab_hash = sha256::Hash::hash(&ab_hash[..]);
        let mut cd_hash = sha256::Hash::hash(&[&c_hash[..], &d_hash[..]].concat());
        cd_hash = sha256::Hash::hash(&cd_hash[..]);
        let mut ef_hash = sha256::Hash::hash(&[&e_hash[..], &f_hash[..]].concat());
        ef_hash = sha256::Hash::hash(&ef_hash[..]);
        let mut gh_hash = sha256::Hash::hash(&[&g_hash[..], &h_hash[..]].concat());
        gh_hash = sha256::Hash::hash(&gh_hash[..]);
        let mut ii_hash = sha256::Hash::hash(&[&i_hash[..], &i_hash[..]].concat());
        ii_hash = sha256::Hash::hash(&ii_hash[..]);

        let mut abcd_hash = sha256::Hash::hash(&[&ab_hash[..], &cd_hash[..]].concat());
        abcd_hash = sha256::Hash::hash(&abcd_hash[..]);
        let mut efgh_hash = sha256::Hash::hash(&[&ef_hash[..], &gh_hash[..]].concat());
        efgh_hash = sha256::Hash::hash(&efgh_hash[..]);
        let mut iiii_hash = sha256::Hash::hash(&[&ii_hash[..], &ii_hash[..]].concat());
        iiii_hash = sha256::Hash::hash(&iiii_hash[..]);

        let mut abcdefgh_hash = sha256::Hash::hash(&[&abcd_hash[..], &efgh_hash[..]].concat());
        abcdefgh_hash = sha256::Hash::hash(&abcdefgh_hash[..]);
        let mut iiiiii_hash = sha256::Hash::hash(&[&iiii_hash[..], &iiii_hash[..]].concat());
        iiiiii_hash = sha256::Hash::hash(&iiiiii_hash[..]);

        let mut abcdefghii_hash =
            sha256::Hash::hash(&[&abcdefgh_hash[..], &iiiiii_hash[..]].concat());
        abcdefghii_hash = sha256::Hash::hash(&abcdefghii_hash[..]);

        // Actual merkle root
        let actual_hash = merkle_tree._get_root_hash().unwrap();
        assert_eq!(actual_hash, abcdefghii_hash);
    }

    #[test]
    fn test_validate_inclusion() {
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
        let mut a_hash = sha256::Hash::hash(a);
        a_hash = sha256::Hash::hash(&a_hash[..]);
        let mut b_hash = sha256::Hash::hash(b);
        b_hash = sha256::Hash::hash(&b_hash[..]);
        let mut c_hash = sha256::Hash::hash(c);
        c_hash = sha256::Hash::hash(&c_hash[..]);
        let mut d_hash = sha256::Hash::hash(d);
        d_hash = sha256::Hash::hash(&d_hash[..]);
        let mut e_hash = sha256::Hash::hash(e);
        e_hash = sha256::Hash::hash(&e_hash[..]);
        let mut f_hash = sha256::Hash::hash(f);
        f_hash = sha256::Hash::hash(&f_hash[..]);
        let mut g_hash = sha256::Hash::hash(g);
        g_hash = sha256::Hash::hash(&g_hash[..]);
        let mut h_hash = sha256::Hash::hash(h);
        h_hash = sha256::Hash::hash(&h_hash[..]);
        let mut i_hash = sha256::Hash::hash(i);
        i_hash = sha256::Hash::hash(&i_hash[..]);

        let txid_hashes = vec![
            a_hash, b_hash, c_hash, d_hash, e_hash, f_hash, g_hash, h_hash, i_hash,
        ];

        // Merkle tree
        let merkle_tree = MerkleTree::from_hashes(txid_hashes);

        assert!(merkle_tree._validate_inclusion(a_hash));

        // Not included in the tree
        let j = b"j";
        let j_hash = sha256::Hash::hash(j);

        assert!(!merkle_tree._validate_inclusion(j_hash));

    }
}
