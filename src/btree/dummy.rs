use crate::{BTreeSet, Error, Result};
use std::collections::VecDeque;
use std::result::Result as StdResult;

/// A dummy BTreeSet implementation. Neither it concerns itself with an on-disk implementation, nor
/// with an efficient in-memory implementation. The only thing it cares about is the logic behind
/// the basic operations of a BTree data structure.
///
/// There are 2 problems with this implementation:
///
///     1 - The Node<K, B> enum requires everything every method of its variants to be reimplemented
///         on itself. We might get around this situtation by using a more clever (whatever that means)
///         interface, or by using transmute<T, U>(), and #[repr(C)] to actually hide leafs under
///         intermediate nodes. Or, we can just use the same "Node" representation with a boolean flag.
///
///     2 - The message passing implementation belabors the implementation, especially the "deficient"
///         propagations, what if we
///
pub struct DummyBTreeSet<K, const B: usize = 6> {
    root: Option<Root<K, B>>,
}

struct Root<K, const B: usize> {
    node: Node<K, B>,
}

struct IntermediateNode<K, const B: usize> {
    keys: VecDeque<K>,
    children: VecDeque<Link<K, B>>,
}

struct LeafNode<K, const B: usize> {
    keys: VecDeque<K>,
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
            x => x,
        }
    }

    fn remove(&mut self, key: &K) -> RemovalResult<K> {
        match self.keys.binary_search(key) {
            Ok(idx) => self.remove_at(idx),
            Err(idx) => {
                let result = self.children[idx].remove(key);

                if let RemovalResult::Deficient(removed_key) = result {
                    if idx == 0 {
                        if self.children[1].has_more_than_minimum_keys() {
                            let (stolen_key, stolen_child) = self.children[1].steal_front();
                            let parent_key = std::mem::replace(&mut self.keys[0], stolen_key);
                            self.children[0].receive_back(parent_key, stolen_child);
                        } else {
                            let parent_key = self.keys.pop_front().unwrap();
                            let deficient_sibling = self.children.pop_front().unwrap();
                            self.children[0].merge_with_left_sibling_and_parent_key(
                                deficient_sibling,
                                parent_key,
                            );
                        }
                    } else if idx == self.keys.len() {
                        if self.children[idx - 1].has_more_than_minimum_keys() {
                            let (stolen_key, stolen_child) = self.children[idx - 1].steal_back();
                            let parent_key = std::mem::replace(&mut self.keys[idx - 1], stolen_key);
                            self.children[idx].receive_front(parent_key, stolen_child);
                        } else {
                            let parent_key = self.keys.pop_back().unwrap();
                            let deficient_sibling = self.children.pop_back().unwrap();
                            self.children[0].merge_with_right_sibling_and_parent_key(
                                deficient_sibling,
                                parent_key,
                            );
                        }
                    } else {
                        if self.children[idx - 1].has_more_than_minimum_keys() {
                            let (stolen_key, stolen_child) = self.children[idx - 1].steal_back();
                            let parent_key = std::mem::replace(&mut self.keys[idx], stolen_key);
                            self.children[idx].receive_front(parent_key, stolen_child);
                        } else if self.children[idx + 1].has_more_than_minimum_keys() {
                            let (stolen_key, stolen_child) = self.children[idx + 1].steal_front();
                            let parent_key = std::mem::replace(&mut self.keys[idx], stolen_key);
                            self.children[idx].receive_back(parent_key, stolen_child);
                        } else {
                            let parent_key = self.keys.remove(idx).unwrap();
                            let deficient_sibling = self.children.remove(idx).unwrap();
                            self.children[idx].merge_with_right_sibling_and_parent_key(
                                deficient_sibling,
                                parent_key,
                            );
                        }
                    }

                    if self.keys.len() < B - 1 {
                        RemovalResult::Deficient(removed_key)
                    } else {
                        RemovalResult::Removed(removed_key)
                    }
                } else {
                    result
                }
            }
        }
    }

    fn remove_at(&mut self, idx: usize) -> RemovalResult<K> {
        let key = if self.children[idx].has_more_than_minimum_keys() {
            let rotation = self.children[idx].remove_back();
            std::mem::replace(&mut self.keys[idx], rotation)
        } else if self.children[idx + 1].has_more_than_minimum_keys() {
            let rotation = self.children[idx + 1].remove_front();
            std::mem::replace(&mut self.keys[idx], rotation)
        } else {
            self.remove_and_merge_at(idx)
        };

        if self.keys.len() < B - 1 {
            RemovalResult::Deficient(key)
        } else {
            RemovalResult::Removed(key)
        }
    }

    fn remove_and_merge_at(&mut self, idx: usize) -> K {
        let parent_key = self.keys.remove(idx).unwrap();
        let right_sibling = self.children.remove(idx + 1).unwrap();

        self.children[idx].merge_with_right_sibling_and_parent_key(right_sibling, parent_key);
        self.children[idx].remove_at(B - 1)
    }

    fn split(&mut self) -> (K, IntermediateNode<K, B>) {
        let keys = self.keys.split_off(B);
        let children = self.children.split_off(B);
        let hoist = self.keys.pop_back().unwrap();
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

    fn remove(&mut self, key: &K) -> RemovalResult<K> {
        let Ok(idx) = self.keys.binary_search(&key) else {
            return RemovalResult::NotFound;
        };

        self.remove_at(idx)
    }

    fn remove_at(&mut self, idx: usize) -> RemovalResult<K> {
        let val = self.keys.remove(idx).unwrap();

        if self.keys.len() < B {
            RemovalResult::Deficient(val)
        } else {
            RemovalResult::Removed(val)
        }
    }

    fn split(&mut self) -> (K, LeafNode<K, B>) {
        let keys = self.keys.split_off(B);
        let hoist = self.keys.pop_back().unwrap();
        let sibling = LeafNode { keys };

        (hoist, sibling)
    }
}

