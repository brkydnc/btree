use crate::{BTreeSet, Error, Result};

/// A dummy BTreeSet implementation. Neither it concerns itself with an on-disk implementation, nor
/// with an efficient in-memory implementation. The only thing it cares about is the logic behind
/// the basic operations of a BTree data structure.
pub struct DummyBTreeSet<K, const B: usize = 6> {
    root: Option<Root<K, B>>,
}

struct Root<K, const B: usize> {
    node: Node<K, B>,
}

struct IntermediateNode<K, const B: usize> {
    keys: Vec<K>,
    children: Vec<Link<K, B>>,
}

struct LeafNode<K, const B: usize> {
    keys: Vec<K>,
}

impl<K: Ord, const B: usize> IntermediateNode<K, B> {
    fn insert(&mut self, key: K) -> InsertionResult<K, B> {
        let Err(idx) = self.keys.binary_search(&key) else {
            return InsertionResult::AlreadyExists;
        };

        let child = &mut self.children[idx];

        match child.insert(key) {
            InsertionResult::Split(hoist, sibling) => {
                self.keys.insert(idx, hoist);
                self.children.insert(idx + 1, sibling);

                if self.children.len() > 2 * B - 1 {
                    let (hoist, sibling) = self.split();
                    InsertionResult::Split(hoist, Node::Intermediate(sibling).linked())
                } else {
                    InsertionResult::Inserted
                }
            }
            x => x
        }
    }

    fn split(&mut self) -> (K, IntermediateNode<K, B>) {
        let keys = self.keys.split_off(B);
        let children = self.children.split_off(B);
        let hoist = self.keys.pop().unwrap();
        let sibling = IntermediateNode { keys, children };

        (hoist, sibling)
    }
}

impl<K: Ord, const B: usize> LeafNode<K, B> {
    fn insert(&mut self, key: K) -> InsertionResult<K, B> {
        let Err(idx) = self.keys.binary_search(&key) else {
            return InsertionResult::AlreadyExists;
        };

        self.keys.insert(idx, key);

        if self.keys.len() > 2 * B - 1 {
            let (hoist, sibling) = self.split();
            let link = Node::Leaf(sibling).linked();
            InsertionResult::Split(hoist, link) 
        } else {
            InsertionResult::Inserted
        }
    }

    fn split(&mut self) -> (K, LeafNode<K, B>) {
        let keys = self.keys.split_off(B);
        let hoist = self.keys.pop().unwrap();
        let sibling = LeafNode { keys };

        (hoist, sibling)
    }
}

enum Node<K, const B: usize> {
    Intermediate(IntermediateNode<K, B>),
    Leaf(LeafNode<K, B>),
}

/// A conventional type that represents a pointer to a node.
type Link<K, const B: usize> = Box<Node<K, B>>;

enum InsertionResult<K, const B: usize> {
    AlreadyExists,
    Inserted,
    Split(K, Link<K, B>),
}

impl<K: Ord, const B: usize> Node<K, B> {
    fn new_intermediate(keys: Vec<K>, children: Vec<Link<K, B>>) -> Self {
        Node::Intermediate(IntermediateNode { keys, children })
    }

    fn new_leaf(keys: Vec<K>) -> Self {
        Node::Leaf(LeafNode { keys })
    }

    fn linked(self) -> Link<K, B> {
        Box::new(self)
    }

    fn keys(&self) -> &[K] {
        match self {
            Node::Intermediate(node) => &node.keys,
            Node::Leaf(node) => &node.keys,
        }
    }

    fn insert(&mut self, key: K) -> InsertionResult<K, B> {
        match self {
            Node::Intermediate(node) => node.insert(key),
            Node::Leaf(node) => node.insert(key),
        }
    }
}

impl<K: Ord, const B: usize> BTreeSet for DummyBTreeSet<K, B> {
    type Key = K;

    fn new() -> Self {
        DummyBTreeSet { root: None, }
    }

    fn min_degree(&self) -> usize {
        B
    }

    fn search(&self, key: &Self::Key) -> Result<&Self::Key> {
        let root = self.root.as_ref().ok_or(Error::KeyNotFound)?;
        let mut node = &root.node;

        loop {
            match node.keys().binary_search(key) {
                Ok(idx) => { return Ok(&node.keys()[idx]) },
                Err(idx) => {
                    match node {
                        Node::Intermediate(intermediate) => {
                            node = &intermediate.children[idx]
                        },
                        Node::Leaf(_) => {
                            return Err(Error::KeyNotFound)
                        }
                    }
                }
            }
        }
    }

    fn insert(&mut self, key: Self::Key) -> Result<()> {
        match self.root.take() {
            Some(mut root) => {
                match root.node.insert(key) {
                    InsertionResult::Split(hoist, sibling) => {
                        let node = Node::new_intermediate(
                            vec![hoist],
                            vec![root.node.linked(), sibling.linked()]
                        );

                        self.root = Some(Root { node });
                        Ok(())
                    }
                    InsertionResult::AlreadyExists => {
                        self.root = Some(root);
                        Err(Error::KeyAlreadyExists)
                    },
                    InsertionResult::Inserted => {
                        self.root = Some(root);
                        Ok(())
                    },
                }
            }
            None => {
                let node = Node::new_leaf(vec![key]);
                self.root = Some(Root { node });
                Ok(())
            }
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Result<Self::Key> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_btree_impl;

    test_btree_impl!(DummyBTreeSet);
}
