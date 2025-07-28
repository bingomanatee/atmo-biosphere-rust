use dashmap::DashMap;
use std::hash::Hash;

/// Generic concurrent collection that stores data by key
#[derive(Debug)]
pub struct Collection<K, T>
where
    K: Hash + Eq + Clone + Send + Sync,
    T: Clone + Send + Sync,
{
    data: DashMap<K, T>,
}

impl<K, T> Collection<K, T>
where
    K: Hash + Eq + Clone + Send + Sync,
    T: Clone + Send + Sync,
{
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    /// Get item by key (concurrent read access)
    pub fn get(&self, key: &K) -> Option<dashmap::mapref::one::Ref<K, T>> {
        self.data.get(key)
    }

    /// Check if collection contains key
    pub fn contains_key(&self, key: &K) -> bool {
        self.data.contains_key(key)
    }

    /// Get number of items
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Insert item (concurrent write access)
    pub fn insert(&self, key: K, value: T) -> Option<T> {
        self.data.insert(key, value)
    }

    /// Remove item (concurrent write access)
    pub fn remove(&self, key: &K) -> Option<(K, T)> {
        self.data.remove(key)
    }

    /// Get mutable reference (concurrent write access)
    pub fn get_mut(&self, key: &K) -> Option<dashmap::mapref::one::RefMut<K, T>> {
        self.data.get_mut(key)
    }

    /// Iterate over all key-value pairs (concurrent read access)
    pub fn iter(&self) -> dashmap::iter::Iter<K, T> {
        self.data.iter()
    }

    /// Apply a function to a value if it exists (atomic read-modify-write)
    pub fn modify<F>(&self, key: &K, f: F) -> bool
    where
        F: FnOnce(&mut T),
    {
        if let Some(mut entry) = self.data.get_mut(key) {
            f(&mut *entry);
            true
        } else {
            false
        }
    }
}
