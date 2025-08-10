use thiserror::Error;

pub mod btree;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("key not found")]
    KeyNotFound,

    #[error("key already exists")]
    KeyAlreadyExists,
}

pub trait BTreeSet {
    type Key: Ord;
    const B: usize;

    fn search(&self, key: &Self::Key) -> Result<&Self::Key>;
    fn insert(&mut self, key: Self::Key) -> Result<()>;
    fn remove(&mut self, key: &Self::Key) -> Result<Self::Key>;

    fn contains(&self, key: &Self::Key) -> bool {
        self.search(key).is_ok()
    }

    fn max_keys(&self) -> usize {
        2 * Self::B - 1
    }
}

macro_rules! test_btree_impl (
    ($impl:ident) => {
        #[test]
        fn test_new_returns_instance() {
            let _tree = $impl::<i32>::new();
        }

        #[test]
        fn test_empty_tree_does_not_contain_keys() {
            let tree = $impl::<i32>::new();
            let items = vec![0, 420, i32::MAX, i32::MIN];

            for i in items {
                assert!(!tree.contains(&i));
            }
        }

        #[test]
        fn test_contains_returns_true_after_insertion_without_splits() {
            let mut tree = $impl::<usize>::new();
            let items = (0..tree.max_keys());

            for i in items {
                assert!(!tree.contains(&i));
                assert_eq!(tree.insert(i).unwrap(), ());
                assert!(tree.contains(&i));
            }
        }

        #[test]
        fn test_contains_returns_true_after_insertion_with_splits() {
            let mut tree = $impl::<usize>::new();
            let items = (0..tree.max_keys() + 1);

            for i in items {
                assert!(!tree.contains(&i));
                assert_eq!(tree.insert(i).unwrap(), ());
                assert!(tree.contains(&i));
            }
        }

        #[test]
        fn test_contains_returns_true_after_insertion_with_many_splits() {
            let mut tree = $impl::<usize>::new();
            let items = (0..tree.max_keys().pow(4));

            for i in items {
                assert!(!tree.contains(&i));
                assert_eq!(tree.insert(i).unwrap(), ());
                assert!(tree.contains(&i));
            }
        }

        #[test]
        fn test_duplicate_key_returns_error_without_splits() {
            let mut tree = $impl::<usize>::new();
            let items = (0..tree.max_keys());

            for i in items {
                assert_eq!(tree.insert(i).unwrap(), ());
                assert!(tree.contains(&i));
                let result = tree.insert(i);
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), Error::KeyAlreadyExists));
            }
        }

        #[test]
        fn test_duplicate_key_returns_error_with_splits() {
            let mut tree = $impl::<usize>::new();
            let items = (0..tree.max_keys() + 1);

            for i in items {
                assert_eq!(tree.insert(i).unwrap(), ());
                assert!(tree.contains(&i));
                let result = tree.insert(i);
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), Error::KeyAlreadyExists));
            }
        }

        #[test]
        fn test_duplicate_key_returns_error_with_many_splits() {
            let mut tree = $impl::<usize>::new();
            let items = (0..tree.max_keys().pow(4));

            for i in items {
                assert_eq!(tree.insert(i).unwrap(), ());
                assert!(tree.contains(&i));
                let result = tree.insert(i);
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), Error::KeyAlreadyExists));
            }
        }

        #[test]
        fn test_search_existing_key_returns_ok() {
            let mut tree = $impl::<i32>::new();
            let key = 50;
            assert_eq!(tree.insert(key).unwrap(), ());
            assert_eq!(tree.search(&key).unwrap(), &key);
        }

        #[test]
        fn test_search_non_existing_key_returns_error() {
            let tree = $impl::<i32>::new();
            let key = 75;
            let result = tree.search(&key);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::KeyNotFound));
        }

        #[test]
        fn test_remove_existing_key_returns_ok_and_removes() {
            let mut tree = $impl::<i32>::new();
            let key = 20;
            assert_eq!(tree.insert(key).unwrap(), ());
            assert!(tree.contains(&key));
            assert_eq!(tree.remove(&key).unwrap(), key);
            assert!(!tree.contains(&key));
        }

        #[test]
        fn test_remove_non_existing_key_returns_error() {
            let mut tree = $impl::<i32>::new();
            let key = 99;
            let result = tree.remove(&key);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), Error::KeyNotFound));
        }

        #[test]
        fn test_multiple_insertions_and_deletions() {
            let mut tree = $impl::<i32>::new();
            let items = vec![10, 5, 15, 2, 7, 12, 18];

            for &item in &items {
                tree.insert(item).unwrap();
            }

            for &item in &items {
                assert!(tree.contains(&item));
            }

            assert_eq!(tree.remove(&7).unwrap(), 7);
            assert!(!tree.contains(&7));
            assert_eq!(tree.remove(&18).unwrap(), 18);
            assert!(!tree.contains(&18));

            assert!(tree.contains(&10));
            assert!(tree.contains(&5));
            assert!(tree.contains(&15));
            assert!(tree.contains(&2));
            assert!(tree.contains(&12));

            let result = tree.remove(&7);
            assert!(result.is_err());
        }

        #[test]
        fn test_tree_stability_after_many_operations() {
            let mut tree = $impl::<i32>::new();
            let mut inserted_keys = Vec::new();

            // Insert many elements
            for i in 0..1000 {
                tree.insert(i).unwrap();
                inserted_keys.push(i);
            }

            // Verify all inserted elements are present
            for &key in &inserted_keys {
                assert!(tree.contains(&key));
            }

            // Delete some elements
            for i in (0..1000).step_by(2) {
                tree.remove(&i).unwrap();
            }

            // Verify remaining elements
            for i in 0..1000 {
                if i % 2 == 0 {
                    assert!(!tree.contains(&i));
                } else {
                    assert!(tree.contains(&i));
                }
            }
        }

    }
);

pub(crate) use test_btree_impl;
