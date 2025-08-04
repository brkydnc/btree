use crate::{BTreeSet, Error, Result};
use std::collections::VecDeque;

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
            match node.find(key) {
                SearchResult::NotFound => return Err(Error::KeyNotFound),
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
        todo!()
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
    fn find(&self, key: &K) -> SearchResult<'_, K, B> {
        match self.keys.binary_search(key) {
            Ok(idx) => SearchResult::Key(&self.keys[idx]),
            Err(idx) => {
                if self.is_leaf {
                    SearchResult::NotFound
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

            if self.keys.len() > Self::MAX_KEYS {
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
}

impl<K: Ord, const B: usize> Node<K, B> {
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
}

// enum RemoveResult<K> {
//     NotFound,
//     Deficient(K),
//     Removed(K),
// }

enum SearchResult<'a, K, const B: usize> {
    NotFound,
    Key(&'a K),
    Child(&'a Node<K, B>),
}
enum InsertResult<K, const B: usize> {
    AlreadyExists,
    Inserted,
    Split(K, Node<K, B>),
}

// impl<K: Ord, const B: usize> IntermediateNode<K, B> {
//     fn insert(&mut self, key: K) -> InsertionResult<K, B> {
//         let Err(idx) = self.keys.binary_search(&key) else {
//             return InsertionResult::AlreadyExists;
//         };

//         let child = &mut self.children[idx];

//         match child.insert(key) {
//             InsertionResult::Split(hoist, sibling) => {
//                 self.keys.insert(idx, hoist);
//                 self.children.insert(idx + 1, sibling);

//                 if self.children.len() > 2 * B - 1 {
//                     let (hoist, sibling) = self.split();
//                     InsertionResult::Split(hoist, Node::Intermediate(sibling).linked())
//                 } else {
//                     InsertionResult::Inserted
//                 }
//             }
//             x => x,
//         }
//     }

//     fn remove(&mut self, key: &K) -> RemovalResult<K> {
//         match self.keys.binary_search(key) {
//             Ok(idx) => self.remove_at(idx),
//             Err(idx) => {
//                 let result = self.children[idx].remove(key);

//                 if let RemovalResult::Deficient(removed_key) = result {
//                     if idx == 0 {
//                         if self.children[1].has_more_than_minimum_keys() {
//                             let (stolen_key, stolen_child) = self.children[1].steal_front();
//                             let parent_key = std::mem::replace(&mut self.keys[0], stolen_key);
//                             self.children[0].receive_back(parent_key, stolen_child);
//                         } else {
//                             let parent_key = self.keys.pop_front().unwrap();
//                             let deficient_sibling = self.children.pop_front().unwrap();
//                             self.children[0].merge_with_left_sibling_and_parent_key(
//                                 deficient_sibling,
//                                 parent_key,
//                             );
//                         }
//                     } else if idx == self.keys.len() {
//                         if self.children[idx - 1].has_more_than_minimum_keys() {
//                             let (stolen_key, stolen_child) = self.children[idx - 1].steal_back();
//                             let parent_key = std::mem::replace(&mut self.keys[idx - 1], stolen_key);
//                             self.children[idx].receive_front(parent_key, stolen_child);
//                         } else {
//                             let parent_key = self.keys.pop_back().unwrap();
//                             let deficient_sibling = self.children.pop_back().unwrap();
//                             self.children[0].merge_with_right_sibling_and_parent_key(
//                                 deficient_sibling,
//                                 parent_key,
//                             );
//                         }
//                     } else {
//                         if self.children[idx - 1].has_more_than_minimum_keys() {
//                             let (stolen_key, stolen_child) = self.children[idx - 1].steal_back();
//                             let parent_key = std::mem::replace(&mut self.keys[idx], stolen_key);
//                             self.children[idx].receive_front(parent_key, stolen_child);
//                         } else if self.children[idx + 1].has_more_than_minimum_keys() {
//                             let (stolen_key, stolen_child) = self.children[idx + 1].steal_front();
//                             let parent_key = std::mem::replace(&mut self.keys[idx], stolen_key);
//                             self.children[idx].receive_back(parent_key, stolen_child);
//                         } else {
//                             let parent_key = self.keys.remove(idx).unwrap();
//                             let deficient_sibling = self.children.remove(idx).unwrap();
//                             self.children[idx].merge_with_right_sibling_and_parent_key(
//                                 deficient_sibling,
//                                 parent_key,
//                             );
//                         }
//                     }

//                     if self.keys.len() < B - 1 {
//                         RemovalResult::Deficient(removed_key)
//                     } else {
//                         RemovalResult::Removed(removed_key)
//                     }
//                 } else {
//                     result
//                 }
//             }
//         }
//     }

//     fn remove_at(&mut self, idx: usize) -> RemovalResult<K> {
//         let key = if self.children[idx].has_more_than_minimum_keys() {
//             let rotation = self.children[idx].remove_back();
//             std::mem::replace(&mut self.keys[idx], rotation)
//         } else if self.children[idx + 1].has_more_than_minimum_keys() {
//             let rotation = self.children[idx + 1].remove_front();
//             std::mem::replace(&mut self.keys[idx], rotation)
//         } else {
//             self.remove_and_merge_at(idx)
//         };

//         if self.keys.len() < B - 1 {
//             RemovalResult::Deficient(key)
//         } else {
//             RemovalResult::Removed(key)
//         }
//     }

//     fn remove_and_merge_at(&mut self, idx: usize) -> K {
//         let parent_key = self.keys.remove(idx).unwrap();
//         let right_sibling = self.children.remove(idx + 1).unwrap();

//         self.children[idx].merge_with_right_sibling_and_parent_key(right_sibling, parent_key);
//         self.children[idx].remove_at(B - 1)
//     }

//     fn split(&mut self) -> (K, IntermediateNode<K, B>) {
//         let keys = self.keys.split_off(B);
//         let children = self.children.split_off(B);
//         let hoist = self.keys.pop_back().unwrap();
//         let sibling = IntermediateNode { keys, children };

//         (hoist, sibling)
//     }
// }

// impl<K: Ord, const B: usize> LeafNode<K, B> {
//     fn insert(&mut self, key: K) -> InsertionResult<K, B> {
//         let Err(idx) = self.keys.binary_search(&key) else {
//             return InsertionResult::AlreadyExists;
//         };

//         self.keys.insert(idx, key);

//         if self.keys.len() > 2 * B - 1 {
//             let (hoist, sibling) = self.split();
//             let link = Node::Leaf(sibling).linked();
//             InsertionResult::Split(hoist, link)
//         } else {
//             InsertionResult::Inserted
//         }
//     }

//     fn remove(&mut self, key: &K) -> RemovalResult<K> {
//         let Ok(idx) = self.keys.binary_search(&key) else {
//             return RemovalResult::NotFound;
//         };

//         self.remove_at(idx)
//     }

//     fn remove_at(&mut self, idx: usize) -> RemovalResult<K> {
//         let val = self.keys.remove(idx).unwrap();

//         if self.keys.len() < B {
//             RemovalResult::Deficient(val)
//         } else {
//             RemovalResult::Removed(val)
//         }
//     }

//     fn split(&mut self) -> (K, LeafNode<K, B>) {
//         let keys = self.keys.split_off(B);
//         let hoist = self.keys.pop_back().unwrap();
//         let sibling = LeafNode { keys };

//         (hoist, sibling)
//     }

// impl<K: Ord, const B: usize> Node<K, B> {
//     fn merge_with_right_sibling_and_parent_key(
//         &mut self,
//         right_sibling: Link<K, B>,
//         parent_key: K,
//     ) {
//         match self {
//             Node::Intermediate(node) => {
//                 node.merge_with_right_sibling_and_parent_key(right_sibling, parent_key)
//             }
//             Node::Leaf(node) => {
//                 node.merge_with_right_sibling_and_parent_key(right_sibling, parent_key)
//             }
//         }
//     }

//     fn merge_with_left_sibling_and_parent_key(&mut self, left_sibling: Link<K, B>, parent_key: K) {
//         match self {
//             Node::Intermediate(node) => {
//                 node.merge_with_left_sibling_and_parent_key(left_sibling, parent_key)
//             }
//             Node::Leaf(node) => {
//                 node.merge_with_left_sibling_and_parent_key(left_sibling, parent_key)
//             }
//         }
//     }

//     fn receive_front(&mut self, key: K, child: Link<K, B>) {
//         match self {
//             Node::Intermediate(node) => node.receive_front(),
//             Node::Leaf(node) => node.receive_front(),
//         }
//     }

//     fn receive_back(&mut self, key: K, child: Link<K, B>) {
//         match self {
//             Node::Intermediate(node) => node.receive_back(),
//             Node::Leaf(node) => node.receive_back(),
//         }
//     }

//     fn steal_front(&mut self) -> (K, Link<K, B>) {
//         match self {
//             Node::Intermediate(node) => node.steal_front(),
//             Node::Leaf(node) => node.steal_front(),
//         }
//     }

//     fn steal_back(&mut self) -> (K, Link<K, B>) {
//         match self {
//             Node::Intermediate(node) => node.steal_back(),
//             Node::Leaf(node) => node.steal_back(),
//         }
//     }

//     fn has_more_than_minimum_keys(&self) -> bool {
//         match self {
//             Node::Intermediate(node) => node.keys.len() >= B,
//             Node::Leaf(node) => node.keys.len() >= B,
//         }
//     }

//     fn get(&self, idx: usize) -> &K {
//         match self {
//             Node::Intermediate(node) => &node.keys[idx],
//             Node::Leaf(node) => &node.keys[idx],
//         }
//     }

//     fn binary_search(&self, key: &K) -> StdResult<usize, usize> {
//         match self {
//             Node::Intermediate(node) => node.keys.binary_search(key),
//             Node::Leaf(node) => node.keys.binary_search(key),
//         }
//     }

//     fn insert(&mut self, key: K) -> InsertionResult<K, B> {
//         match self {
//             Node::Intermediate(node) => node.insert(key),
//             Node::Leaf(node) => node.insert(key),
//         }
//     }

//     fn remove_front(&mut self) -> K {
//         match self {
//             Node::Intermediate(node) => node.remove_front(),
//             Node::Leaf(node) => node.remove_front(),
//         }
//     }

//     fn remove_back(&mut self) -> K {
//         match self {
//             Node::Intermediate(node) => node.remove_back(),
//             Node::Leaf(node) => node.remove_back(),
//         }
//     }

//     fn remove_at(&mut self, idx: usize) -> K {
//         match self {
//             Node::Intermediate(node) => node.remove_at(key, idx),
//             Node::Leaf(node) => node.remove_at(key, idx),
//         }
//     }

//     fn remove(&mut self, key: &K) -> RemovalResult<K> {
//         match self {
//             Node::Intermediate(node) => node.remove(key),
//             Node::Leaf(node) => node.remove(key),
//         }
//     }
// }

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
