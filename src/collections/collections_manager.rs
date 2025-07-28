use std::collections::HashMap;
use std::any::Any;
use crate::collections::{Collection, CollectionsEnum};
use crate::collections::changes::CollectionChange;

/// Central manager for all collections with type-safe access
pub struct CollectionsManager {
    // Hidden internal storage - not directly accessible
    collections: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl CollectionsManager {
    /// Create new empty collections manager
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }

    /// Add a collection to the manager
    pub fn add_collection<K, T>(&mut self, name: &str, collection: Collection<K, T>)
    where
        K: 'static + Clone + std::hash::Hash + Eq + Send + Sync,
        T: 'static + Clone + Send + Sync,
    {
        self.collections.insert(name.to_string(), Box::new(collection));
    }

    /// Create and add an empty collection
    pub fn add_empty_collection<K, T>(&mut self, name: &str)
    where
        K: 'static + Clone + std::hash::Hash + Eq + Send + Sync,
        T: 'static + Clone + Send + Sync,
    {
        self.add_collection(name, Collection::<K, T>::new());
    }

    /// Type-safe access to collections by string name
    pub fn get<K, T>(&self, name: &str) -> Option<&Collection<K, T>>
    where
        K: 'static + Clone + std::hash::Hash + Eq + Send + Sync,
        T: 'static + Clone + Send + Sync,
    {
        self.collections
            .get(name)
            .and_then(|boxed| boxed.downcast_ref::<Collection<K, T>>())
    }

    /// Type-safe access to collections by enum (for convenience)
    pub fn get_by_enum<K, T>(&self, name: CollectionsEnum) -> Option<&Collection<K, T>>
    where
        K: 'static + Clone + std::hash::Hash + Eq + Send + Sync,
        T: 'static + Clone + Send + Sync,
    {
        let collection_name = match name {
            CollectionsEnum::FOO => "FOO",
            CollectionsEnum::BAR => "BAR",
        };
        self.get(collection_name)
    }

    /// Apply a batch of changes atomically
    pub fn apply_changes(&mut self, changes: Vec<Box<dyn CollectionChange>>) -> Result<(), String> {
        // Group changes by collection
        let mut grouped_changes: HashMap<String, Vec<Box<dyn CollectionChange>>> = HashMap::new();

        for change in changes {
            let collection_name = change.collection_name().to_string();
            grouped_changes.entry(collection_name).or_insert_with(Vec::new).push(change);
        }

        // Apply changes to each collection using the trait
        for (collection_name, collection_changes) in grouped_changes {
            let collection = self.collections
                .get_mut(&collection_name)
                .ok_or_else(|| format!("Collection '{}' not found", collection_name))?;

            for change in collection_changes {
                change.apply_to_collection(collection.as_mut())?;
            }
        }

        Ok(())
    }


}
