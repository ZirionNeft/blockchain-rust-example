use std::{rc::Rc};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub left: Option<Rc<MerkleNode>>,
    pub right: Option<Rc<MerkleNode>>,
    pub data: Vec<u8>,
}

pub struct MerkleTree {
    pub root: MerkleNode,
}

impl MerkleNode {
    fn new(left: Option<Rc<Self>>, right: Option<Rc<Self>>, data: Option<Vec<u8>>) -> Self {
        let hash;
        
        if left.is_none() && right.is_none() {
            let data = data.unwrap();
            let mut hasher = Sha256::new();
            hasher.update(data);
            hash = hasher.finalize();

            Self {
                data: hash.to_vec(),
                left: None,
                right: None,
            }
        } else {
            let left = left.unwrap();
            let right = right.unwrap();

            let mut data = left.data.clone();
            data.append(&mut right.data.clone());

            let mut hasher = Sha256::new();
            hasher.update(data);
            hash = hasher.finalize();

            Self {
                data: hash.to_vec(),
                left: Some(Rc::clone(&left)),
                right: Some(Rc::clone(&right)),
            }
        }
    }
}


impl MerkleTree {
    pub fn new(mut data: Vec<Vec<u8>>) -> Self {
        let data_size = data.len();
        
        if data_size % 2 != 0 {
            data.push(data.last().unwrap().clone());
        }

        let mut nodes = Vec::<MerkleNode>::new();

        for bottom_node in data.into_iter() {
            let node = MerkleNode::new(None, None, Some(bottom_node));
            nodes.push(node);
        }

        for _i in 0..(data_size/2) {
            let mut next_level = Vec::<MerkleNode>::new();

            for w in nodes.windows(2) {
                if let [left, right] = w {
                    let node = MerkleNode::new(
                        Some(Rc::new(left.to_owned())),
                         Some(Rc::new(right.to_owned())
                        ), None);

                    next_level.push(node);
                };
            }
            
            nodes = next_level;
        }

        MerkleTree { root: nodes[0].clone() }
    }
}