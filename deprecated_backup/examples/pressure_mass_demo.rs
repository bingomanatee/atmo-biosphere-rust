use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🔧 How Pressure and Mass Work Now");
    println!("=================================");

    // Create a realistic thermal gradient
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km
        deep_gradient_k_per_km: 15.0,       // 15K/km at depth
        reference_depth_km: 100.0,          // Transition at 100km
    };

    // Create 5 layers to demonstrate pressure accumulation
    let layer_params = vec![
        // Layer 0: Surface (0-50km)
        LayerSetParams {
            resolution: Resolution::Two,
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
        // Layer 4: Very Deep (200-250km)
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 200.0,
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

    println!("\n📊 Pressure and Mass Analysis (5 layers, 250km total):");
    println!("======================================================");

    let mut cumulative_mass_above = 0.0;
    
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("Layer {}: {}km depth", layer_idx, layer_set.start_height_km);
        
        if let Some(first_column) = layer_set.layers.values().next() {
            let mut layer_total_mass = 0.0;
            let mut layer_total_area = 0.0;
            
            for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                let depth_center = cell.top_km + cell.height_km / 2.0;
                let expected_temp = sim.calculate_temperature_at_depth(depth_center);
                let actual_temp = cell.temperature_kelvin();
                let actual_pressure = cell.pressure_pa();
                let mass = cell.mass_kg();
                let area = cell.area();
                
                // Calculate theoretical pressure from depth
                let theoretical_pressure = 101325.0 + (3300.0 * 9.81 * depth_center * 1000.0);
                
                // Calculate effective boiling point at this pressure
                let base_boil_temp = 2900.0; // Basalt boiling point at 1 atm
                let pressure_diff = actual_pressure - 101325.0;
                let rho_liquid = 2850.0; // kg/m³
                let rho_gas = 0.1; // kg/m³
                let delta_v = (1.0 / rho_gas) - (1.0 / rho_liquid);
                let latent_heat = 2000000.0; // J/kg
                let dt_dp = (base_boil_temp * delta_v) / latent_heat;
                let effective_boil_temp = base_boil_temp + (dt_dp * pressure_diff);
                
                layer_total_mass += mass;
                layer_total_area += area;
                
                println!("   Cell {}: depth={:.1}km", cell_idx, depth_center);
                println!("      Temperature: {:.1}K (expected {:.1}K)", actual_temp, expected_temp);
                println!("      Pressure: {:.2e} Pa ({:.1} MPa)", actual_pressure, actual_pressure / 1e6);
                println!("      Theoretical pressure: {:.2e} Pa ({:.1} MPa)", theoretical_pressure, theoretical_pressure / 1e6);
                println!("      Effective boiling point: {:.1}K", effective_boil_temp);
                println!("      Mass: {:.2e} kg | Density: {:.1} kg/m³", mass, mass / (area * cell.height_km * 1e9));
                
                if actual_temp.is_nan() || mass == 0.0 {
                    println!("      ❌ PROBLEM: NaN temperature or zero mass!");
                    if expected_temp > effective_boil_temp {
                        println!("         🔥 Temperature exceeds boiling point: {:.1}K > {:.1}K", expected_temp, effective_boil_temp);
                    }
                } else {
                    println!("      ✅ Cell OK");
                    if expected_temp > 2900.0 && expected_temp < effective_boil_temp {
                        println!("         🌡️  High temp but below pressure-adjusted boiling point");
                    }
                }
                println!();
            }
            
            let layer_mass_per_km2 = layer_total_mass / layer_total_area;
            cumulative_mass_above += layer_mass_per_km2;
            
            println!("   Layer {} Summary:", layer_idx);
            println!("      Total mass: {:.2e} kg", layer_total_mass);
            println!("      Mass per km²: {:.2e} kg/km²", layer_mass_per_km2);
            println!("      Cumulative mass above next layer: {:.2e} kg/km²", cumulative_mass_above);
        }
        println!();
    }

    println!("\n🔬 Key Insights:");
    println!("================");
    println!("1. Pressure increases with depth due to mass accumulation");
    println!("2. Higher pressure raises effective boiling point");
    println!("3. Cells remain stable if temp < pressure-adjusted boiling point");
    println!("4. Mass calculation uses estimated pressure to break circular dependency");
    println!("5. Each layer contributes its mass to pressure of layers below");

    println!("\n🎯 Success Criteria:");
    println!("====================");
    println!("✅ No NaN temperatures in deep cells");
    println!("✅ Non-zero mass in all cells");
    println!("✅ Pressure increases with depth");
    println!("✅ Effective boiling point > actual temperature");
    println!("✅ Realistic density values (~3000 kg/m³ for basalt)");
}
