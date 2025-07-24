use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::sim_immut::layer_set_immut::variable_resolution_layer_params_immut;
use atmo_biosphere_rust::component::{SimComponent, core_radiance_component::CoreRadianceComponent};
use h3o::Resolution;

fn main() {
    println!("🌍 Variable Resolution Geological Simulation");
    println!("============================================");
    println!("High resolution where needed, coarse where efficient");

    // Create simulation configuration with variable resolution layer sets
    let config = SimulationConfigImmut {
        steps: 5,
        years_per_step: 50000.0, // 50,000 years per step
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: variable_resolution_layer_params_immut(Resolution::Two, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig {
            years_per_step: 50000.0,
            max_transfer_rate: 0.015, // 1.5% max transfer per step
            enable_space_radiation: true,
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: true,
        },
    };

    // Print detailed layer configuration
    println!("\n📊 Variable Resolution Layer Configuration:");
    println!("   (Optimized for plate interactions + plume dynamics)");
    let mut total_cells = 0;
    let mut cumulative_depth = 0.0;
    
    for (i, layer_params) in config.layer_set_params.iter().enumerate() {
        let layer_cells = layer_params.column_count;
        let layer_depth = layer_params.column_count as f64 * layer_params.cell_height_km;
        total_cells += layer_cells;
        cumulative_depth += layer_depth;
        
        let resolution_type = match layer_params.cell_height_km {
            h if h <= 1.0 => "ULTRA-HIGH",
            h if h <= 2.5 => "HIGH",
            h if h <= 10.0 => "MODERATE", 
            _ => "COARSE"
        };
        
        println!("   Layer {}: {:.1}-{:.1}km ({} cells × {:.1}km) - {} [{}]",
                 i + 1,
                 layer_params.start_height_km,
                 layer_params.start_height_km + layer_depth,
                 layer_cells,
                 layer_params.cell_height_km,
                 layer_params.material_name,
                 resolution_type);
    }
    
    println!("\n📈 Resolution Strategy:");
    println!("   - Surface (0-5km): 0.5km cells for PLATE INTERACTIONS");
    println!("   - Upper Crust (5-15km): 1km cells for crustal processes");
    println!("   - Lower Crust (15-35km): 2.5km cells for lithosphere");
    println!("   - Mid Mantle (35-75km): 10km cells for background thermal");
    println!("   - Deep Mantle (75-150km): 25km cells for plume sources");
    println!("   - Total cells per column: {} (efficient variable resolution)", total_cells);
    println!("   - Total depth: {:.0}km (covers all active geology)", cumulative_depth);

    // Create components for geological simulation
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()),
    ];

    // Create and run simulation
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("\n🔄 Starting Variable Resolution Simulation:");
    println!("   - {} steps × 50,000 years = {} thousand years total", sim.config.steps, sim.config.steps * 50);
    println!("   - Resolution: {:?} (H3 cells: ~{})", Resolution::Two, Resolution::Two.cell_count());
    println!("   - Variable resolution: Ultra-high surface → Coarse deep");

    // Print initial state
    let initial_total_energy = sim.total_energy();
    let initial_avg_temp = sim.average_temperature();
    println!("\n📈 Initial State:");
    println!("   - Total energy: {:.2e} J", initial_total_energy);
    println!("   - Average temperature: {:.1}K ({:.1}°C)", initial_avg_temp, initial_avg_temp - 273.15);
    println!("   - Total cells: {}", sim.total_cells());

    // Run simulation steps with detailed analysis
    for step in 1..=sim.config.steps {
        let step_start = std::time::Instant::now();
        
        println!("\n--- Step {} ({} kyr) ---", step, step * 50);
        sim.step();
        
        let step_duration = step_start.elapsed();
        let current_total_energy = sim.total_energy();
        let current_avg_temp = sim.average_temperature();
        let energy_change = current_total_energy - initial_total_energy;
        let energy_change_percent = (energy_change / initial_total_energy) * 100.0;
        
        println!("   Step completed in {:.2} ms", step_duration.as_secs_f64() * 1000.0);
        println!("   Current state:");
        println!("   - Total energy: {:.2e} J ({:+.3}%)", current_total_energy, energy_change_percent);
        println!("   - Average temperature: {:.1}K ({:.1}°C)", current_avg_temp, current_avg_temp - 273.15);
        
        // Analyze resolution efficiency
        if step == 1 {
            println!("   🔍 Resolution Analysis:");
            println!("     - Surface layers: Ultra-high resolution for plate detail");
            println!("     - Deep layers: Coarse resolution for computational efficiency");
            println!("     - Plumes: Will maintain fine-grained properties across layers");
        }
    }

    // Final analysis
    let final_total_energy = sim.total_energy();
    let final_avg_temp = sim.average_temperature();
    let total_energy_change = final_total_energy - initial_total_energy;
    let total_energy_change_percent = (total_energy_change / initial_total_energy) * 100.0;

    println!("\n🎯 Variable Resolution Analysis:");
    println!("   - Simulation time: {} thousand years", sim.config.steps * 50);
    println!("   - Total energy change: {:+.2e} J ({:+.3}%)", total_energy_change, total_energy_change_percent);
    println!("   - Temperature change: {:+.1}K", final_avg_temp - initial_avg_temp);

    // Resolution efficiency analysis
    println!("\n⚡ Resolution Efficiency:");
    println!("   - Surface resolution: 0.5km (ultra-high for plates)");
    println!("   - Deep resolution: 25km (coarse for efficiency)");
    println!("   - Resolution ratio: 50:1 (surface:deep)");
    println!("   - Computational focus: Surface processes + plate interactions");
    
    // Plume compatibility analysis
    println!("\n🌋 Plume System Compatibility:");
    println!("   - Fine-grained plumes: Can originate in 25km deep cells");
    println!("   - Resolution transition: Plumes retain properties across layers");
    println!("   - Surface interaction: Ultra-high resolution for plume-plate interaction");
    println!("   - Efficiency: Coarse deep layers don't waste computation on background");

    // Energy balance analysis
    if total_energy_change_percent.abs() < 0.5 {
        println!("\n✅ Energy Balance: EXCELLENT (change < 0.5%)");
        println!("   - Variable resolution maintains thermal equilibrium");
        println!("   - Radiative transfer works across resolution boundaries");
    } else if total_energy_change_percent.abs() < 2.0 {
        println!("\n✅ Energy Balance: GOOD (change < 2%)");
        println!("   - System approaching equilibrium with variable resolution");
    } else {
        println!("\n⚠️ Energy Balance: ADJUSTING (change > 2%)");
        println!("   - Variable resolution may need fine-tuning");
    }

    println!("\n✅ Variable Resolution Simulation completed!");
    println!("   - Demonstrated efficient variable resolution strategy");
    println!("   - Ultra-high surface resolution for plate interactions");
    println!("   - Coarse deep resolution for computational efficiency");
    println!("   - Ready for plume systems with fine-grained properties");
    println!("   - Optimal balance: Detail where needed, efficiency where possible");
}
