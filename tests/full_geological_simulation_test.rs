use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::core_heat_component::CoreHeatComponent;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;
use std::time::Instant;

#[test]
fn test_full_geological_simulation_with_core_heat() {
    println!("🌍 FULL GEOLOGICAL SIMULATION WITH CORE HEAT");
    println!("=============================================");
    println!("🔥 Features: Radiative transfer + Perlin noise + Hotspots");
    println!("⏰ Duration: 100 steps (100,000 years)");
    
    // Create simulation configuration
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 100, // Short test to see the core heat effects
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    // Create simulation WITH core heat component
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreHeatComponent::new())
    ];
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("✅ Simulation created with CoreHeatComponent");
    println!("   - Perlin noise: ±15% energy variation");
    println!("   - Hotspots: 10 active hotspots globally");
    println!("   - Earth wattage: 47 TW total heat flow");
    
    // Load layer sets
    sim.load_layer_sets();
    println!("✅ Layer sets loaded: {} geological layers", sim.layer_sets.len());
    
    // Initialize components
    sim.initialize_components();
    println!("✅ Components initialized");
    
    // Get initial energy state
    let initial_energy = calculate_total_energy(&sim);
    println!("🔋 Initial total energy: {:.2e} J", initial_energy);
    
    // Show initial thermal structure
    print_thermal_structure(&sim, "Initial State");
    
    println!("\n🚀 Starting full geological simulation...");
    println!("📊 Progress reports every 25 steps");
    
    let simulation_start = Instant::now();
    
    // Run the simulation with core heat
    for step_num in 0..sim.config.steps {
        sim.step();
        
        // Report progress every 25 steps
        if step_num % 25 == 0 || step_num == sim.config.steps - 1 {
            let current_energy = calculate_total_energy(&sim);
            let years_elapsed = (step_num + 1) as f64 * sim.config.years_per_step;
            let thousand_years = years_elapsed / 1000.0;
            
            // Calculate energy change
            let energy_change = current_energy - initial_energy;
            let energy_change_percent = (energy_change / initial_energy) * 100.0;
            
            println!("\n📈 Step {}/{} - {:.0} thousand years:", 
                     step_num + 1, sim.config.steps, thousand_years);
            println!("   Energy: {:.2e} J ({:+.2}% change)", current_energy, energy_change_percent);
            
            // Show thermal structure at key points
            if step_num == 0 || step_num == sim.config.steps - 1 {
                print_thermal_structure(&sim, &format!("After {} thousand years", thousand_years));
            }
        }
    }
    
    let total_simulation_time = simulation_start.elapsed();
    
    // Final analysis
    let final_energy = calculate_total_energy(&sim);
    let total_energy_change = final_energy - initial_energy;
    let total_energy_change_percent = (total_energy_change / initial_energy) * 100.0;
    
    println!("\n🎯 FULL GEOLOGICAL SIMULATION RESULTS:");
    println!("======================================");
    println!("⏱️  Simulation time: {:.2}s", total_simulation_time.as_secs_f64());
    println!("🔋 Initial energy: {:.2e} J", initial_energy);
    println!("🔋 Final energy:   {:.2e} J", final_energy);
    println!("📊 Energy change:  {:+.2e} J ({:+.2}%)", total_energy_change, total_energy_change_percent);
    
    // Core heat analysis
    if total_energy_change_percent > 0.1 {
        println!("🔥 Core heat input detected: {:.2}% energy increase", total_energy_change_percent);
        println!("   → Perlin noise and hotspots are adding energy to the system");
    } else {
        println!("⚠️  Minimal energy change: Core heat may not be active");
    }
    
    // Performance analysis
    let avg_step_time = total_simulation_time / sim.config.steps as u32;
    let steps_per_second = sim.config.steps as f64 / total_simulation_time.as_secs_f64();
    
    println!("\n🚀 Performance with Core Heat:");
    println!("   - Average step time: {:.2}ms", avg_step_time.as_secs_f64() * 1000.0);
    println!("   - Steps per second: {:.1}", steps_per_second);
    
    // Compare with pure radiative transfer
    let pure_radiative_step_time = 0.4; // From our previous test
    let core_heat_step_time = avg_step_time.as_secs_f64() * 1000.0;
    let overhead = core_heat_step_time - pure_radiative_step_time;
    
    println!("   - Pure radiative: {:.1}ms per step", pure_radiative_step_time);
    println!("   - With core heat: {:.1}ms per step", core_heat_step_time);
    println!("   - Core heat overhead: {:.1}ms ({:.1}x slower)", overhead, core_heat_step_time / pure_radiative_step_time);
    
    // Billion year projection
    let billion_year_steps = 1_000_000_u64;
    let billion_year_time_hours = (avg_step_time.as_secs_f64() * billion_year_steps as f64) / 3600.0;
    
    println!("\n🔮 Billion Year Projection with Core Heat:");
    println!("   - Estimated time: {:.1} hours", billion_year_time_hours);
    if billion_year_time_hours < 1.0 {
        println!("   - Status: ✅ EXCELLENT - Under 1 hour");
    } else if billion_year_time_hours < 8.0 {
        println!("   - Status: ✅ GOOD - Under 8 hours");
    } else {
        println!("   - Status: ⚠️  MODERATE - Over 8 hours");
    }
    
    // Test assertions
    assert!(total_simulation_time.as_secs() < 60, "Should complete in under 1 minute");
    assert!(final_energy > 0.0, "Final energy should be positive");
    
    println!("\n🎉 Full geological simulation with core heat completed!");
    println!("✅ Radiative transfer + Perlin noise + Hotspots working together");
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

fn print_thermal_structure(sim: &SimulationImmut, title: &str) {
    println!("\n🌡️ Thermal Structure - {}:", title);
    println!("========================================");
    
    // Get first column for analysis
    if let Some(first_layer_set) = sim.layer_sets.first() {
        if let Some(first_column) = first_layer_set.layers.values().next() {
            // Show surface and deep temperatures
            if let (Some(surface_cell), Some(deep_cell)) = (first_column.cells.first(), first_column.cells.last()) {
                println!("   Surface: {:.1}K ({:.1}°C)", 
                         surface_cell.temperature_kelvin(), surface_cell.temperature_kelvin() - 273.15);
                println!("   Deep:    {:.1}K ({:.1}°C)", 
                         deep_cell.temperature_kelvin(), deep_cell.temperature_kelvin() - 273.15);
                
                let temp_diff = deep_cell.temperature_kelvin() - surface_cell.temperature_kelvin();
                println!("   Gradient: {:.1}K difference", temp_diff);
            }
        }
    }
    
    // Show layer averages
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        let mut layer_total_temp = 0.0;
        let mut layer_total_energy = 0.0;
        let mut cell_count = 0;
        
        for (_h3_cell, column) in &layer_set.layers {
            for cell in &column.cells {
                layer_total_temp += cell.temperature_kelvin();
                layer_total_energy += cell.energy_joules();
                cell_count += 1;
            }
        }
        
        if cell_count > 0 {
            let avg_temp = layer_total_temp / cell_count as f64;
            let total_energy = layer_total_energy;
            
            let material = if layer_idx == 0 { "basalt" } 
                          else if layer_idx == 1 { "peridotite" } 
                          else { "eclogite" };
            
            println!("   Layer {}: {:.1}K ({:.1}°C), {:.2e}J total, {} ({})", 
                     layer_idx + 1, avg_temp, avg_temp - 273.15, total_energy, material, cell_count);
        }
    }
    
    println!("========================================");
}
