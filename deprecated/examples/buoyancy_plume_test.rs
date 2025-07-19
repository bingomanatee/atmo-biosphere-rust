use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Buoyancy-Driven Plume Formation Test");
    println!("=======================================");

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

    // Helper function to check densities and buoyancy conditions
    fn check_buoyancy_conditions(sim: &Simulation, test_name: &str) {
        println!("\n🔍 Buoyancy Analysis: {}", test_name);
        println!("=====================================");
        
        for layer_idx in 1..sim.layer_sets.len() {  // Skip first layer (no layer above)
            let lower_layer = &sim.layer_sets[layer_idx];
            let upper_layer = &sim.layer_sets[layer_idx - 1];
            
            println!("   Layer {} vs Layer {} comparison:", layer_idx, layer_idx - 1);
            
            // Get first column from each layer
            if let (Some(lower_column), Some(upper_column)) = 
                (lower_layer.layers.values().next(), upper_layer.layers.values().next()) {
                
                if let (Some(lower_cell), Some(upper_cell)) = 
                    (lower_column.cells.first(), upper_column.cells.first()) {
                    
                    let lower_temp = lower_cell.temperature_kelvin();
                    let upper_temp = upper_cell.temperature_kelvin();
                    let temp_diff = lower_temp - upper_temp;
                    
                    let lower_volume_km3 = lower_cell.area() * lower_cell.height_km;
                    let upper_volume_km3 = upper_cell.area() * upper_cell.height_km;
                    
                    let lower_density = lower_cell.mass_kg() / (lower_volume_km3 * 1e9);
                    let upper_density = upper_cell.mass_kg() / (upper_volume_km3 * 1e9);
                    let density_diff = upper_density - lower_density; // Positive = buoyancy instability
                    
                    println!("      Lower cell: {:.1}K, {:.1} kg/m³", lower_temp, lower_density);
                    println!("      Upper cell: {:.1}K, {:.1} kg/m³", upper_temp, upper_density);
                    println!("      Temperature difference: {:.1}K", temp_diff);
                    println!("      Density difference: {:.1} kg/m³", density_diff);
                    
                    if density_diff > 0.0 && temp_diff > 50.0 {
                        let buoyancy_force = 9.81 * density_diff;
                        println!("      ✅ BUOYANCY INSTABILITY! Force: {:.1} N/m³", buoyancy_force);
                        println!("      🌋 Should trigger plume formation!");
                    } else if density_diff <= 0.0 {
                        println!("      ❌ No buoyancy (lower cell denser than upper)");
                    } else {
                        println!("      ⚠️  Insufficient temperature difference ({:.1}K < 50K)", temp_diff);
                    }
                }
            }
            println!();
        }
    }

    // Test 1: Check baseline buoyancy conditions
    println!("\n🌡️ Test 1: Baseline Buoyancy Conditions");
    println!("========================================");

    let mut components_baseline: Vec<Box<dyn SimComponent>> = vec![];
    let sim_baseline = Simulation::new(config.clone(), &mut components_baseline);
    check_buoyancy_conditions(&sim_baseline, "Baseline");

    // Test 2: Energy injection + buoyancy-driven plume formation
    println!("\n🔥 Test 2: Energy Injection + Buoyancy Plume Formation");
    println!("======================================================");

    let mut components_buoyancy: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(5e20)        // High energy injection
            .with_noise_amplitude(0.0)),   // No variation for consistency
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-6, 0.5)      // Higher probability for detection
            .with_temperature_threshold(1000.0)), // Lower threshold for easier triggering
    ];
    let mut sim_buoyancy = Simulation::new(config.clone(), &mut components_buoyancy);
    sim_buoyancy.initialize();

    println!("Before energy injection:");
    check_buoyancy_conditions(&sim_buoyancy, "Before Injection");

    println!("Executing simulation step (energy injection + buoyancy plume detection)...");
    sim_buoyancy.step();

    println!("After energy injection:");
    check_buoyancy_conditions(&sim_buoyancy, "After Injection");

    println!("\n🎯 Expected Behavior:");
    println!("   1. 🔥 Core radiance heats deep layer → thermal expansion → lower density");
    println!("   2. 🏋️  Density inversion: hot deep layer becomes less dense than cool upper layer");
    println!("   3. ⬆️  Buoyancy force: density difference × gravity creates upward force");
    println!("   4. 🌋 Plume formation: buoyancy instability triggers plume generation");
    println!("   5. 📊 Look for '🌋 Buoyancy Plume #X created...' messages above");

    println!("\n🔬 Key Physics:");
    println!("   - Thermal expansion: ρ = ρ₀ × (1 - α × ΔT)");
    println!("   - Buoyancy force: F = g × (ρ_upper - ρ_lower)");
    println!("   - Plume probability: exponential in buoyancy force and temperature excess");
    println!("   - Realistic geological process: hot material rises due to density difference");

    println!("\n✅ This tests the fundamental physics of mantle convection!");
    println!("   Real plumes form when heated material expands, becomes buoyant, and rises!");
}
