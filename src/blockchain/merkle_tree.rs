use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct MerkleNode<'a> {
    pub left: Option<&'a MerkleNode<'a>>,
    pub right: Option<&'a MerkleNode<'a>>,
    pub data: Vec<u8>,
}

pub struct MerkleTree<'a> {
    root: MerkleNode<'a>
}

impl<'a> MerkleNode<'a> {
    fn new(left: Option<&'a Self>, right: Option<&'a Self>, data: Option<&[u8]>) -> Self {
        let hash;
        
        if (left.is_none() && right.is_none()) {
            let data = data.unwrap();
            let mut hasher = Sha256::new();
            hasher.update(data);
            hash = hasher.finalize();
        } else {
            let mut data = left.unwrap().data.clone();
            data.append(&mut right.unwrap().data.clone());

            let mut hasher = Sha256::new();
            hasher.update(data);
            hash = hasher.finalize();
        }

        Self {
            data: hash.to_vec(),
            left,
            right,
        }
    }
}


impl<'a: 'b, 'b> MerkleTree<'a> {
    pub fn new(mut data: Vec<&[u8]>) -> Self {
        let data_size = data.len();
        
        if data_size % 2 != 0 {
            data.push(data.last().unwrap());
        }

        let mut nodes = Vec::<&'a MerkleNode<'_>>::new();

        for bottom_node in data.into_iter() {
            let node = MerkleNode::<'a>::new(None, None, Some(bottom_node));
            nodes.push(&node);
        }

        for _i in 0..(data_size/2) {
            let mut nextLevel = Vec::<&'a MerkleNode<'_>>::new();

            for w in nodes.clone().windows(2) {
                if let [left, right] = w {
                    let node = MerkleNode::<'a>::new(Some(left), Some(right), None);

                    nextLevel.push(&node);
                };
            }
            
            nodes = nextLevel;
        }

        MerkleTree { root: nodes[0].clone() }
    }
}