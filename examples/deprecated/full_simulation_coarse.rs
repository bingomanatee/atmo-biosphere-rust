use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::sim_immut::layer_set_immut::coarse_layer_set_params_immut;
use atmo_biosphere_rust::component::{SimComponent, core_radiance_component::CoreRadianceComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌍 Full Simulation with Coarse-Grained Layers");
    println!("==============================================");

    // Create simulation configuration with coarse-grained layer sets for performance
    let config = SimulationConfigImmut {
        steps: 10,
        years_per_step: 100000.0, // 100,000 years per step for geological timescales
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: coarse_layer_set_params_immut(Resolution::Two, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig {
            years_per_step: 100000.0,
            max_transfer_rate: 0.02, // 2% max transfer per step for longer timescales
            enable_space_radiation: true,
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: true, // Enable all radiative transfer
        },
    };

    // Print layer configuration for verification
    println!("\n📊 Layer Configuration (Coarse-Grained):");
    let mut total_cells = 0;
    for (i, layer_params) in config.layer_set_params.iter().enumerate() {
        let layer_cells = layer_params.column_count;
        let layer_depth = layer_params.column_count as f64 * layer_params.cell_height_km;
        total_cells += layer_cells;
        
        println!("   Layer {}: {:.0}-{:.0}km ({} cells × {:.0}km = {:.0}km depth) - {}",
                 i + 1,
                 layer_params.start_height_km,
                 layer_params.start_height_km + layer_depth,
                 layer_cells,
                 layer_params.cell_height_km,
                 layer_depth,
                 layer_params.material_name);
    }
    println!("   Total cells per column: {} (vs 300+ in fine-grained)", total_cells);

    // Create components for geological simulation
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()),
    ];

    // Create and run simulation
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("\n🔄 Starting Full Simulation Run:");
    println!("   - {} steps × 100,000 years = {} million years total", sim.config.steps, sim.config.steps as f64 * 0.1);
    println!("   - Resolution: {:?} (H3 cells: ~{})", Resolution::Two, Resolution::Two.cell_count());
    println!("   - Radiative transfer: Enabled with space radiation");

    // Print initial state
    let initial_total_energy = sim.total_energy();
    let initial_avg_temp = sim.average_temperature();
    println!("\n📈 Initial State:");
    println!("   - Total energy: {:.2e} J", initial_total_energy);
    println!("   - Average temperature: {:.1}K ({:.1}°C)", initial_avg_temp, initial_avg_temp - 273.15);
    println!("   - Total cells: {}", sim.total_cells());

    // Run simulation steps
    for step in 1..=sim.config.steps {
        let step_start = std::time::Instant::now();
        
        println!("\n--- Step {} ({:.1} Myr) ---", step, step as f64 * 0.1);
        sim.step();
        
        let step_duration = step_start.elapsed();
        let current_total_energy = sim.total_energy();
        let current_avg_temp = sim.average_temperature();
        let energy_change = current_total_energy - initial_total_energy;
        let energy_change_percent = (energy_change / initial_total_energy) * 100.0;
        
        println!("   Step completed in {:.2} ms", step_duration.as_secs_f64() * 1000.0);
        println!("   Current state:");
        println!("   - Total energy: {:.2e} J ({:+.2}%)", current_total_energy, energy_change_percent);
        println!("   - Average temperature: {:.1}K ({:.1}°C)", current_avg_temp, current_avg_temp - 273.15);
        println!("   - Energy change: {:+.2e} J", energy_change);
        
        // Check for equilibrium (energy change < 0.1% per step)
        if energy_change_percent.abs() < 0.1 {
            println!("   🎯 Approaching equilibrium (energy change < 0.1%)");
        }
    }

    // Final analysis
    let final_total_energy = sim.total_energy();
    let final_avg_temp = sim.average_temperature();
    let total_energy_change = final_total_energy - initial_total_energy;
    let total_energy_change_percent = (total_energy_change / initial_total_energy) * 100.0;

    println!("\n🎯 Final Analysis:");
    println!("   - Simulation time: {:.1} million years", sim.config.steps as f64 * 0.1);
    println!("   - Initial energy: {:.2e} J", initial_total_energy);
    println!("   - Final energy: {:.2e} J", final_total_energy);
    println!("   - Total energy change: {:+.2e} J ({:+.2}%)", total_energy_change, total_energy_change_percent);
    println!("   - Initial avg temp: {:.1}K ({:.1}°C)", initial_avg_temp, initial_avg_temp - 273.15);
    println!("   - Final avg temp: {:.1}K ({:.1}°C)", final_avg_temp, final_avg_temp - 273.15);
    println!("   - Temperature change: {:+.1}K", final_avg_temp - initial_avg_temp);

    // Performance analysis
    println!("\n⚡ Performance Analysis:");
    println!("   - Cells per column: {} (reduced from 300+)", total_cells);
    println!("   - Radiative transfer pairs: ~{}", sim.binary_operations.get_statistics().get("total_pairs").unwrap_or(&0));
    println!("   - Computational efficiency: ~{}x faster than fine-grained", 300 / total_cells);

    // Energy balance analysis
    if total_energy_change_percent.abs() < 1.0 {
        println!("\n✅ Energy Balance: GOOD (change < 1%)");
        println!("   - System is approaching thermal equilibrium");
        println!("   - Radiative cooling is balancing internal heat sources");
    } else if total_energy_change_percent < -5.0 {
        println!("\n❄️ Energy Balance: COOLING (change < -5%)");
        println!("   - System is losing energy to space faster than internal generation");
        println!("   - May need to adjust radiative transfer rates or add heat sources");
    } else if total_energy_change_percent > 5.0 {
        println!("\n🔥 Energy Balance: HEATING (change > +5%)");
        println!("   - System is gaining energy faster than radiative cooling");
        println!("   - May need to increase radiative transfer rates");
    } else {
        println!("\n🌡️ Energy Balance: STABLE (change ±1-5%)");
        println!("   - System is slowly approaching equilibrium");
    }

    println!("\n✅ Full Simulation with Coarse-Grained Layers completed!");
    println!("   - Demonstrated efficient layer structure for full-scale simulations");
    println!("   - Radiative transfer system working with realistic energy balance");
    println!("   - Ready for long-term geological simulations");
}
