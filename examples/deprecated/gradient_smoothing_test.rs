use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;

fn analyze_temperature_gradients(sim: &SimulationImmut, step_name: &str) {
    println!("\n📊 Temperature Gradient Analysis - {}", step_name);
    println!("=================================================");
    
    // Get first column for analysis
    let mut all_temps = Vec::new();
    let mut layer_temps = Vec::new();
    
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        let layer_params = &sim.config.layer_set_params[layer_idx];
        
        if let Some(first_column) = layer_set.layers.values().next() {
            let mut layer_cell_temps = Vec::new();
            
            for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                let depth_km = layer_params.start_height_km + (cell_idx as f64 * layer_params.cell_height_km);
                let temp_k = cell.temperature_kelvin();
                let temp_c = temp_k - 273.15;
                
                all_temps.push((depth_km, temp_k));
                layer_cell_temps.push(temp_k);
                
                println!("Layer {} Cell {}: {:.0}km depth, {:.1}K ({:.1}°C)", 
                         layer_idx + 1, cell_idx + 1, depth_km, temp_k, temp_c);
            }
            
            // Calculate layer statistics
            let min_temp = layer_cell_temps.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_temp = layer_cell_temps.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let temp_range = max_temp - min_temp;
            
            layer_temps.push((layer_idx + 1, min_temp, max_temp, temp_range));
            
            println!("  Layer {} range: {:.1}K to {:.1}K (span: {:.1}K)", 
                     layer_idx + 1, min_temp, max_temp, temp_range);
        }
    }
    
    // Overall gradient analysis
    if all_temps.len() > 1 {
        let min_temp = all_temps.iter().map(|(_, t)| *t).fold(f64::INFINITY, f64::min);
        let max_temp = all_temps.iter().map(|(_, t)| *t).fold(f64::NEG_INFINITY, f64::max);
        let total_range = max_temp - min_temp;
        
        // Calculate steepest gradient between adjacent cells
        let mut max_gradient = 0.0;
        let mut max_gradient_location = String::new();
        
        for i in 1..all_temps.len() {
            let (depth1, temp1) = all_temps[i-1];
            let (depth2, temp2) = all_temps[i];
            let depth_diff = depth2 - depth1;
            let temp_diff = temp2 - temp1;
            let gradient = temp_diff.abs() / depth_diff; // K/km
            
            if gradient > max_gradient {
                max_gradient = gradient;
                max_gradient_location = format!("{:.0}km to {:.0}km", depth1, depth2);
            }
        }
        
        println!("\n🌡️ Overall Temperature Statistics:");
        println!("   Global range: {:.1}K to {:.1}K (total span: {:.1}K)", min_temp, max_temp, total_range);
        println!("   Steepest gradient: {:.1}K/km between {}", max_gradient, max_gradient_location);
        
        // Return key metrics for comparison
        (total_range, max_gradient)
    } else {
        (0.0, 0.0)
    }
}

