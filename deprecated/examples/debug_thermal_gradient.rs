use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌡️ Debug Thermal Gradient Calculation");
    println!("=====================================");

    // Same thermal config as the failing demo
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km in crust
        deep_gradient_k_per_km: 8.0,        // 8K/km in deep asthenosphere
        reference_depth_km: 150.0,          // Transition at 150km
    };

    // Test the thermal gradient calculation directly
    println!("\n🔍 Direct Thermal Gradient Test:");
    let test_depths = [0.0, 50.0, 100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0];
    
    for depth in test_depths {
        let temp_k = thermal_config.calculate_temperature_at_depth(depth);
        let temp_c = temp_k - 273.15;
        let gradient = thermal_config.gradient_at_depth(depth);
        
        println!("   Depth {:3.0}km: {:6.1}K ({:6.1}°C) | Gradient: {:4.1}K/km", 
            depth, temp_k, temp_c, gradient);
    }

    // Create the same layer structure as the failing demo
    let layer_params = vec![
        // Layer 0: Crust (0-50km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 1: Upper Mantle (50-100km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 2: LAB (100-150km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 100.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 3: Upper Asthenosphere (150-200km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 150.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 4: Mid Asthenosphere (200-250km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 200.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 5: Lower Asthenosphere (250-300km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 250.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 6: Deep Asthenosphere (300-350km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 300.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 7: Deepest Asthenosphere (350-400km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 350.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 1,                           // Just 1 step for debugging
        years_per_step: 1000.0,
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Create simulation and check actual cell temperatures
    let mut components: Vec<Box<dyn SimComponent>> = vec![];
    let sim = Simulation::new(config, &mut components);

    println!("\n🏗️ Actual Cell Temperatures After Initialization:");
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("   Layer {}: start_height = {}km", layer_idx, layer_set.start_height_km);
        
        // Get first column to examine
        if let Some(first_column) = layer_set.layers.values().next() {
            for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                let depth_center = cell.top_km + cell.height_km / 2.0;
                let expected_temp = sim.calculate_temperature_at_depth(depth_center);
                let actual_temp = cell.temperature_kelvin();
                let actual_energy = cell.energy_joules();
                let mass = cell.mass_kg();
                
                println!("      Cell {}: depth={:.1}km | expected={:.1}K | actual={:.1}K | energy={:.2e}J | mass={:.2e}kg", 
                    cell_idx, depth_center, expected_temp, actual_temp, actual_energy, mass);
                
                if actual_temp.is_nan() || actual_energy == 0.0 {
                    println!("         ❌ PROBLEM: NaN temperature or zero energy!");
                }
            }
        }
        println!();
    }

    println!("\n🔍 Analysis:");
    println!("   - Check if expected temperatures are reasonable");
    println!("   - Check if actual temperatures match expected");
    println!("   - Check if energy is properly calculated from temperature");
    println!("   - Identify where the calculation breaks down");
}
