use std::collections::HashMap;
use std::any::Any;


use crossbeam::channel::{unbounded, Sender, Receiver};
use crate::collections::{Collection, CollectionsEnum};
use crate::collections::changes::CollectionChange;
use crate::cell_location::CellLocation;

/// Generic collection event for thread-safe event-driven updates
#[derive(Debug)]
pub enum CollectionEvent {
    /// Add delta to a field (compressible)
    AddToField {
        collection: String,
        key: CellLocation,
        field: String,
        delta: f64
    },
    /// Set field to absolute value
    SetField {
        collection: String,
        key: CellLocation,
        field: String,
        value: f64
    },
    /// Create new cell data
    CreateCell {
        collection: String,
        key: CellLocation,
        data: Box<dyn Any + Send + Sync>
    },
    /// Delete cell
    DeleteCell {
        collection: String,
        key: CellLocation
    },
}

impl Clone for CollectionEvent {
    fn clone(&self) -> Self {
        match self {
            CollectionEvent::AddToField { collection, key, field, delta } => {
                CollectionEvent::AddToField {
                    collection: collection.clone(),
                    key: *key,
                    field: field.clone(),
                    delta: *delta,
                }
            },
            CollectionEvent::SetField { collection, key, field, value } => {
                CollectionEvent::SetField {
                    collection: collection.clone(),
                    key: *key,
                    field: field.clone(),
                    value: *value,
                }
            },
            CollectionEvent::CreateCell { collection, key, data: _ } => {
                // Can't clone Box<dyn Any>, so create a placeholder
                CollectionEvent::DeleteCell {
                    collection: collection.clone(),
                    key: *key,
                }
            },
            CollectionEvent::DeleteCell { collection, key } => {
                CollectionEvent::DeleteCell {
                    collection: collection.clone(),
                    key: *key,
                }
            },
        }
    }
}

/// Central manager for all collections with thread-safe event-driven access
#[derive(Debug)]
pub struct CollectionsManager {
    // Hidden internal storage - not directly accessible
    collections: HashMap<String, Box<dyn Any + Send + Sync>>,
    // Thread-safe event channel
    event_sender: Sender<CollectionEvent>,
    event_receiver: Receiver<CollectionEvent>,
    /// Current simulation step
    pub current_step: u32,
}