fn main() {
    println!("🌡️ Temperature Gradient Smoothing Test");
    println!("======================================");
    println!("Testing that radiative transfer smooths gradients rather than creating extremes");

    // Create simulation configuration with radiative transfer
    let config = SimulationConfigImmut {
        steps: 3, // Run 3 steps to see gradient evolution
        years_per_step: 10000.0, // 10,000 years per step
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: default_layer_set_params_immut(Resolution::Two, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig {
            years_per_step: 10000.0,
            max_transfer_rate: 0.01, // 1% max transfer per step
            enable_space_radiation: true,
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: true,
        },
    };

    // Create components (no additional energy sources)
    let mut components: Vec<Box<dyn SimComponent>> = vec![];

    // Create immutable simulation
    let mut sim = SimulationImmut::new(config, &mut components);

    println!("\n🌍 Starting gradient smoothing test...");
    println!("Configuration:");
    println!("   - Steps: {}", sim.config.steps);
    println!("   - Radiative transfer enabled");
    println!("   - Layer structure: 5+5+5 cells (15 total, 165km depth)");

    // Analyze initial gradients
    let (initial_range, initial_max_gradient) = analyze_temperature_gradients(&sim, "Initial State");
    
    let mut gradient_history = vec![(0, initial_range, initial_max_gradient)];

    // Run simulation steps and track gradient changes
    for step in 0..sim.config.steps {
        println!("\n🔄 Running Step {}...", step + 1);
        
        sim.step();
        
        let step_name = format!("After Step {}", step + 1);
        let (range, max_gradient) = analyze_temperature_gradients(&sim, &step_name);
        gradient_history.push((step + 1, range, max_gradient));
    }

    // Gradient smoothing analysis
    println!("\n🔍 GRADIENT SMOOTHING ANALYSIS");
    println!("==============================");
    
    println!("Step | Total Range (K) | Max Gradient (K/km) | Range Change | Gradient Change");
    println!("-----|-----------------|---------------------|--------------|----------------");
    
    for (i, (step, range, gradient)) in gradient_history.iter().enumerate() {
        if i == 0 {
            println!("{:4} | {:15.1} | {:19.1} | {:12} | {:14}", 
                     step, range, gradient, "baseline", "baseline");
        } else {
            let prev_range = gradient_history[i-1].1;
            let prev_gradient = gradient_history[i-1].2;
            let range_change = range - prev_range;
            let gradient_change = gradient - prev_gradient;
            
            println!("{:4} | {:15.1} | {:19.1} | {:+12.1} | {:+14.1}", 
                     step, range, gradient, range_change, gradient_change);
        }
    }
    
    // Final analysis
    let final_range = gradient_history.last().unwrap().1;
    let final_gradient = gradient_history.last().unwrap().2;
    let range_change = final_range - initial_range;
    let gradient_change = final_gradient - initial_max_gradient;
    
    println!("\n📈 GRADIENT SMOOTHING VERDICT");
    println!("=============================");
    
    // Temperature range analysis
    if range_change < -0.1 {
        println!("✅ Temperature range DECREASED by {:.1}K (smoothing working)", -range_change);
    } else if range_change > 0.1 {
        println!("🚨 Temperature range INCREASED by {:.1}K (creating extremes!)", range_change);
        println!("   This indicates radiative transfer is creating artificial temperature extremes");
    } else {
        println!("➡️  Temperature range unchanged ({:.1}K change)", range_change);
    }
    
    // Gradient steepness analysis
    if gradient_change < -0.1 {
        println!("✅ Maximum gradient DECREASED by {:.1}K/km (smoothing working)", -gradient_change);
    } else if gradient_change > 0.1 {
        println!("🚨 Maximum gradient INCREASED by {:.1}K/km (creating steeper gradients!)", gradient_change);
        println!("   This indicates radiative transfer is creating artificial steep gradients");
    } else {
        println!("➡️  Maximum gradient unchanged ({:.1}K/km change)", gradient_change);
    }
    
    // Overall verdict
    println!("\n🎯 OVERALL ASSESSMENT:");
    if range_change <= 0.0 && gradient_change <= 0.0 {
        println!("✅ RADIATIVE TRANSFER WORKING CORRECTLY");
        println!("   - Smoothing temperature gradients as expected");
        println!("   - No artificial temperature extremes created");
        println!("   - Energy redistribution is physically realistic");
    } else if range_change > 0.0 || gradient_change > 0.0 {
        println!("🚨 RADIATIVE TRANSFER CREATING ARTIFICIAL EXTREMES");
        println!("   - Temperature gradients becoming steeper");
        println!("   - This violates physical principles");
        println!("   - Check radiative transfer algorithm for bugs");
    } else {
        println!("➡️  RADIATIVE TRANSFER NEUTRAL");
        println!("   - No significant gradient changes");
        println!("   - May need more steps or higher transfer rates to see smoothing");
    }

    println!("\n🔬 Gradient Smoothing Test Complete!");
    println!("====================================");
    println!("Expected behavior:");
    println!("  - Temperature ranges should decrease or stay same");
    println!("  - Gradients should become gentler or stay same");
    println!("  - No artificial temperature extremes should be created");
}