enum Node<K, const B: usize> {
    Intermediate(IntermediateNode<K, B>),
    Leaf(LeafNode<K, B>),
}

type Link<K, const B: usize> = Box<Node<K, B>>;

enum InsertionResult<K, const B: usize> {
    AlreadyExists,
    Inserted,
    Split(K, Link<K, B>),
}

enum RemovalResult<K> {
    NotFound,
    Deficient(K),
    Removed(K),
}

impl<K: Ord, const B: usize> Node<K, B> {
    fn new_intermediate(
        keys: impl IntoIterator<Item = K>,
        children: impl IntoIterator<Item = Link<K, B>>,
    ) -> Self {
        Node::Intermediate(IntermediateNode {
            keys: keys.into_iter().collect(),
            children: children.into_iter().collect(),
        })
    }

    fn new_leaf(keys: impl IntoIterator<Item = K>) -> Self {
        Node::Leaf(LeafNode {
            keys: keys.into_iter().collect(),
        })
    }

    fn linked(self) -> Link<K, B> {
        Box::new(self)
    }

    fn merge_with_right_sibling_and_parent_key(
        &mut self,
        right_sibling: Link<K, B>,
        parent_key: K,
    ) {
        match self {
            Node::Intermediate(node) => {
                node.merge_with_right_sibling_and_parent_key(right_sibling, parent_key)
            }
            Node::Leaf(node) => {
                node.merge_with_right_sibling_and_parent_key(right_sibling, parent_key)
            }
        }
    }

    fn merge_with_left_sibling_and_parent_key(&mut self, left_sibling: Link<K, B>, parent_key: K) {
        match self {
            Node::Intermediate(node) => {
                node.merge_with_left_sibling_and_parent_key(left_sibling, parent_key)
            }
            Node::Leaf(node) => {
                node.merge_with_left_sibling_and_parent_key(left_sibling, parent_key)
            }
        }
    }

