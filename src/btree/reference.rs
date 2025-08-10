use std::collections::BTreeSet as StdBTreeSet;

use crate::{BTreeSet, Error, Result};

/// A BTreeSet test oracle.
pub struct ReferenceBTreeSet<K>(StdBTreeSet<K>);

impl<K> ReferenceBTreeSet<K> {
    pub fn new() -> Self {
        Self(StdBTreeSet::new())
    }
}

impl<K: Ord> BTreeSet for ReferenceBTreeSet<K> {
    type Key = K;
    const B: usize = 6;

    fn search(&self, key: &Self::Key) -> Result<&Self::Key> {
        self.0.get(key).ok_or(Error::KeyNotFound)
    }

    fn insert(&mut self, key: Self::Key) -> Result<()> {
        if self.0.insert(key) {
            Ok(())
        } else {
            Err(Error::KeyAlreadyExists)
        }
    }

    fn remove(&mut self, key: &Self::Key) -> Result<Self::Key> {
        self.0.take(key).ok_or(Error::KeyNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_btree_impl;

    test_btree_impl!(ReferenceBTreeSet);
}