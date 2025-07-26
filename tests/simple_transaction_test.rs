use atmo_biosphere_rust::transaction_manager_simple::{SimpleTransactionManager, CellLocation};
use h3o::CellIndex;

#[test]
fn test_simple_transaction_manager_basic() {
    println!("🧪 Testing Simple Transaction Manager");
    
    let mut manager = SimpleTransactionManager::new();
    
    let location = CellLocation {
        layer_set_index: 0,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 0,
    };
    
    // Test energy delta
    manager.add_energy_delta(location.clone(), 100.0, "test");
    assert_eq!(manager.get_energy_delta(&location), 100.0);
    
    // Test accumulation
    manager.add_energy_delta(location.clone(), 50.0, "test");
    assert_eq!(manager.get_energy_delta(&location), 150.0);
    
    // Test mass delta
    manager.add_mass_delta(location.clone(), 10.0, "test");
    assert_eq!(manager.get_mass_delta(&location), 10.0);
    
    // Test combined operation
    let location2 = CellLocation {
        layer_set_index: 1,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 1,
    };
    
    manager.add_energy_mass_delta(location2.clone(), 200.0, 20.0, "combined_test");
    assert_eq!(manager.get_energy_delta(&location2), 200.0);
    assert_eq!(manager.get_mass_delta(&location2), 20.0);
    
    // Check stats
    let (pending, total) = manager.get_transaction_stats();
    assert_eq!(total, 4); // 4 transactions added
    assert!(pending > 0); // Should have pending deltas
    
    println!("✅ Basic functionality works");
    
    // Test clearing
    manager.clear_deltas();
    assert_eq!(manager.get_energy_delta(&location), 0.0);
    assert_eq!(manager.get_mass_delta(&location), 0.0);
    
    println!("✅ Clear functionality works");
}

#[test]
fn test_simple_transaction_manager_debug() {
    println!("🧪 Testing Simple Transaction Manager Debug Mode");
    
    let mut manager = SimpleTransactionManager::new_with_debug();
    
    let location = CellLocation {
        layer_set_index: 0,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 0,
    };
    
    manager.add_energy_delta(location.clone(), 100.0, "radiative_transfer");
    manager.add_mass_delta(location.clone(), 10.0, "component_test");

    // Add balancing energy delta to conserve energy
    let location2 = CellLocation {
        layer_set_index: 1,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 1,
    };
    manager.add_energy_delta(location2, -100.0, "energy_balance");

    // Check debug info is available
    let debug_info = manager.get_debug_info().unwrap();
    assert_eq!(debug_info.len(), 3);
    assert_eq!(debug_info[0].energy_delta, 100.0);
    assert_eq!(debug_info[0].source, "radiative_transfer");
    assert_eq!(debug_info[1].mass_delta, 10.0);
    assert_eq!(debug_info[1].source, "component_test");
    assert_eq!(debug_info[2].energy_delta, -100.0);
    assert_eq!(debug_info[2].source, "energy_balance");

    println!("✅ Debug mode works");

    // Test energy conservation validation (should pass with balanced energy)
    let result = manager.validate_energy_conservation(1e-6);
    assert!(result.is_ok(), "Energy conservation should pass with balanced energy");
    
    println!("✅ Energy conservation validation works");
}

#[test]
fn test_simple_transaction_manager_performance() {
    println!("🧪 Testing Simple Transaction Manager Performance");
    
    let mut manager = SimpleTransactionManager::new();
    
    let start = std::time::Instant::now();
    
    // Simulate a typical radiative transfer step with many transactions
    for layer_set in 0..6 {
        for cell_idx in 0..250 {
            let location = CellLocation {
                layer_set_index: layer_set,
                h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
                cell_index: cell_idx,
            };
            
            // Simulate energy transfer
            manager.add_energy_delta(location, 1000.0, "radiative_transfer");
        }
    }
    
    let creation_time = start.elapsed();
    
    // Get all deltas (simulating application to layer sets)
    let energy_deltas = manager.get_all_energy_deltas();
    let mass_deltas = manager.get_all_mass_deltas();
    
    assert_eq!(energy_deltas.len(), 6 * 250); // Should have 1500 energy deltas
    assert_eq!(mass_deltas.len(), 0); // No mass deltas
    
    let retrieval_time = start.elapsed();
    
    // Clear deltas
    manager.clear_deltas();
    let clear_time = start.elapsed();
    
    println!("📊 Performance Results:");
    println!("   - Creation time: {:.2}ms", creation_time.as_secs_f64() * 1000.0);
    println!("   - Retrieval time: {:.2}ms", retrieval_time.as_secs_f64() * 1000.0);
    println!("   - Clear time: {:.2}ms", clear_time.as_secs_f64() * 1000.0);
    println!("   - Total transactions: 1500");
    println!("   - Time per transaction: {:.3}μs", (creation_time.as_secs_f64() * 1_000_000.0) / 1500.0);
    
    // Should be very fast
    assert!(creation_time.as_millis() < 10, "Creation should be under 10ms");
    assert!(clear_time.as_millis() < 5, "Clear should be under 5ms");
    
    println!("✅ Performance is excellent");
}

#[test]
fn test_energy_conservation_hash() {
    println!("🧪 Testing Energy Conservation Hash");
    
    let mut manager = SimpleTransactionManager::new();
    
    let location1 = CellLocation {
        layer_set_index: 0,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 0,
    };
    
    let location2 = CellLocation {
        layer_set_index: 0,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 1,
    };
    
    // Add equal and opposite energy deltas (should conserve energy)
    manager.add_energy_delta(location1, 100.0, "test");
    manager.add_energy_delta(location2, -100.0, "test");
    
    let hash1 = manager.calculate_energy_hash();
    
    // Add more balanced deltas
    manager.add_energy_delta(location1, 50.0, "test");
    manager.add_energy_delta(location2, -50.0, "test");
    
    let hash2 = manager.calculate_energy_hash();
    
    // Hashes should be the same (total energy delta is still 0)
    assert_eq!(hash1, hash2, "Energy conservation hash should be consistent");
    
    println!("✅ Energy conservation hash works");
}

#[test]
fn test_performance_metrics() {
    println!("🧪 Testing Performance Metrics");
    
    let mut manager = SimpleTransactionManager::new();
    manager.set_current_step(42);
    
    let location = CellLocation {
        layer_set_index: 0,
        h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
        cell_index: 0,
    };
    
    manager.add_energy_delta(location.clone(), 100.0, "test");
    manager.add_mass_delta(location, 10.0, "test");
    
    let metrics = manager.get_performance_metrics();
    
    assert_eq!(metrics.total_transactions, 2);
    assert_eq!(metrics.pending_energy_deltas, 1);
    assert_eq!(metrics.pending_mass_deltas, 1);
    assert_eq!(metrics.current_step, 42);
    assert_eq!(metrics.debug_journal_size, 0); // Not in debug mode
    
    println!("✅ Performance metrics work");
    println!("   - Total transactions: {}", metrics.total_transactions);
    println!("   - Pending energy deltas: {}", metrics.pending_energy_deltas);
    println!("   - Pending mass deltas: {}", metrics.pending_mass_deltas);
    println!("   - Current step: {}", metrics.current_step);
}
