use crate::{BTreeSet, Error, Result};
use std::{collections::VecDeque, f32::MIN};

pub struct DummyBTreeSet<K, const B: usize = 6> {
    root: Option<Root<K, B>>,
}

struct Root<K, const B: usize> {
    node: Node<K, B>,
}

impl<K: Ord, const B: usize> BTreeSet for Root<K, B> {
    type Key = K;
    const B: usize = B;

    fn search(&self, key: &Self::Key) -> Result<&Self::Key> {
        let mut node = &self.node;
        loop {
            match node.search(key) {
                SearchResult::None => return Err(Error::KeyNotFound),
                SearchResult::Key(key) => return Ok(key),
                SearchResult::Child(child) => {
                    node = child;
                }
            }
        }
    }

    fn insert(&mut self, key: Self::Key) -> Result<()> {
        match self.node.insert(key) {
            InsertResult::AlreadyExists => Err(Error::KeyAlreadyExists),
            InsertResult::Inserted => Ok(()),
            InsertResult::Split(hoist, sibling) => {
                let old_node = std::mem::take(&mut self.node);
                self.node = Node::intermediate([hoist], [old_node.link(), sibling.link()]);
                Ok(())
            }
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Result<Self::Key> {
        match self.node.remove(key) {
            RemoveResult::None => return Err(Error::KeyNotFound),
            RemoveResult::Key(key) => return Ok(key),
            RemoveResult::Deficiency(key) => {
                if self.node.has_no_remaining_keys() && !self.node.is_leaf {
                    self.node = *self.node.children.pop_front().unwrap();
                }

                Ok(key)
            }
        }
    }
}

type Link<K, const B: usize> = Box<Node<K, B>>;

struct Node<K, const B: usize> {
    is_leaf: bool,
    keys: VecDeque<K>,
    children: VecDeque<Link<K, B>>,
}

impl<K, const B: usize> Default for Node<K, B> {
    fn default() -> Self {
        Node {
            is_leaf: false,
            keys: VecDeque::new(),
            children: VecDeque::new(),
        }
    }
}

impl<K: Ord, const B: usize> Node<K, B> {
    const MIN_KEYS: usize = B - 1;
    const MAX_KEYS: usize = 2 * B - 1;
    const MIN_CHILDREN: usize = 2 * B;
    const MAX_CHILDREN: usize = B;

    fn has_no_remaining_keys(&self) -> bool {
        self.keys.is_empty()
    }

    fn is_deficient(&self) -> bool {
        self.keys.len() < Self::MIN_KEYS
    }

    fn is_overflowed(&self) -> bool {
        self.keys.len() > Self::MAX_KEYS
    }

    fn can_spare_key(&self) -> bool {
        self.keys.len() >= Self::MIN_KEYS
    }
}

impl<K: Ord, const B: usize> Node<K, B> {
    fn intermediate(
        keys_iter: impl IntoIterator<Item = K>,
        children_iter: impl IntoIterator<Item = Link<K, B>>,
    ) -> Node<K, B> {
        let mut keys = VecDeque::with_capacity(Self::MAX_KEYS + 1);
        let limited_keys = keys_iter.into_iter().take(Self::MAX_KEYS);

        keys.extend(limited_keys);

        let mut children = VecDeque::with_capacity(Self::MAX_CHILDREN + 1);
        let limited_children = children_iter.into_iter().take(Self::MAX_CHILDREN);

        children.extend(limited_children);

        Self {
            keys,
            children,
            is_leaf: false,
        }
    }

    fn leaf(keys_iter: impl IntoIterator<Item = K>) -> Node<K, B> {
        let mut keys = VecDeque::with_capacity(Self::MAX_KEYS + 1);
        let limited_keys = keys_iter.into_iter().take(Self::MAX_KEYS);

        keys.extend(limited_keys);

        Self {
            keys,
            children: VecDeque::new(),
            is_leaf: true,
        }
    }

    fn link(self) -> Link<K, B> {
        Box::new(self)
    }
}

impl<K: Ord, const B: usize> Node<K, B> {
    fn search(&self, key: &K) -> SearchResult<'_, K, B> {
        match self.keys.binary_search(key) {
            Ok(idx) => SearchResult::Key(&self.keys[idx]),
            Err(idx) => {
                if self.is_leaf {
                    SearchResult::None
                } else {
                    SearchResult::Child(&self.children[idx])
                }
            }
        }
    }

    fn insert(&mut self, key: K) -> InsertResult<K, B> {
        let Err(idx) = self.keys.binary_search(&key) else {
            return InsertResult::AlreadyExists;
        };

        if self.is_leaf {
            self.keys.insert(idx, key);

            if self.is_overflowed() {
                let (hoist, sibling) = self.split();
                InsertResult::Split(hoist, sibling)
            } else {
                InsertResult::Inserted
            }
        } else {
            let child = &mut self.children[idx];

            match child.insert(key) {
                InsertResult::Split(hoist, sibling) => {
                    self.keys.insert(idx, hoist);
                    self.children.insert(idx + 1, sibling.link());

                    if self.children.len() > 2 * B - 1 {
                        let (hoist, sibling) = self.split();
                        InsertResult::Split(hoist, sibling)
                    } else {
                        InsertResult::Inserted
                    }
                }
                x => x,
            }
        }
    }

    fn remove(&mut self, key: &K) -> RemoveResult<K> {
        let result = self.keys.binary_search(key);

        let key = if self.is_leaf {
            match result {
                Ok(idx) => self.remove_from_leaf_at(idx),
                Err(_) => return RemoveResult::None,
            }
        } else {
            match result {
                Ok(idx) => self.remove_from_intermediate_at(idx),
                Err(idx) => return self.remove_key_from_intermediate_child_at(key, idx),
            }
        };

        if self.is_deficient() {
            RemoveResult::Deficiency(key)
        } else {
            RemoveResult::Key(key)
        }
    }
}

impl<K: Ord, const B: usize> Node<K, B> {
    /// Splits the node into two nodes, returning the hoisted key and the new sibling node.
    fn split(&mut self) -> (K, Node<K, B>) {
        if self.is_leaf {
            let keys = self.keys.split_off(B);
            let hoist = self.keys.pop_back().unwrap();
            let sibling = Node::leaf(keys);
            (hoist, sibling)
        } else {
            let keys = self.keys.split_off(B);
            let hoist = self.keys.pop_back().unwrap();
            let children = self.children.split_off(B);
            let sibling = Node::intermediate(keys, children);
            (hoist, sibling)
        }
    }

