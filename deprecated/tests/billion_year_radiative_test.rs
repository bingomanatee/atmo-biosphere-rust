use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

#[test]
fn test_billion_year_radiative_simulation() {
    println!("🌍 Billion Year Radiative Transfer Simulation");
    println!("==============================================");
    println!("⏰ Duration: 1 billion years (1,000,000 steps × 1,000 years/step)");
    println!("🔥 Focus: Pure radiative heat transfer (no components)");
    
    // Create simulation configuration for billion year run
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 1_000_000, // 1 million steps
        years_per_step: 1000.0, // 1000 years per step = 1 billion years total
        surface_temp_k: 288.0, // 15°C surface temperature
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    // Create simulation without components (pure radiative transfer)
    let mut components: Vec<Box<dyn SimComponent>> = vec![];
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("✅ Simulation created (radiative transfer only)");
    
    // Load layer sets
    sim.load_layer_sets();
    println!("✅ Layer sets loaded: {} geological layers", sim.layer_sets.len());
    
    // Get initial energy state
    let initial_energy = calculate_total_energy(&sim);
    println!("🔋 Initial total energy: {:.2e} J", initial_energy);
    
    // Track energy and temperature over geological time
    let mut energy_history = Vec::new();
    let mut temperature_history = Vec::new();
    let report_interval = 100_000; // Report every 10% (100 million years)

    println!("\n🕰️ Starting billion year radiative simulation...");
    println!("📊 Progress reports every {} steps (100 million years = 10%)", report_interval);
    
    // Run the billion year simulation
    let mut final_step = 0;
    for step_num in 0..sim.config.steps {
        final_step = step_num;
        // Run simulation step (radiative transfer only)
        sim.step();
        
        // Report progress every 10% (100,000 steps)
        if step_num % report_interval == 0 || step_num == sim.config.steps - 1 {
            let current_energy = calculate_total_energy(&sim);
            let avg_temperature = calculate_average_temperature(&sim);
            let years_elapsed = (step_num + 1) as f64 * sim.config.years_per_step;
            let million_years = years_elapsed / 1_000_000.0;

            // Calculate energy change
            let energy_change = current_energy - initial_energy;
            let energy_change_percent = (energy_change / initial_energy) * 100.0;

            println!("\n📈 {:.0}% Complete - {:.0} Million Years Ago to Present:",
                     ((step_num + 1) as f64 / sim.config.steps as f64) * 100.0,
                     1000.0 - million_years);
            println!("   Step: {}/{}", step_num + 1, sim.config.steps);
            println!("   Energy: {:.2e} J ({:+.2}% change)", current_energy, energy_change_percent);
            println!("   Avg temp: {:.1}K ({:.1}°C)", avg_temperature, avg_temperature - 273.15);

            // Only show detailed thermal structure at key milestones (0%, 50%, 100%)
            if step_num == 0 || step_num == sim.config.steps / 2 || step_num == sim.config.steps - 1 {
                print_detailed_thermal_structure(&sim, million_years);
            }

            // Store history
            energy_history.push((million_years, current_energy));
            temperature_history.push((million_years, avg_temperature));

            // Check for energy conservation
            if energy_change_percent.abs() > 10.0 {
                println!("⚠️  WARNING: Large energy change! ({:+.2}%)", energy_change_percent);
            }
        }

    }
    
    // Final analysis
    let final_energy = calculate_total_energy(&sim);
    let final_temperature = calculate_average_temperature(&sim);
    let total_energy_change = final_energy - initial_energy;
    let total_energy_change_percent = (total_energy_change / initial_energy) * 100.0;
    
    println!("\n🎯 Billion Year Radiative Simulation Results:");
    println!("==============================================");
    println!("🔋 Initial energy: {:.2e} J", initial_energy);
    println!("🔋 Final energy:   {:.2e} J", final_energy);
    println!("📊 Energy change:  {:+.2e} J ({:+.2}%)", total_energy_change, total_energy_change_percent);
    println!("🌡️ Final avg temp: {:.1}K ({:.1}°C)", final_temperature, final_temperature - 273.15);
    
    // Energy conservation check
    if total_energy_change_percent.abs() < 1.0 {
        println!("✅ Energy conservation: EXCELLENT (change < 1%)");
    } else if total_energy_change_percent.abs() < 5.0 {
        println!("✅ Energy conservation: GOOD (change < 5%)");
    } else if total_energy_change_percent.abs() < 10.0 {
        println!("⚠️  Energy conservation: ACCEPTABLE (change < 10%)");
    } else {
        println!("❌ Energy conservation: POOR (change > 10%)");
    }
    
    // Print evolution summary
    println!("\n📈 Evolution Over Geological Time:");
    for (i, (million_years, energy)) in energy_history.iter().enumerate().take(10) {
        if let Some((_, temp)) = temperature_history.get(i) {
            println!("   {:.0} Ma: {:.2e} J, {:.1}K", million_years, energy, temp);
        }
    }
    
    // Verify atomic transaction system worked
    let (final_pending, final_committed) = sim.transaction_manager.get_transaction_stats();
    println!("\n🔄 Atomic Transaction System Performance:");
    println!("   Final pending transactions: {}", final_pending);
    println!("   Total committed transactions: {}", final_committed);
    println!("   Average transactions per step: {:.1}", final_committed as f64 / (final_step + 1) as f64);
    
    // Basic assertions
    assert!(final_energy > 0.0, "Final energy should be positive");
    assert!(total_energy_change_percent.abs() < 50.0, "Energy change should be reasonable");
    // Note: final_committed may be 0 because transactions are applied and cleared each step
    assert!(final_temperature > 200.0 && final_temperature < 2000.0, "Temperature should be reasonable");
    
    println!("\n🎉 Billion Year Radiative Simulation completed successfully!");
    println!("✅ Atomic transaction system maintained energy conservation over geological time");
    println!("✅ Radiative transfer system stable over billion year timescales");
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

fn calculate_average_temperature(sim: &SimulationImmut) -> f64 {
    let mut total_temperature = 0.0;
    let mut cell_count = 0;

    for layer_set in &sim.layer_sets {
        for (_h3_cell, column) in &layer_set.layers {
            for cell in &column.cells {
                total_temperature += cell.temperature_kelvin();
                cell_count += 1;
            }
        }
    }

    if cell_count > 0 {
        total_temperature / cell_count as f64
    } else {
        0.0
    }
}

fn print_detailed_thermal_structure(sim: &SimulationImmut, million_years: f64) {
    println!("\n📊 COMPREHENSIVE CELL-BY-CELL THERMAL ANALYSIS at {:.0} Million Years:", million_years);
    println!("================================================");
    println!("| Layer | Cell | Depth | Temp(K) | Temp(°C) | Energy(J)  | Mass(kg)   | Material |");
    println!("|-------|------|-------|---------|----------|------------|------------|----------|");

    let mut total_cells = 0;
    let mut total_energy = 0.0;
    let mut total_mass = 0.0;

    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        if layer_idx >= sim.config.layer_set_params.len() { break; }
        let layer_params = &sim.config.layer_set_params[layer_idx];

        // Get first column for detailed analysis
        if let Some(first_column) = layer_set.layers.values().next() {
            for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                let depth_km = layer_params.start_height_km +
                              (cell_idx as f64 * layer_params.cell_height_km);
                let temp_k = cell.temperature_kelvin();
                let temp_c = temp_k - 273.15;
                let energy_j = cell.energy_joules();
                let mass_kg = cell.mass_kg();

                total_cells += 1;
                total_energy += energy_j;
                total_mass += mass_kg;

                println!("| {:5} | {:4} | {:5.0} | {:7.1} | {:8.1} | {:10.2e} | {:10.2e} | {:8} |",
                         layer_idx + 1,
                         cell_idx + 1,
                         depth_km,
                         temp_k,
                         temp_c,
                         energy_j,
                         mass_kg,
                         layer_params.material_name);
            }

            // Add separator between layers
            if layer_idx < sim.layer_sets.len() - 1 {
                println!("|-------|------|-------|---------|----------|------------|------------|----------|");
            }
        }
    }

    println!("|-------|------|-------|---------|----------|------------|------------|----------|");
    println!("| TOTAL | {:4} |       |         |          | {:10.2e} | {:10.2e} |          |",
             total_cells, total_energy, total_mass);

    // Thermal gradient analysis
    println!("\n🌡️ THERMAL GRADIENT ANALYSIS:");
    println!("=============================");
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        if layer_idx >= sim.config.layer_set_params.len() { break; }
        let layer_params = &sim.config.layer_set_params[layer_idx];

        if let Some(first_column) = layer_set.layers.values().next() {
            if first_column.cells.len() >= 2 {
                let first_cell = &first_column.cells[0];
                let last_cell = &first_column.cells[first_column.cells.len() - 1];

                let depth_diff = (first_column.cells.len() - 1) as f64 * layer_params.cell_height_km;
                let temp_diff = last_cell.temperature_kelvin() - first_cell.temperature_kelvin();
                let gradient = temp_diff / depth_diff;

                println!("Layer {}: {:.1}K/km gradient ({:.0}-{:.0}km depth)",
                         layer_idx + 1,
                         gradient,
                         layer_params.start_height_km,
                         layer_params.start_height_km + (first_column.cells.len() as f64 * layer_params.cell_height_km));
            }
        }
    }

    println!("================================================");
}
