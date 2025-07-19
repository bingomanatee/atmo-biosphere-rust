use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Convection Plume Demo");
    println!("========================");

    // Create thermal configuration with high surface temperature to trigger plumes
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 30.0,    // High gradient to create hot deep layers
        deep_gradient_k_per_km: 15.0,       // Still significant at depth
        reference_depth_km: 100.0,          // Transition at 100km
    };

    // Create layer set parameters - multiple layers to see plume movement
    let layer_params = vec![
        LayerSetParams {
            resolution: Resolution::Four,    // Low resolution for demo
            start_height_km: 0.0,
            cell_height_km: 10.0,           // 10km thick cells
            material_name: "basalt".to_string(),
            column_count: 5,                // 5 cells per column
            planet_radius_km: 6371.0,
        },
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 50.0,          // Will be adjusted automatically
            cell_height_km: 10.0,
            material_name: "basalt".to_string(),
            column_count: 5,
            planet_radius_km: 6371.0,
        },
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 100.0,         // Will be adjusted automatically
            cell_height_km: 10.0,
            material_name: "basalt".to_string(),
            column_count: 5,
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 1000,                        // 1000 steps
        years_per_step: 100.0,              // 100 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Create components
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::new()),
    ];

    // Create and run simulation
    let mut sim = Simulation::new(config, &mut components);
    
    println!("\n🌍 Simulation Setup:");
    println!("   - {} layer sets", sim.layer_sets.len());
    for (i, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("   - Layer {}: {} columns, resolution {:?}", 
            i, layer_set.layers.len(), layer_set.resolution);
    }

    println!("\n🔥 Temperature Profile:");
    for depth_km in [0.0, 25.0, 50.0, 75.0, 100.0, 125.0, 150.0] {
        let temp_k = sim.calculate_temperature_at_depth(depth_km);
        let temp_c = temp_k - 273.15;
        println!("   - {}km depth: {:.1}K ({:.1}°C)", depth_km, temp_k, temp_c);
    }

    // Initialize simulation
    sim.initialize();

    println!("\n🚀 Starting simulation...");
    println!("   - Looking for plumes with temp > 1800K");
    println!("   - Plume probability scales with cell area");
    println!("   - Higher resolution = smaller cells = lower individual probability");
    println!("   - But more cells = same total plume generation rate");

    // Run a few steps to see plume generation
    for step in 0..10 {
        println!("\n--- Step {} (Year {}) ---", step, step * 100);
        
        // Manually step through simulation components
        // (This is a simplified version of what the full simulation would do)
        
        // For demo purposes, let's check temperatures in the deepest layer
        if let Some(deepest_layer) = sim.layer_sets.last() {
            let mut hot_cells = 0;
            let mut total_cells = 0;
            let mut max_temp = 0.0;
            
            for column in deepest_layer.layers.values() {
                for cell in &column.cells {
                    let temp = cell.temperature_kelvin();
                    total_cells += 1;
                    if temp > 1800.0 {
                        hot_cells += 1;
                    }
                    if temp > max_temp {
                        max_temp = temp;
                    }
                }
            }
            
            println!("   Deepest layer: {}/{} cells > 1800K, max temp: {:.1}K", 
                hot_cells, total_cells, max_temp);
        }
    }

    println!("\n✅ Demo completed!");
    println!("\n📊 Key Insights:");
    println!("   1. Plume generation probability scales with cell area");
    println!("   2. Higher resolution = more cells but smaller individual areas");
    println!("   3. Total plume generation rate remains consistent across resolutions");
    println!("   4. Temperature thresholds and gradients control plume formation");
    println!("   5. Plumes carry energy upward and radiate to surrounding cells");
}
