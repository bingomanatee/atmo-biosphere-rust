use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🔍 Plume Spike Detection Test: Can Plumes See Energy Spikes?");
    println!("=============================================================");

    // Simple thermal config
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,
        surface_gradient_k_per_km: 30.0,
        deep_gradient_k_per_km: 5.0,
        reference_depth_km: 80.0,
    };

    // Simple 3-layer structure for focused testing
    let layer_params = vec![
        // Upper layer (0-50km)
        LayerSetParams {
            resolution: Resolution::Two,     // Coarse for speed
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
        // Mid layer (50-100km)
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
        // Deep layer (100-150km) - target for energy injection
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 100.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 1,                           // Just 1 step for focused test
        years_per_step: 1000.0,
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Helper function to check deep layer temperatures
    fn check_deep_layer_temperatures(sim: &Simulation, test_name: &str) {
        println!("\n🌡️ Deep Layer Temperature Check: {}", test_name);
        println!("================================================");
        
        let deep_layer_index = sim.layer_sets.len() - 1;
        if let Some(deep_layer) = sim.layer_sets.get(deep_layer_index) {
            let mut cell_count = 0;
            let mut total_temp = 0.0;
            let mut max_temp = 0.0f64;
            let mut min_temp = f64::INFINITY;
            
            for column in deep_layer.layers.values() {
                for cell in &column.cells {
                    let temp = cell.temperature_kelvin();
                    total_temp += temp;
                    max_temp = max_temp.max(temp);
                    min_temp = min_temp.min(temp);
                    cell_count += 1;
                    
                    if cell_count <= 5 {  // Show first 5 cells
                        println!("   Cell {}: {:.1}K ({:.1}°C) | Energy: {:.2e}J", 
                            cell_count, temp, temp - 273.15, cell.energy_joules());
                    }
                }
            }
            
            let avg_temp = total_temp / cell_count as f64;
            println!("   Summary: {} cells | Avg: {:.1}K | Min: {:.1}K | Max: {:.1}K", 
                cell_count, avg_temp, min_temp, max_temp);
            
            if max_temp > 1800.0 {
                println!("   ✅ Temperatures above plume threshold (1800K)!");
                let temp_excess = max_temp - 1800.0;
                let exp_factor = (temp_excess / 50.0f64).exp();
                println!("   📈 Max temperature excess: {:.1}K → Exponential factor: {:.2e}", 
                    temp_excess, exp_factor);
            } else {
                println!("   ❌ No temperatures above plume threshold (1800K)");
            }
        }
    }

    // Test 1: Check baseline temperatures
    println!("\n🌡️ Test 1: Baseline Temperatures (no energy injection)");
    println!("=======================================================");

    let mut components_baseline: Vec<Box<dyn SimComponent>> = vec![];
    let sim_baseline = Simulation::new(config.clone(), &mut components_baseline);
    check_deep_layer_temperatures(&sim_baseline, "Baseline");

    // Test 2: Check temperatures with VERY HIGH energy injection
    println!("\n🔥 Test 2: High Energy Injection + Plume Detection");
    println!("==================================================");

    let mut components_high_energy: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(1e21)        // VERY high energy injection
            .with_noise_amplitude(0.0)),   // No variation for consistency
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-8, 0.5)      // Higher probability for detection
            .with_temperature_threshold(1800.0)),
    ];
    let mut sim_high_energy = Simulation::new(config.clone(), &mut components_high_energy);
    sim_high_energy.initialize();

    println!("Before step: checking initial temperatures...");
    check_deep_layer_temperatures(&sim_high_energy, "Before Energy Injection");

    println!("\nExecuting simulation step (core radiance + plume detection)...");
    sim_high_energy.step();

    println!("After step: checking final temperatures...");
    check_deep_layer_temperatures(&sim_high_energy, "After Energy Injection");

    println!("\n🔍 Analysis: Component Execution Order");
    println!("======================================");
    println!("   Look at the output above for:");
    println!("   1. 🔥 Core Radiance messages (energy injection)");
    println!("   2. 🌋 Convection Plume messages (plume detection)");
    println!("   3. 🌡️ Temperature changes in deep layer");

    println!("\n🎯 Expected Behavior:");
    println!("   1. Core radiance should inject massive energy");
    println!("   2. Deep layer temperatures should spike to >5000K");
    println!("   3. Plume component should detect the spike");
    println!("   4. Exponential probability should trigger many plumes");
    println!("   5. Should see '🌋 Convection Plumes (Step 0): X active...' message");

    println!("\n🔬 Diagnostic Questions:");
    println!("   - Did core radiance inject energy? (look for 🔥 messages)");
    println!("   - Did temperatures spike? (compare before/after)");
    println!("   - Did plumes detect the spike? (look for 🌋 messages)");
    println!("   - What's the component execution order?");

    println!("\n✅ This test isolates the spike detection mechanism!");
}
