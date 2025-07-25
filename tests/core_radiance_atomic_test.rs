use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::core_heat_component::CoreHeatComponent;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

#[test]
fn test_core_radiance_with_atomic_transactions() {
    println!("🧪 Testing Core Radiance Component with Atomic Transactions");
    
    // Create simulation configuration
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 10,
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

    // Initialize components
    sim.initialize_components();
    println!("✅ Components initialized");

    // Get initial energy state
    let initial_energy = calculate_total_energy(&sim);
    println!("🔋 Initial total energy: {:.2e} J", initial_energy);

    // Run a few simulation steps
    for step in 0..3 {
        println!("\n🔄 Running step {}", step + 1);

        // Run components
        sim.step_components(step, step * 1000);
        
        // Check transaction stats
        let (pending, committed) = sim.transaction_manager.get_transaction_stats();
        println!("📊 Transactions - Pending: {}, Committed: {}", pending, committed);
        
        // Validate and apply transactions
        let validated = sim.transaction_manager.validate_and_regulate_transactions(sim.config.years_per_step);
        println!("✅ Validated {} atomic transactions", validated.len());
        
        // Commit transactions
        sim.transaction_manager.commit_transactions(validated);
        
        // Check energy conservation
        let current_energy = calculate_total_energy(&sim);
        println!("🔋 Current total energy: {:.2e} J", current_energy);
        
        // Energy should increase due to core radiance injection
        if step > 0 {
            assert!(current_energy > initial_energy, "Energy should increase due to core radiance");
        }
    }
    
    println!("\n🎉 Core Radiance Component working correctly with atomic transactions!");
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
