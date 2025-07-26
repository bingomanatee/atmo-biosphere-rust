use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;

fn calculate_total_energy(sim: &SimulationImmut) -> f64 {
    sim.layer_sets.iter()
        .flat_map(|layer_set| layer_set.layers.values())
        .flat_map(|column| &column.cells)
        .map(|cell| cell.energy_joules())
        .sum()
}

fn main() {
    println!("🔋 Simple Energy Conservation Monitor");
    println!("====================================");
    println!("Testing radiative transfer for energy conservation violations");

    // Create simulation configuration with radiative transfer
    let config = SimulationConfigImmut {
        steps: 5, // Run 5 steps to see energy trends
        years_per_step: 10000.0, // 10,000 years per step
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: default_layer_set_params_immut(Resolution::Two, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig {
            years_per_step: 10000.0,
            max_transfer_rate: 0.01, // 1% max transfer per step
            enable_space_radiation: true,  // Enable space cooling
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: true,
        },
    };

    // Create components (no additional energy sources)
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        // No components - testing pure radiative transfer
    ];

    // Create immutable simulation
    let mut sim = SimulationImmut::new(config, &mut components);

    println!("\n🌍 Starting energy-monitored simulation...");
    println!("Configuration:");
    println!("   - Steps: {}", sim.config.steps);
    println!("   - Years per step: {}", sim.config.years_per_step);
    println!("   - Space radiation: {}", sim.config.radiative_transfer_config.enable_space_radiation);
    println!("   - Layer structure: 5+5+5 cells (15 total, 165km depth)");

    // Track energy through simulation
    let mut energy_history = Vec::new();
    
    // Record initial energy
    let initial_energy = calculate_total_energy(&sim);
    energy_history.push((0, initial_energy));
    println!("\n🔋 Initial system energy: {:.6e} J", initial_energy);

    // Run simulation steps with energy monitoring
    for step in 0..sim.config.steps {
        println!("\n--- Step {} ---", step + 1);
        
        sim.step();
        
        // Calculate total energy after step
        let total_energy = calculate_total_energy(&sim);
        energy_history.push((step + 1, total_energy));
        
        // Calculate energy change
        let energy_change = total_energy - initial_energy;
        let energy_change_percent = (energy_change / initial_energy) * 100.0;
        
        println!("📊 Step {}: Energy = {:.6e} J", step + 1, total_energy);
        println!("   Change from initial: {:.6e} J ({:.6}%)", energy_change, energy_change_percent);
        
        // Flag energy violations
        if energy_change > 0.0 {
            println!("🚨 ENERGY CREATION DETECTED! System gained {:.6e} J", energy_change);
        } else if energy_change < 0.0 {
            println!("❄️  Energy loss: {:.6e} J (expected from space radiation)", -energy_change);
        } else {
            println!("✅ Perfect energy conservation");
        }
        
        // Check step-to-step changes
        if step > 0 {
            let prev_energy = energy_history[step as usize].1;
            let step_change = total_energy - prev_energy;
            let step_change_percent = (step_change / prev_energy) * 100.0;
            println!("   Step-to-step change: {:.6e} J ({:.6}%)", step_change, step_change_percent);
        }
    }

    // Generate final energy conservation report
    println!("\n🔋 ENERGY CONSERVATION ANALYSIS REPORT");
    println!("=====================================");
    
    let final_energy = energy_history.last().unwrap().1;
    let total_change = final_energy - initial_energy;
    let total_change_percent = (total_change / initial_energy) * 100.0;
    
    println!("Initial Energy: {:.6e} J", initial_energy);
    println!("Final Energy:   {:.6e} J", final_energy);
    println!("Total Change:   {:.6e} J ({:.6}%)", total_change, total_change_percent);
    println!("Steps Monitored: {}", energy_history.len() - 1);
    
    // Energy conservation verdict
    if total_change > 1e-6 { // More than 1 microjoule increase
        println!("\n🚨 ENERGY CONSERVATION VIOLATION: System gained energy!");
        println!("   This indicates a bug in the radiative transfer or transaction system.");
        println!("   Energy should only decrease (space cooling) or stay constant.");
    } else if total_change < -1e-6 { // More than 1 microjoule decrease
        println!("\n❄️  Energy loss detected (expected from space radiation)");
        println!("   Energy lost to space: {:.6e} J", -total_change);
        println!("   This is normal behavior for radiative cooling.");
    } else {
        println!("\n✅ Perfect energy conservation (no significant change)");
        println!("   Energy change within numerical precision limits.");
    }
    
    // Step-by-step analysis
    println!("\n📈 Step-by-Step Energy Changes:");
    println!("Step | Energy (J)        | Change (J)       | Change (%)");
    println!("-----|-------------------|------------------|------------");
    
    for (i, (step, energy)) in energy_history.iter().enumerate() {
        if i == 0 {
            println!("{:4} | {:.6e} | {:16} | {:10}", step, energy, "baseline", "0.000000");
        } else {
            let prev_energy = energy_history[i-1].1;
            let change = energy - prev_energy;
            let change_percent = (change / prev_energy) * 100.0;
            println!("{:4} | {:.6e} | {:+.6e} | {:+.6}", step, energy, change, change_percent);
        }
    }
    
    // Energy stability analysis
    if energy_history.len() > 2 {
        let mut max_step_change: f64 = 0.0;
        let mut min_step_change: f64 = 0.0;
        
        for i in 1..energy_history.len() {
            let step_change = energy_history[i].1 - energy_history[i-1].1;
            max_step_change = max_step_change.max(step_change);
            min_step_change = min_step_change.min(step_change);
        }
        
        println!("\n📊 Energy Stability Analysis:");
        println!("Max step increase: {:.6e} J", max_step_change);
        println!("Max step decrease: {:.6e} J", min_step_change);
        
        if max_step_change > 0.0 {
            println!("⚠️  Energy increases detected - investigate radiative transfer");
        } else {
            println!("✅ No energy increases - radiative transfer working correctly");
        }
    }

    println!("\n🔬 Energy Conservation Test Complete!");
    println!("=====================================");
    println!("Expected behavior:");
    println!("  - Energy should decrease (space cooling) or stay constant");
    println!("  - Energy should NEVER increase");
    println!("  - Any energy increase indicates a bug in radiative transfer");
}
