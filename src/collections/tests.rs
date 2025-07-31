#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::{CollectionsManager, ChangeFactory};
    use crate::collections::changes::{FooData, BarData, FooChange, BarChange, CollectionChange};
    use std::thread;
    use std::sync::{Arc, Mutex};

    fn setup_manager() -> CollectionsManager {
        let config = crate::simulation::SimulationConfig {
            planet: crate::simulation::PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
                surface_temperature_k: 288.15,
            },
            years_per_step: 1000,
            steps: 1,
            layers: vec![],
        };
        let mut manager = CollectionsManager::new();
        manager.add_empty_collection::<u32, FooData>("FOO");
        manager.add_empty_collection::<u32, BarData>("BAR");
        manager
    }

    #[test]
    fn test_basic_collection_access() {
        let manager = setup_manager();

        // Test type-safe access
        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        let bar_collection = manager.get::<u32, BarData>("BAR").unwrap();

        assert_eq!(foo_collection.len(), 0);
        assert_eq!(bar_collection.len(), 0);
        assert!(foo_collection.is_empty());
        assert!(bar_collection.is_empty());
    }

    #[test]
    fn test_foo_changes() {
        let mut manager = setup_manager();

        // Create some foo items
        let changes = vec![
            Box::new(FooChange::Create(FooData { id: 1, value: 10.0, name: "first".to_string() })) as Box<dyn CollectionChange>,
            Box::new(FooChange::Create(FooData { id: 2, value: 20.0, name: "second".to_string() })) as Box<dyn CollectionChange>,
        ];

        manager.apply_changes(changes).unwrap();

        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        assert_eq!(foo_collection.len(), 2);

        let item1 = foo_collection.get(&1).unwrap();
        assert_eq!(item1.value, 10.0);
        assert_eq!(item1.name, "first");

        let item2 = foo_collection.get(&2).unwrap();
        assert_eq!(item2.value, 20.0);
        assert_eq!(item2.name, "second");
    }

    #[test]
    fn test_foo_updates() {
        let mut manager = setup_manager();

        // Create and then update
        let changes = vec![
            Box::new(FooChange::Create(FooData { id: 1, value: 10.0, name: "original".to_string() })) as Box<dyn CollectionChange>,
            Box::new(FooChange::Update { id: 1, value: 99.0 }) as Box<dyn CollectionChange>,
            Box::new(FooChange::UpdateName { id: 1, name: "updated".to_string() }) as Box<dyn CollectionChange>,
        ];

        manager.apply_changes(changes).unwrap();

        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        let item = foo_collection.get(&1).unwrap();
        assert_eq!(item.value, 99.0);
        assert_eq!(item.name, "updated");
    }

    #[test]
    fn test_bar_energy_deltas() {
        let mut manager = setup_manager();

        // Create bar item and apply multiple energy deltas
        let changes = vec![
            Box::new(BarChange::Create(BarData { id: 1, energy: 100.0, temperature: 300.0 })) as Box<dyn CollectionChange>,
            Box::new(BarChange::EnergyDelta { id: 1, delta: 50.0 }) as Box<dyn CollectionChange>,
            Box::new(BarChange::EnergyDelta { id: 1, delta: -20.0 }) as Box<dyn CollectionChange>,
            Box::new(BarChange::EnergyDelta { id: 1, delta: 10.0 }) as Box<dyn CollectionChange>,
        ];

        manager.apply_changes(changes).unwrap();

        let bar_collection = manager.get::<u32, BarData>("BAR").unwrap();
        let item = bar_collection.get(&1).unwrap();
        assert_eq!(item.energy, 140.0); // 100 + 50 - 20 + 10
        assert_eq!(item.temperature, 300.0); // Unchanged
    }

    #[test]
    fn test_change_flattening() {
        let mut manager = setup_manager();

        // Multiple changes to same item - each change applies in order
        let changes = vec![
            Box::new(FooChange::Create(FooData { id: 1, value: 10.0, name: "first".to_string() })) as Box<dyn CollectionChange>,
            Box::new(FooChange::Update { id: 1, value: 20.0 }) as Box<dyn CollectionChange>,
            Box::new(FooChange::Update { id: 1, value: 30.0 }) as Box<dyn CollectionChange>,
            Box::new(FooChange::UpdateName { id: 1, name: "final".to_string() }) as Box<dyn CollectionChange>,
        ];

        manager.apply_changes(changes).unwrap();

        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        let item = foo_collection.get(&1).unwrap();
        assert_eq!(item.value, 30.0); // Final value
        assert_eq!(item.name, "final"); // Final name
    }

    #[test]
    fn test_delete_operations() {
        let mut manager = setup_manager();

        // Create and then delete
        let changes = vec![
            Box::new(FooChange::Create(FooData { id: 1, value: 10.0, name: "temp".to_string() })) as Box<dyn CollectionChange>,
            Box::new(FooChange::Create(FooData { id: 2, value: 20.0, name: "keep".to_string() })) as Box<dyn CollectionChange>,
            Box::new(FooChange::Delete(1)) as Box<dyn CollectionChange>,
        ];

        manager.apply_changes(changes).unwrap();

        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        assert_eq!(foo_collection.len(), 1);
        assert!(foo_collection.get(&1).is_none());
        assert!(foo_collection.get(&2).is_some());
    }

    #[test]
    fn test_collections_system_integration() {
        println!("🧪 Testing Collections System with crossbeam + dashmap");

        // Setup collections manager
        let config = crate::simulation::SimulationConfig {
            planet: crate::simulation::PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
                surface_temperature_k: 288.15,
            },
            years_per_step: 1000,
            steps: 1,
            layers: vec![],
        };
        let mut manager = CollectionsManager::new();
        manager.add_empty_collection::<u32, FooData>("FOO");
        manager.add_empty_collection::<u32, BarData>("BAR");

        println!("✅ Collections manager created with FOO and BAR collections");

        // Test basic access
        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        println!("✅ FOO collection accessed, length: {}", foo_collection.len());

        let bar_collection = manager.get::<u32, BarData>("BAR").unwrap();
        println!("✅ BAR collection accessed, length: {}", bar_collection.len());

        // Test adding data
        let changes = vec![
            Box::new(FooChange::Create(FooData { id: 1, value: 100.0, name: "test".to_string() })) as Box<dyn CollectionChange>,
            Box::new(BarChange::Create(BarData { id: 1, energy: 1000.0, temperature: 300.0 })) as Box<dyn CollectionChange>,
        ];

        println!("📝 Applying {} changes...", changes.len());
        manager.apply_changes(changes).unwrap();

        // Check results
        let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
        println!("✅ FOO collection after changes, length: {}", foo_collection.len());

        if let Some(item) = foo_collection.get(&1) {
            println!("  📊 Item 1: value={}, name={}", item.value, item.name);
        }

        let bar_collection = manager.get::<u32, BarData>("BAR").unwrap();
        println!("✅ BAR collection after changes, length: {}", bar_collection.len());

        if let Some(item) = bar_collection.get(&1) {
            println!("  🔋 Item 1: energy={}, temperature={}", item.energy, item.temperature);
        }

        println!("🎉 Collections system test completed successfully!");

        // Verify the data is actually there
        assert_eq!(foo_collection.len(), 1);
        assert_eq!(bar_collection.len(), 1);
    }

    #[test]
    fn test_parallel_collections_execution() {
        use std::sync::Arc;

        println!("🚀 Testing Parallel Collections System");

        // Setup collections manager
        let config = crate::simulation::SimulationConfig {
            planet: crate::simulation::PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
                surface_temperature_k: 288.15,
            },
            years_per_step: 1000,
            steps: 1,
            layers: vec![],
        };
        let mut manager = CollectionsManager::new();
        manager.add_empty_collection::<u32, FooData>("FOO");
        manager.add_empty_collection::<u32, BarData>("BAR");

        // Add initial data
        let initial_changes = vec![
            Box::new(FooChange::Create(FooData { id: 1, value: 100.0, name: "initial".to_string() })) as Box<dyn CollectionChange>,
            Box::new(BarChange::Create(BarData { id: 1, energy: 1000.0, temperature: 300.0 })) as Box<dyn CollectionChange>,
            Box::new(FooChange::Create(FooData { id: 2, value: 200.0, name: "second".to_string() })) as Box<dyn CollectionChange>,
            Box::new(BarChange::Create(BarData { id: 2, energy: 2000.0, temperature: 400.0 })) as Box<dyn CollectionChange>,
        ];
        manager.apply_changes(initial_changes).unwrap();

        // Share collections across threads
        let manager = Arc::new(manager);

        println!("🔧 Starting parallel component execution...");

        // Execute components in parallel using crossbeam
        let all_changes = crossbeam::scope(|s| {
            let manager1 = Arc::clone(&manager);
            let manager2 = Arc::clone(&manager);
            let manager3 = Arc::clone(&manager);

            // Component 1: Foo processor
            let handle1 = s.spawn(move |_| {
                println!("  🔧 Component 1: Processing FOO collection");
                let foo_collection = manager1.get::<u32, FooData>("FOO").unwrap();

                let mut changes = Vec::new();

                // Read and process all foo items concurrently
                for entry in foo_collection.iter() {
                    let (id, data) = (entry.key(), entry.value());
                    println!("    📊 Component 1: Processing foo {} with value {}", id, data.value);

                    // Create changes based on current data
                    if data.value < 150.0 {
                        changes.push(Box::new(FooChange::Update { id: *id, value: data.value * 1.5 }) as Box<dyn CollectionChange>);
                    }
                }

                changes
            });

            // Component 2: Bar energy processor
            let handle2 = s.spawn(move |_| {
                println!("  ⚡ Component 2: Processing BAR energy");
                let bar_collection = manager2.get::<u32, BarData>("BAR").unwrap();

                let mut changes = Vec::new();

                // Process energy deltas concurrently
                for entry in bar_collection.iter() {
                    let (id, data) = (entry.key(), entry.value());
                    println!("    🔋 Component 2: Processing bar {} with energy {}", id, data.energy);

                    // Add energy based on temperature
                    let energy_delta = data.temperature * 0.1;
                    changes.push(Box::new(BarChange::EnergyDelta { id: *id, delta: energy_delta }) as Box<dyn CollectionChange>);
                }

                changes
            });

            // Component 3: Cross-collection processor
            let handle3 = s.spawn(move |_| {
                println!("  🔄 Component 3: Cross-collection processing");
                let foo_collection = manager3.get::<u32, FooData>("FOO").unwrap();
                let bar_collection = manager3.get::<u32, BarData>("BAR").unwrap();

                let mut changes = Vec::new();

                // Process relationships between collections
                for foo_entry in foo_collection.iter() {
                    let (foo_id, foo_data) = (foo_entry.key(), foo_entry.value());

                    if let Some(bar_entry) = bar_collection.get(foo_id) {
                        let bar_data = bar_entry.value();
                        println!("    🔗 Component 3: Linking foo {} (value: {}) with bar {} (energy: {})",
                                 foo_id, foo_data.value, foo_id, bar_data.energy);

                        // Create temperature change based on foo value
                        let temp_delta = foo_data.value * 0.01;
                        changes.push(Box::new(BarChange::TemperatureDelta { id: *foo_id, delta: temp_delta }) as Box<dyn CollectionChange>);
                    }
                }

                changes
            });

            // Collect all changes from parallel components
            let changes1 = handle1.join().unwrap();
            let changes2 = handle2.join().unwrap();
            let changes3 = handle3.join().unwrap();

            // Combine all changes
            let mut all_changes = Vec::new();
            all_changes.extend(changes1);
            all_changes.extend(changes2);
            all_changes.extend(changes3);

            println!("📝 Collected {} changes from parallel components", all_changes.len());
            all_changes

        }).unwrap();

        println!("✅ Parallel execution completed with {} total changes", all_changes.len());
        println!("🎉 Parallel collections system test completed successfully!");

        // Verify we got the expected number of changes
        assert!(all_changes.len() > 0);
        assert!(all_changes.len() <= 10); // Reasonable upper bound
    }
}
