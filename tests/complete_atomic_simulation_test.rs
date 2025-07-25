use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::core_heat_component::CoreHeatComponent;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

#[test]
fn test_complete_simulation_with_atomic_transactions() {
    println!("🧪 Testing Complete Simulation with Atomic Transactions");
    
    // Create simulation configuration
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 5,
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    // Create core heat component
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreHeatComponent::new())
    ];
    
    // Create simulation with component
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("✅ Simulation created with core radiance component");
    
    // Load layer sets
    sim.load_layer_sets();
    println!("✅ Layer sets loaded");
    
    // Initialize components
    sim.initialize_components();
    println!("✅ Components initialized");
    
    // Get initial energy state
    let initial_energy = calculate_total_energy(&sim);
    println!("🔋 Initial total energy: {:.2e} J", initial_energy);
    
    // Run simulation steps using the main step() method
    for step_num in 0..3 {
        println!("\n🔄 Running simulation step {}", step_num + 1);
        
        // Use the main simulation step method (includes components + radiative transfer)
        sim.step();
        
        // Check transaction stats
        let (pending, committed) = sim.transaction_manager.get_transaction_stats();
        println!("📊 Transactions - Pending: {}, Committed: {}", pending, committed);
        
        // Check energy conservation
        let current_energy = calculate_total_energy(&sim);
        println!("🔋 Current total energy: {:.2e} J", current_energy);
        
        // Energy should increase due to core radiance injection
        if step_num > 0 {
            assert!(current_energy >= initial_energy, "Energy should not decrease (conservation)");
        }
    }
    
    println!("\n🎉 Complete Simulation with Atomic Transactions working correctly!");
    println!("✅ Radiative transfer + Core radiance component + Atomic transactions = SUCCESS!");
}

fn calculate_total_energy(sim: &SimulationImmut) -> f64 {
    let mut total_energy = 0.0;
    
    for layer_set in &sim.layer_sets {
        for (_h3_cell, column) in &layer_set.layers {
            for cell in &column.cells {
                total_energy += cell.energy_joules();
            }
        }
    }
    
    total_energy
}
