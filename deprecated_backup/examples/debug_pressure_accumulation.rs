use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🔍 Debug Pressure Accumulation Through Layers");
    println!("==============================================");

    // Simple thermal config
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km
        deep_gradient_k_per_km: 25.0,       // Keep constant for simplicity
        reference_depth_km: 1000.0,         // No transition
    };

    // Create 4 simple layers to test pressure accumulation
    let layer_params = vec![
        // Layer 0: Surface (0-50km)
        LayerSetParams {
            resolution: Resolution::Two,     // Coarse resolution for simplicity
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 1: Upper (50-100km)
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 2: Mid (100-150km)
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 100.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Layer 3: Deep (150-200km)
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 150.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 1,
        years_per_step: 1000.0,
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Create simulation
    let mut components: Vec<Box<dyn SimComponent>> = vec![];
    let sim = Simulation::new(config, &mut components);

    println!("\n📊 Pressure Analysis Through All Layers:");
    println!("=========================================");

    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("Layer {}: start_height = {}km", layer_idx, layer_set.start_height_km);
        
        // Get first column to examine
        if let Some(first_column) = layer_set.layers.values().next() {
            let mut total_layer_mass = 0.0;
            let mut total_layer_area = 0.0;
            
            for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                let depth_center = cell.top_km + cell.height_km / 2.0;
                let expected_temp = sim.calculate_temperature_at_depth(depth_center);
                let actual_temp = cell.temperature_kelvin();
                let actual_pressure = cell.pressure_pa();
                let mass = cell.mass_kg();
                let area = cell.area();
                
                // Calculate expected pressure at this depth (rough estimate)
                let expected_pressure = 101325.0 + (3300.0 * 9.81 * depth_center * 1000.0);
                
                total_layer_mass += mass;
                total_layer_area += area;
                
                println!("   Cell {}: depth={:.1}km", cell_idx, depth_center);
                println!("      Temp: {:.1}K (expected {:.1}K)", actual_temp, expected_temp);
                println!("      Pressure: {:.2e} Pa (expected ~{:.2e} Pa)", actual_pressure, expected_pressure);
                println!("      Mass: {:.2e} kg | Area: {:.2e} km²", mass, area);
                
                if actual_temp.is_nan() || mass == 0.0 {
                    println!("      ❌ PROBLEM: NaN temperature or zero mass!");
                } else {
                    println!("      ✅ Cell OK");
                }
            }
            
            let avg_mass_per_km2 = total_layer_mass / total_layer_area;
            println!("   Layer {} Summary: {:.2e} kg total, {:.2e} kg/km²", 
                layer_idx, total_layer_mass, avg_mass_per_km2);
        }
        println!();
    }

    println!("\n🔬 Pressure Accumulation Analysis:");
    println!("==================================");
    
    // Calculate what the pressure SHOULD be at each layer
    let mut cumulative_mass_per_km2 = 0.0;
    
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        if let Some(first_column) = layer_set.layers.values().next() {
            if let Some(first_cell) = first_column.cells.first() {
                let expected_pressure_from_above = 101325.0 + (cumulative_mass_per_km2 / 1e6) * 9.81;
                let actual_pressure = first_cell.pressure_pa();
                
                println!("Layer {} (top cell):", layer_idx);
                println!("   Cumulative mass above: {:.2e} kg/km²", cumulative_mass_per_km2);
                println!("   Expected pressure: {:.2e} Pa", expected_pressure_from_above);
                println!("   Actual pressure: {:.2e} Pa", actual_pressure);
                println!("   Ratio (actual/expected): {:.3}", actual_pressure / expected_pressure_from_above);
                
                if (actual_pressure / expected_pressure_from_above - 1.0).abs() > 0.1 {
                    println!("   ❌ PRESSURE MISMATCH!");
                } else {
                    println!("   ✅ Pressure OK");
                }
            }
            
            // Add this layer's mass to cumulative for next layer
            let layer_mass = first_column.cells.iter().map(|c| c.mass_kg()).sum::<f64>();
            let layer_area = first_column.cells.iter().map(|c| c.area()).sum::<f64>();
            cumulative_mass_per_km2 += layer_mass / layer_area;
        }
        println!();
    }

    println!("\n🎯 Key Findings:");
    println!("================");
    println!("1. Check if pressure increases with depth");
    println!("2. Check if pressure accumulation is working");
    println!("3. Check if mass calculations are correct");
    println!("4. Identify where the pressure calculation breaks");
}