    /// Merges the right child into the left child and lowers the parent key.
    ///
    /// This method assumes that:
    ///    1. The given index points to a valid key.
    ///    2. The left and right children contains at most `2B - 2` keys in total.
    fn merge_and_lower_intermediate_parent_key(&mut self, idx: usize) {
        let right_child = self.children.remove(idx + 1).unwrap();
        let parent_key = self.keys.remove(idx).unwrap();
        let left = &mut self.children[idx];
        left.keys.push_back(parent_key);
        left.keys.extend(right_child.keys);
        left.children.extend(right_child.children);
    }

    /// Performs a left rotation on the key at the given index.
    ///
    /// This method assumes that:
    ///     1. The index points to a valid key.
    ///     2. The right child can spare a key.
    ///     3. The left child contains less keys than the maximum number allowed.
    fn rotate_left(&mut self, idx: usize) {
        if self.children[idx].is_leaf {
            let right = &mut self.children[idx + 1];
            let right_key = right.keys.pop_front().unwrap();
            let parent_key = std::mem::replace(&mut self.keys[idx], right_key);
            let left = &mut self.children[idx];
            left.keys.push_back(parent_key);
        } else {
            let right = &mut self.children[idx + 1];
            let right_key = right.keys.pop_front().unwrap();
            let right_child = right.children.pop_front().unwrap();
            let parent_key = std::mem::replace(&mut self.keys[idx], right_key);
            let left = &mut self.children[idx];
            left.keys.push_back(parent_key);
            left.children.push_back(right_child);
        }
    }

    /// Performs a right rotation on the key at the given index.
    ///
    /// This method assumes that:
    ///     1. The index points to a valid key.
    ///     2. The left child can spare a key.
    ///     3. The right child contains less keys than the maximum number allowed.
    fn rotate_right(&mut self, idx: usize) {
        if self.children[idx + 1].is_leaf {
            let left = &mut self.children[idx];
            let left_key = left.keys.pop_back().unwrap();
            let parent_key = std::mem::replace(&mut self.keys[idx], left_key);
            let right = &mut self.children[idx + 1];
            right.keys.push_front(parent_key);
        } else {
            let left = &mut self.children[idx];
            let left_key = left.keys.pop_back().unwrap();
            let left_child = left.children.pop_back().unwrap();
            let parent_key = std::mem::replace(&mut self.keys[idx], left_key);
            let right = &mut self.children[idx + 1];
            right.keys.push_front(parent_key);
            right.children.push_front(left_child);
        }
    }