    fn receive_front(&mut self, key: K, child: Link<K, B>) {
        match self {
            Node::Intermediate(node) => node.receive_front(),
            Node::Leaf(node) => node.receive_front(),
        }
    }

    fn receive_back(&mut self, key: K, child: Link<K, B>) {
        match self {
            Node::Intermediate(node) => node.receive_back(),
            Node::Leaf(node) => node.receive_back(),
        }
    }

    fn steal_front(&mut self) -> (K, Link<K, B>) {
        match self {
            Node::Intermediate(node) => node.steal_front(),
            Node::Leaf(node) => node.steal_front(),
        }
    }

    fn steal_back(&mut self) -> (K, Link<K, B>) {
        match self {
            Node::Intermediate(node) => node.steal_back(),
            Node::Leaf(node) => node.steal_back(),
        }
    }

    fn has_more_than_minimum_keys(&self) -> bool {
        match self {
            Node::Intermediate(node) => node.keys.len() >= B,
            Node::Leaf(node) => node.keys.len() >= B,
        }
    }

    fn get(&self, idx: usize) -> &K {
        match self {
            Node::Intermediate(node) => &node.keys[idx],
            Node::Leaf(node) => &node.keys[idx],
        }
    }

    fn binary_search(&self, key: &K) -> StdResult<usize, usize> {
        match self {
            Node::Intermediate(node) => node.keys.binary_search(key),
            Node::Leaf(node) => node.keys.binary_search(key),
        }
    }

    fn insert(&mut self, key: K) -> InsertionResult<K, B> {
        match self {
            Node::Intermediate(node) => node.insert(key),
            Node::Leaf(node) => node.insert(key),
        }
    }

    fn remove_front(&mut self) -> K {
        match self {
            Node::Intermediate(node) => node.remove_front(),
            Node::Leaf(node) => node.remove_front(),
        }
    }

    fn remove_back(&mut self) -> K {
        match self {
            Node::Intermediate(node) => node.remove_back(),
            Node::Leaf(node) => node.remove_back(),
        }
    }

    fn remove_at(&mut self, idx: usize) -> K {
        match self {
            Node::Intermediate(node) => node.remove_at(key, idx),
            Node::Leaf(node) => node.remove_at(key, idx),
        }
    }

    fn remove(&mut self, key: &K) -> RemovalResult<K> {
        match self {
            Node::Intermediate(node) => node.remove(key),
            Node::Leaf(node) => node.remove(key),
        }
    }
}

impl<K: Ord, const B: usize> BTreeSet for DummyBTreeSet<K, B> {
    type Key = K;

    fn new() -> Self {
        DummyBTreeSet { root: None }
    }

    fn min_degree(&self) -> usize {
        B
    }

    fn search(&self, key: &Self::Key) -> Result<&Self::Key> {
        let root = self.root.as_ref().ok_or(Error::KeyNotFound)?;
        let mut node = &root.node;

        loop {
            match node.binary_search(key) {
                Ok(idx) => return Ok(&node.get(idx)),
                Err(idx) => match node {
                    Node::Intermediate(intermediate) => node = &intermediate.children[idx],
                    Node::Leaf(_) => return Err(Error::KeyNotFound),
                },
            }
        }
    }

    fn insert(&mut self, key: Self::Key) -> Result<()> {
        match self.root.take() {
            Some(mut root) => match root.node.insert(key) {
                InsertionResult::Split(hoist, sibling) => {
                    let node = Node::new_intermediate(
                        vec![hoist],
                        vec![root.node.linked(), sibling.linked()],
                    );

                    self.root = Some(Root { node });
                    Ok(())
                }
                InsertionResult::AlreadyExists => {
                    self.root = Some(root);
                    Err(Error::KeyAlreadyExists)
                }
                InsertionResult::Inserted => {
                    self.root = Some(root);
                    Ok(())
                }
            },
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