impl CollectionsManager {
    /// Create new empty collections manager with event channel
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            collections: HashMap::new(),
            event_sender: sender,
            event_receiver: receiver,
            current_step: 0,
        }
    }

    /// Update the current simulation step
    pub fn set_current_step(&mut self, step: u32) {
        self.current_step = step;
    }

    /// Get the current simulation year
    pub fn current_year(&self, years_per_step: u32) -> f64 {
        self.current_step as f64 * years_per_step as f64
    }

    /// Get a thread-safe event emitter for components
    pub fn get_event_emitter(&self) -> EventEmitter {
        EventEmitter {
            sender: self.event_sender.clone(),
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

    /// Type-safe mutable access to collections by string name
    pub fn get_mut<K, T>(&mut self, name: &str) -> Option<&mut Collection<K, T>>
    where
        K: 'static + Clone + std::hash::Hash + Eq + Send + Sync,
        T: 'static + Clone + Send + Sync,
    {
        self.collections
            .get_mut(name)
            .and_then(|boxed| boxed.downcast_mut::<Collection<K, T>>())
    }

    /// List all collection names for debugging
    pub fn list_collections(&self) -> Vec<String> {
        self.collections.keys().cloned().collect()
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

    /// Process all pending events atomically
    pub fn apply_pending_events(&mut self) -> Result<(), String> {
        // Collect all pending events
        let mut events = Vec::new();
        while let Ok(event) = self.event_receiver.try_recv() {
            events.push(event);
        }

        if events.is_empty() {
            return Ok(());
        }

        // Compress events (multiple AddToField for same key+field)
        let compressed_events = self.compress_events(&events);

        // Apply compressed events
        for event in compressed_events {
            self.apply_single_event(event)?;
        }

        Ok(())
    }

    /// Compress multiple AddToField events for same key+field
    fn compress_events(&self, events: &[CollectionEvent]) -> Vec<CollectionEvent> {
        let mut field_deltas: HashMap<(String, CellLocation, String), f64> = HashMap::new();
        let mut other_events = Vec::new();

        for event in events {
            match event {
                CollectionEvent::AddToField { collection, key, field, delta } => {
                    let key_tuple = (collection.clone(), key.clone(), field.clone());
                    *field_deltas.entry(key_tuple).or_insert(0.0) += delta;
                },
                other => other_events.push(other.clone()),
            }
        }

        // Convert compressed deltas back to events
        let mut compressed = Vec::new();
        for ((collection, key, field), total_delta) in field_deltas {
            compressed.push(CollectionEvent::AddToField {
                collection,
                key,
                field,
                delta: total_delta,
            });
        }
        compressed.extend(other_events);
        compressed
    }

    /// Apply single event using field name matching
    fn apply_single_event(&mut self, event: CollectionEvent) -> Result<(), String> {
        match event {
            CollectionEvent::AddToField { collection, key, field, delta } => {
                self.apply_field_delta(&collection, &key, &field, delta)
            },
            CollectionEvent::SetField { collection, key, field, value } => {
                self.apply_field_set(&collection, &key, &field, value)
            },
            CollectionEvent::CreateCell { collection: _, key: _, data: _ } => {
                // Handle cell creation (would need specific implementation per data type)
                Ok(())
            },
            CollectionEvent::DeleteCell { collection: _, key: _ } => {
                // Handle cell deletion
                Ok(())
            },
        }
    }

    /// Apply field delta using structural pattern matching
    fn apply_field_delta(&mut self, collection_name: &str, key: &CellLocation, field: &str, delta: f64) -> Result<(), String> {
        // This is a placeholder - in real implementation, you'd need to handle
        // different collection types generically or use a trait system
        if collection_name == "geological_cells" {
            if let Some(collection) = self.collections.get_mut(collection_name) {
                if let Some(cells) = collection.downcast_mut::<crate::collections::Collection<CellLocation, crate::simulation::GeologicalCellData>>() {
                    cells.modify(key, |cell_data| {
                        match field {
                            "temperature_k" => cell_data.temperature_k += delta,
                            "pressure_pa" => cell_data.pressure_pa += delta,
                            "density_kg_m3" => cell_data.density_kg_m3 += delta,
                            "energy_joules" => cell_data.energy_mass.add_energy_joules(delta),
                            "mass_kg" => cell_data.energy_mass.add_mass_kg(delta),
                            _ => {}, // Unknown field - could log warning
                        }
                    });
                }
            }
        }
        Ok(())
    }

    /// Apply field set using structural pattern matching
    fn apply_field_set(&mut self, collection_name: &str, key: &CellLocation, field: &str, value: f64) -> Result<(), String> {
        if collection_name == "geological_cells" {
            if let Some(collection) = self.collections.get_mut(collection_name) {
                if let Some(cells) = collection.downcast_mut::<crate::collections::Collection<CellLocation, crate::simulation::GeologicalCellData>>() {
                    cells.modify(key, |cell_data| {
                        match field {
                            "temperature_k" => cell_data.temperature_k = value,
                            "pressure_pa" => cell_data.pressure_pa = value,
                            "density_kg_m3" => cell_data.density_kg_m3 = value,
                            "energy_joules" => cell_data.energy_mass.set_energy_joules(value),
                            "mass_kg" => cell_data.energy_mass.set_mass_kg(value),
                            _ => {}, // Unknown field
                        }
                    });
                }
            }
        }
        Ok(())
    }

    /// Apply a pre-blended array of events (for Actor pattern)
    pub fn apply_events(&mut self, events: Vec<CollectionEvent>) -> Result<(), String> {
        for event in events {
            self.apply_single_event(event)?;
        }
        Ok(())
    }

    /// Apply a batch of changes atomically (legacy method)
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

/// Thread-safe event emitter for components
#[derive(Debug, Clone)]
pub struct EventEmitter {
    sender: Sender<CollectionEvent>,
}

impl EventEmitter {
    /// Emit an AddToField event (thread-safe)
    pub fn add_to_field(&self, collection: &str, key: CellLocation, field: &str, delta: f64) {
        let event = CollectionEvent::AddToField {
            collection: collection.to_string(),
            key,
            field: field.to_string(),
            delta,
        };
        let _ = self.sender.send(event); // Ignore send errors for now
    }

    /// Emit a SetField event (thread-safe)
    pub fn set_field(&self, collection: &str, key: CellLocation, field: &str, value: f64) {
        let event = CollectionEvent::SetField {
            collection: collection.to_string(),
            key,
            field: field.to_string(),
            value,
        };
        let _ = self.sender.send(event);
    }

    /// Emit a DeleteCell event (thread-safe)
    pub fn delete_cell(&self, collection: &str, key: CellLocation) {
        let event = CollectionEvent::DeleteCell {
            collection: collection.to_string(),
            key,
        };
        let _ = self.sender.send(event);
    }
}