    /// Removes the last key from the node.
    ///
    /// This method assumes that the node `.can_spare_key()`.
    fn force_remove_last_key(&mut self) -> K {
        if self.is_leaf {
            self.keys.pop_back().unwrap()
        } else {
            self.remove_from_intermediate_at(self.keys.len() - 1)
        }
    }

    /// Removes the first key from the node.
    ///
    /// This method assumes that the node `.can_spare_key()`.
    fn force_remove_first_key(&mut self) -> K {
        if self.is_leaf {
            self.keys.pop_front().unwrap()
        } else {
            self.remove_from_intermediate_at(0)
        }
    }

    /// Removes a key from a leaf node at the given index.
    ///
    /// This method assumes that:
    ///      1 - The current node is a leaf node.
    ///      2 - The given index points to an existing key.
    fn remove_from_leaf_at(&mut self, idx: usize) -> K {
        self.keys.remove(idx).unwrap()
    }

    /// Removes a key from an intermediate node at the given index.
    ///
    /// This method assumes that:
    ///      1 - The current node is an intermediate node.
    ///      2 - The current node is not deficient before the removal.
    ///      3 - The given index points to an existing key.
    fn remove_from_intermediate_at(&mut self, idx: usize) -> K {
        if self.children[idx].can_spare_key() {
            // Case 1: If the left child can spare a key, we take it.
            let key_from_children = self.children[idx].force_remove_last_key();
            std::mem::replace(&mut self.keys[idx], key_from_children)
        } else if self.children[idx + 1].can_spare_key() {
            // Case 2: If the right child can spare a key, we take it.
            let key_from_children = self.children[idx].force_remove_first_key();
            std::mem::replace(&mut self.keys[idx], key_from_children)
        } else {
            // Case 3: If neither child can spare a key, we merge with the right sibling.
            let right = self.children.remove(idx + 1).unwrap();
            let left = &mut self.children[idx];
            left.keys.extend(right.keys);
            left.children.extend(right.children);
            self.keys.remove(idx).unwrap()
        }
    }

    /// Removes a key from an intermediate child at the given index. Be aware
    /// that this method might remove the key from the parent node as well, if a
    /// merge happens.
    ///
    /// This method assumes that:
    ///      1 - The current node is an intermediate node.
    ///      2 - The given index points to an existing child.
    fn remove_key_from_intermediate_child_at(&mut self, key: &K, idx: usize) -> RemoveResult<K> {
        let key = match self.children[idx].remove(key) {
            RemoveResult::Deficiency(key) => key,
            result => return result,
        };

        if idx == self.keys.len() {
            if self.children[idx].can_spare_key() {
                self.rotate_right(idx - 1);
            } else {
                self.merge_and_lower_intermediate_parent_key(idx - 1)
            }
        } else {
            if self.children[idx + 1].can_spare_key() {
                self.rotate_left(idx);
            } else {
                self.merge_and_lower_intermediate_parent_key(idx)
            }
        }

        if self.is_deficient() {
            RemoveResult::Deficiency(key)
        } else {
            RemoveResult::Key(key)
        }
    }
}

enum RemoveResult<K> {
    None,
    Key(K),
    Deficiency(K),
}

enum SearchResult<'a, K, const B: usize> {
    None,
    Key(&'a K),
    Child(&'a Node<K, B>),
}
enum InsertResult<K, const B: usize> {
    AlreadyExists,
    Inserted,
    Split(K, Node<K, B>),
}

impl<K: Ord, const B: usize> DummyBTreeSet<K, B> {
    fn new() -> Self {
        DummyBTreeSet { root: None }
    }
}

impl<K: Ord, const B: usize> BTreeSet for DummyBTreeSet<K, B> {
    type Key = K;
    const B: usize = B;

    fn search(&self, key: &Self::Key) -> Result<&Self::Key> {
        let root = self.root.as_ref().ok_or(Error::KeyNotFound)?;
        root.search(key)
    }

    fn insert(&mut self, key: Self::Key) -> Result<()> {
        if let Some(root) = self.root.as_mut() {
            root.insert(key)
        } else {
            let node = Node::leaf([key]);
            self.root = Some(Root { node });
            Ok(())
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Result<Self::Key> {
        if let Some(root) = self.root.as_mut() {
            root.remove(key)
        } else {
            Err(Error::KeyNotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_btree_impl;

    test_btree_impl!(DummyBTreeSet);
}
