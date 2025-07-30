use crate::cell_location::CellLocation;
use crate::collections::CollectionEvent;

/// Actor that accumulates changes in its own queue
#[derive(Debug)]
pub struct Actor {
    pub changes: Vec<CollectionEvent>,
}

impl Actor {
    /// Create new empty actor
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }
    
    /// Add delta to a field (compressible operation)
    pub fn add(&mut self, collection: &str, key: CellLocation, field: &str, delta: f64) {
        self.changes.push(CollectionEvent::AddToField {
            collection: collection.to_string(),
            key,
            field: field.to_string(),
            delta,
        });
    }
    
    /// Replace/set field to absolute value
    pub fn replace(&mut self, collection: &str, key: CellLocation, field: &str, value: f64) {
        self.changes.push(CollectionEvent::SetField {
            collection: collection.to_string(),
            key,
            field: field.to_string(),
            value,
        });
    }
    
    /// Delete a cell
    pub fn delete(&mut self, collection: &str, key: CellLocation) {
        self.changes.push(CollectionEvent::DeleteCell {
            collection: collection.to_string(),
            key,
        });
    }
    
    /// Create a new cell (placeholder - would need specific data type)
    pub fn create(&mut self, collection: &str, key: CellLocation) {
        // For now, just add a placeholder delete event
        // In real implementation, would need to handle specific data types
        self.changes.push(CollectionEvent::DeleteCell {
            collection: collection.to_string(),
            key,
        });
    }
    
    /// Get number of changes queued
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
    
    /// Clear all changes
    pub fn clear(&mut self) {
        self.changes.clear();
    }
    
    /// Get reference to changes (for inspection)
    pub fn get_changes(&self) -> &[CollectionEvent] {
        &self.changes
    }
    
    /// Take ownership of changes (consumes the actor's queue)
    pub fn take_changes(self) -> Vec<CollectionEvent> {
        self.changes
    }
}

/// Controller for blending multiple actor change queues
#[derive(Debug)]
pub struct ChangeController;

impl ChangeController {
    /// Blend multiple actor change arrays into a single compressed array
    pub fn blend(actors: Vec<Actor>) -> Vec<CollectionEvent> {
        // Collect all changes from all actors
        let mut all_changes = Vec::new();
        for actor in actors {
            all_changes.extend(actor.take_changes());
        }
        
        // Compress the changes
        Self::compress_changes(all_changes)
    }
    
    /// Blend change arrays directly (alternative interface)
    pub fn blend_arrays(change_arrays: Vec<Vec<CollectionEvent>>) -> Vec<CollectionEvent> {
        let mut all_changes = Vec::new();
        for changes in change_arrays {
            all_changes.extend(changes);
        }
        
        Self::compress_changes(all_changes)
    }
    
    /// Compress multiple AddToField events for same key+field
    fn compress_changes(changes: Vec<CollectionEvent>) -> Vec<CollectionEvent> {
        use std::collections::HashMap;
        
        let mut field_deltas: HashMap<(String, CellLocation, String), f64> = HashMap::new();
        let mut other_changes = Vec::new();
        
        // Separate AddToField events from others
        for change in changes {
            match change {
                CollectionEvent::AddToField { collection, key, field, delta } => {
                    let key_tuple = (collection, key, field);
                    *field_deltas.entry(key_tuple).or_insert(0.0) += delta;
                },
                other => other_changes.push(other),
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
        
        // Add non-compressible events
        compressed.extend(other_changes);
        compressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use h3o::{CellIndex, Resolution};
    
    fn create_test_location() -> CellLocation {
        let h3_cell = CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap_or_else(|_| {
            use h3o::LatLng;
            LatLng::new(37.7749, -122.4194).unwrap().to_cell(Resolution::Five)
        });
        CellLocation::new(0, h3_cell, 0)
    }
    
    #[test]
    fn test_actor_crud_operations() {
        let mut actor = Actor::new();
        let location = create_test_location();
        
        // Test CRUD operations
        actor.add("GEOLOGICAL_CELLS", location, "temperature_k", 5.0);
        actor.replace("GEOLOGICAL_CELLS", location, "pressure_pa", 101325.0);
        actor.delete("GEOLOGICAL_CELLS", location);
        
        assert_eq!(actor.change_count(), 3);
        
        let changes = actor.get_changes();
        assert!(matches!(changes[0], CollectionEvent::AddToField { .. }));
        assert!(matches!(changes[1], CollectionEvent::SetField { .. }));
        assert!(matches!(changes[2], CollectionEvent::DeleteCell { .. }));
    }
    
    #[test]
    fn test_change_controller_blend() {
        let location = create_test_location();
        
        // Create multiple actors with changes
        let mut actor1 = Actor::new();
        actor1.add("GEOLOGICAL_CELLS", location, "temperature_k", 5.0);
        actor1.add("GEOLOGICAL_CELLS", location, "energy_joules", 1000.0);
        
        let mut actor2 = Actor::new();
        actor2.add("GEOLOGICAL_CELLS", location, "temperature_k", 3.0); // Should compress
        actor2.replace("GEOLOGICAL_CELLS", location, "pressure_pa", 101325.0);
        
        let mut actor3 = Actor::new();
        actor3.add("GEOLOGICAL_CELLS", location, "temperature_k", 2.0); // Should compress
        
        // Blend all actors
        let blended = ChangeController::blend(vec![actor1, actor2, actor3]);
        
        // Should have compressed temperature changes: 5.0 + 3.0 + 2.0 = 10.0
        let temp_change = blended.iter().find(|change| {
            matches!(change, CollectionEvent::AddToField { field, delta, .. } 
                     if field == "temperature_k" && *delta == 10.0)
        });
        assert!(temp_change.is_some(), "Temperature changes should be compressed to 10.0");
        
        // Should have energy change: 1000.0
        let energy_change = blended.iter().find(|change| {
            matches!(change, CollectionEvent::AddToField { field, delta, .. } 
                     if field == "energy_joules" && *delta == 1000.0)
        });
        assert!(energy_change.is_some(), "Energy change should be preserved");
        
        // Should have pressure set: 101325.0
        let pressure_change = blended.iter().find(|change| {
            matches!(change, CollectionEvent::SetField { field, value, .. } 
                     if field == "pressure_pa" && *value == 101325.0)
        });
        assert!(pressure_change.is_some(), "Pressure set should be preserved");
        
        // Verify compression worked
    }
    
    #[test]
    fn test_actor_clear_and_take() {
        let mut actor = Actor::new();
        let location = create_test_location();
        
        actor.add("GEOLOGICAL_CELLS", location, "temperature_k", 5.0);
        actor.add("GEOLOGICAL_CELLS", location, "pressure_pa", 1000.0);
        
        assert_eq!(actor.change_count(), 2);
        
        // Test clear
        actor.clear();
        assert_eq!(actor.change_count(), 0);
        
        // Add more changes
        actor.add("GEOLOGICAL_CELLS", location, "temperature_k", 10.0);
        assert_eq!(actor.change_count(), 1);
        
        // Test take_changes (consumes)
        let changes = actor.take_changes();
        assert_eq!(changes.len(), 1);
        // Actor is consumed, can't check its state
    }
}
