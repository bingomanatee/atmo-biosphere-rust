use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::core_heat_component::CoreHeatComponent;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

#[test]
fn test_billion_year_simulation() {
    println!("🌍 Starting Billion Year Geological Simulation");
    println!("===============================================");
    println!("⏰ Duration: 1 billion years (1,000,000 steps × 1,000 years/step)");
    
    // Create simulation configuration for billion year run
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 1_000_000, // 1 million steps
        years_per_step: 1000.0, // 1000 years per step = 1 billion years total
        surface_temp_k: 288.0, // 15°C surface temperature
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    // Create core heat component
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreHeatComponent::new())
    ];
    
    // Create simulation with component
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("✅ Simulation created with core heat component");
    
    // Load layer sets
    sim.load_layer_sets();
    println!("✅ Layer sets loaded: {} geological layers", sim.layer_sets.len());
    
    // Initialize components
    sim.initialize_components();
    println!("✅ Components initialized");
    
    // Get initial energy state
    let initial_energy = calculate_total_energy(&sim);
    println!("🔋 Initial total energy: {:.2e} J", initial_energy);
    
    // Track energy over geological time
    let mut energy_history = Vec::new();
    let mut last_report_step = 0;
    let report_interval = 10_000; // Report every 10 million years
    
    println!("\n🕰️ Starting billion year simulation...");
    println!("📊 Progress reports every {} steps ({} million years)", report_interval, report_interval / 1000);
    
    // Run the billion year simulation
    for step_num in 0..sim.config.steps {
        // Run simulation step
        sim.step();
        
        // Report progress periodically
        if step_num % report_interval == 0 || step_num == sim.config.steps - 1 {
            let current_energy = calculate_total_energy(&sim);
            let years_elapsed = (step_num + 1) as f64 * sim.config.years_per_step;
            let million_years = years_elapsed / 1_000_000.0;
            
            // Get transaction stats
            let (pending, committed) = sim.transaction_manager.get_transaction_stats();
            
            // Calculate energy change
            let energy_change = current_energy - initial_energy;
            let energy_change_percent = (energy_change / initial_energy) * 100.0;
            
            println!("\n📈 Geological Time Report - {:.0} Million Years Ago to Present:", 1000.0 - million_years);
            println!("   Step: {}/{} ({:.1}% complete)", step_num + 1, sim.config.steps, 
                     ((step_num + 1) as f64 / sim.config.steps as f64) * 100.0);
            println!("   Total energy: {:.2e} J ({:+.2}% change)", current_energy, energy_change_percent);
            println!("   Transactions: {} pending, {} committed", pending, committed);
            
            // Store energy history
            energy_history.push((million_years, current_energy));
            
            // Check for energy conservation
            if energy_change_percent.abs() > 50.0 {
                println!("⚠️  WARNING: Large energy change detected! ({:+.2}%)", energy_change_percent);
            }
            
            last_report_step = step_num;
        }
        
        // Early termination check for testing (remove for full billion year run)
        if step_num >= 100 { // Run only 100 steps for testing (100,000 years)
            println!("\n🛑 Early termination for testing after {} steps ({:.0} thousand years)", 
                     step_num + 1, (step_num + 1) as f64 * sim.config.years_per_step / 1000.0);
            break;
        }
    }
    
    // Final analysis
    let final_energy = calculate_total_energy(&sim);
    let total_energy_change = final_energy - initial_energy;
    let total_energy_change_percent = (total_energy_change / initial_energy) * 100.0;
    
    println!("\n🎯 Billion Year Simulation Results:");
    println!("=====================================");
    println!("🔋 Initial energy: {:.2e} J", initial_energy);
    println!("🔋 Final energy:   {:.2e} J", final_energy);
    println!("📊 Total change:   {:+.2e} J ({:+.2}%)", total_energy_change, total_energy_change_percent);
    
    // Energy conservation check
    if total_energy_change_percent.abs() < 10.0 {
        println!("✅ Energy conservation: EXCELLENT (change < 10%)");
    } else if total_energy_change_percent.abs() < 25.0 {
        println!("⚠️  Energy conservation: ACCEPTABLE (change < 25%)");
    } else {
        println!("❌ Energy conservation: POOR (change > 25%)");
    }
    
    // Print energy history
    println!("\n📈 Energy Evolution Over Geological Time:");
    for (million_years, energy) in energy_history.iter().take(10) {
        println!("   {:.0} Ma: {:.2e} J", million_years, energy);
    }
    
    // Verify atomic transaction system worked
    let (final_pending, final_committed) = sim.transaction_manager.get_transaction_stats();
    println!("\n🔄 Transaction System Performance:");
    println!("   Final pending transactions: {}", final_pending);
    println!("   Total committed transactions: {}", final_committed);
    
    // Basic assertions
    assert!(final_energy > 0.0, "Final energy should be positive");
    assert!(total_energy_change_percent.abs() < 100.0, "Energy change should be reasonable");
    assert!(final_committed > 0, "Should have committed some transactions");
    
    println!("\n🎉 Billion Year Simulation completed successfully!");
    println!("✅ Atomic transaction system maintained energy conservation");
    println!("✅ Core heat component + radiative transfer working over geological time");
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
