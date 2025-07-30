use crate::collections::{CollectionsManager, CollectionChange};
use crate::collections::changes::{FooData, BarData, FooChange, BarChange};
use std::sync::Arc;
use crossbeam;

/// Example of parallel component execution with concurrent collections
pub fn run_parallel_components_example() -> Vec<Box<dyn CollectionChange>> {
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
    
    // Add some initial data
    let initial_changes = vec![
        Box::new(FooChange::Create(FooData { id: 1, value: 100.0, name: "initial".to_string() })) as Box<dyn CollectionChange>,
        Box::new(BarChange::Create(BarData { id: 1, energy: 1000.0, temperature: 300.0 })) as Box<dyn CollectionChange>,
        Box::new(FooChange::Create(FooData { id: 2, value: 200.0, name: "second".to_string() })) as Box<dyn CollectionChange>,
        Box::new(BarChange::Create(BarData { id: 2, energy: 2000.0, temperature: 400.0 })) as Box<dyn CollectionChange>,
    ];
    manager.apply_changes(initial_changes).unwrap();
    
    // Share collections across threads
    let manager = Arc::new(manager);
    
    println!("🚀 Starting parallel component execution...");
    
    // Execute components in parallel using crossbeam
    crossbeam::scope(|s| {
        let manager1 = Arc::clone(&manager);
        let manager2 = Arc::clone(&manager);
        let manager3 = Arc::clone(&manager);
        
        // Component 1: Foo processor
        let handle1 = s.spawn(move |_| {
            println!("🔧 Component 1: Processing FOO collection");
            let foo_collection = manager1.get::<u32, FooData>("FOO").unwrap();
            
            let mut changes = Vec::new();
            
            // Read and process all foo items concurrently
            for entry in foo_collection.iter() {
                let (id, data) = (entry.key(), entry.value());
                println!("  📊 Component 1: Processing foo {} with value {}", id, data.value);
                
                // Create changes based on current data
                if data.value < 150.0 {
                    changes.push(Box::new(FooChange::Update { id: *id, value: data.value * 1.5 }) as Box<dyn CollectionChange>);
                }
            }
            
            changes
        });
        
        // Component 2: Bar energy processor
        let handle2 = s.spawn(move |_| {
            println!("⚡ Component 2: Processing BAR energy");
            let bar_collection = manager2.get::<u32, BarData>("BAR").unwrap();
            
            let mut changes = Vec::new();
            
            // Process energy deltas concurrently
            for entry in bar_collection.iter() {
                let (id, data) = (entry.key(), entry.value());
                println!("  🔋 Component 2: Processing bar {} with energy {}", id, data.energy);
                
                // Add energy based on temperature
                let energy_delta = data.temperature * 0.1;
                changes.push(Box::new(BarChange::EnergyDelta { id: *id, delta: energy_delta }) as Box<dyn CollectionChange>);
            }
            
            changes
        });
        
        // Component 3: Cross-collection processor
        let handle3 = s.spawn(move |_| {
            println!("🔄 Component 3: Cross-collection processing");
            let foo_collection = manager3.get::<u32, FooData>("FOO").unwrap();
            let bar_collection = manager3.get::<u32, BarData>("BAR").unwrap();
            
            let mut changes = Vec::new();
            
            // Process relationships between collections
            for foo_entry in foo_collection.iter() {
                let (foo_id, foo_data) = (foo_entry.key(), foo_entry.value());
                
                if let Some(bar_entry) = bar_collection.get(foo_id) {
                    let bar_data = bar_entry.value();
                    println!("  🔗 Component 3: Linking foo {} (value: {}) with bar {} (energy: {})", 
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
        
    }).unwrap()
}

/// Example pure functional component
pub fn process_foo_component(manager: &CollectionsManager) -> Vec<Box<dyn CollectionChange>> {
    let foo_collection = manager.get::<u32, FooData>("FOO").unwrap();
    let mut changes = Vec::new();
    
    // Pure functional processing - no side effects
    for entry in foo_collection.iter() {
        let (id, data) = (entry.key(), entry.value());
        
        if data.value > 100.0 {
            changes.push(Box::new(FooChange::UpdateName { 
                id: *id, 
                name: format!("processed_{}", data.name) 
            }) as Box<dyn CollectionChange>);
        }
    }
    
    changes
}

/// Example of batched parallel execution
pub fn run_batched_parallel_execution() {
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
    
    // Add test data
    for i in 1..=10 {
        let changes = vec![
            Box::new(FooChange::Create(FooData { 
                id: i, 
                value: i as f64 * 10.0, 
                name: format!("item_{}", i) 
            })) as Box<dyn CollectionChange>,
        ];
        manager.apply_changes(changes).unwrap();
    }
    
    let manager = Arc::new(manager);
    
    println!("🔄 Running batched parallel execution...");
    
    // Multiple rounds of parallel processing
    for round in 1..=3 {
        println!("📍 Round {}", round);
        
        let all_changes = crossbeam::scope(|s| {
            let mut handles = Vec::new();
            
            // Spawn multiple parallel processors
            for _worker_id in 1..=4 {
                let manager_clone = Arc::clone(&manager);
                let handle = s.spawn(move |_| {
                    process_foo_component(&manager_clone)
                });
                handles.push(handle);
            }
            
            // Collect all changes
            let mut all_changes = Vec::new();
            for handle in handles {
                all_changes.extend(handle.join().unwrap());
            }
            all_changes
        }).unwrap();
        
        println!("  ✅ Round {} completed with {} changes", round, all_changes.len());
    }
}
