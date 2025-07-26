use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use h3o::Resolution;

#[test]
fn test_minimal_integration() {
    println!("🔧 MINIMAL INTEGRATION TEST");
    println!("===========================");
    
    // Create minimal simulation
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 1,
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    let mut components = vec![];
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("✅ Simulation created");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    println!("   - Total cells: {}", sim.total_cells());
    
    // Check if binary pairing system is integrated
    let (pairs_processed, listener_calls, total_pairs) = sim.binary_pairing_system.get_performance_stats();
    println!("✅ Binary pairing system integrated:");
    println!("   - Total pairs: {}", total_pairs);
    println!("   - Pairs processed: {}", pairs_processed);
    println!("   - Listener calls: {}", listener_calls);
    
    // Check if simple transaction manager is integrated
    let metrics = sim.simple_transaction_manager.get_performance_metrics();
    println!("✅ Simple transaction manager integrated:");
    println!("   - Total transactions: {}", metrics.total_transactions);
    
    // Test if step_with_binary_pairing method exists and can be called
    println!("🔄 Testing step_with_binary_pairing method...");
    sim.step_with_binary_pairing();
    println!("✅ step_with_binary_pairing method works!");
    
    // Check performance after one step
    let (pairs_processed_after, listener_calls_after, total_pairs_after) = sim.binary_pairing_system.get_performance_stats();
    println!("✅ After one step:");
    println!("   - Total pairs: {}", total_pairs_after);
    println!("   - Pairs processed: {}", pairs_processed_after);
    println!("   - Listener calls: {}", listener_calls_after);
    
    let metrics_after = sim.simple_transaction_manager.get_performance_metrics();
    println!("   - Total transactions: {}", metrics_after.total_transactions);
    
    // Validation
    assert!(total_pairs_after > 0, "Should have binary pairs");
    assert!(pairs_processed_after > pairs_processed, "Should have processed pairs");
    assert!(listener_calls_after > listener_calls, "Should have called listeners");
    
    println!("\n🎉 MINIMAL INTEGRATION TEST PASSED!");
    println!("   ✅ BinaryPairingSystem: INTEGRATED");
    println!("   ✅ SimpleTransactionManager: INTEGRATED");
    println!("   ✅ step_with_binary_pairing(): WORKING");
    println!("   ✅ Component listeners: INTEGRATED");
    println!("   ✅ All systems working together!");
}
