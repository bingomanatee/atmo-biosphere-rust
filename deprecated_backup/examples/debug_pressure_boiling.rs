use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌡️ Debug Pressure vs Boiling Point at Depth");
    println!("============================================");

    // Same thermal config as the failing demo
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km in crust
        deep_gradient_k_per_km: 8.0,        // 8K/km in deep asthenosphere
        reference_depth_km: 150.0,          // Transition at 150km
    };

    // Test pressure calculations at various depths
    println!("\n🔍 Expected Pressure vs Depth:");
    let test_depths = [0.0, 50.0, 100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0];
    
    for depth in test_depths {
        // Rough pressure calculation: P = ρ * g * h
        // Average mantle density ~3300 kg/m³, g = 9.81 m/s²
        let pressure_pa = 3300.0 * 9.81 * depth * 1000.0; // Convert km to m
        let pressure_gpa = pressure_pa / 1e9; // Convert to GPa for readability
        
        let temp_k = thermal_config.calculate_temperature_at_depth(depth);
        let temp_c = temp_k - 273.15;
        
        println!("   Depth {:3.0}km: {:6.1}K ({:6.1}°C) | Pressure: {:.1} GPa ({:.2e} Pa)", 
            depth, temp_k, temp_c, pressure_gpa, pressure_pa);
    }

    // Create a simple single-layer test to examine pressure in cells
    let layer_params = vec![
        // Deep layer (350-400km) - where the problem occurs
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

    // Create simulation and check actual cell pressures
    let mut components: Vec<Box<dyn SimComponent>> = vec![];
    let sim = Simulation::new(config, &mut components);

    println!("\n🏗️ Actual Cell Conditions in Deep Layer (350-400km):");
    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("   Layer {}: start_height = {}km", layer_idx, layer_set.start_height_km);
        
        // Get first column to examine
        if let Some(first_column) = layer_set.layers.values().next() {
            for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                let depth_center = cell.top_km + cell.height_km / 2.0;
                let expected_temp = sim.calculate_temperature_at_depth(depth_center);
                let actual_temp = cell.temperature_kelvin();
                let actual_pressure = cell.pressure_pa();
                let actual_energy = cell.energy_joules();
                let mass = cell.mass_kg();
                
                // Calculate expected pressure at this depth
                let expected_pressure = 3300.0 * 9.81 * depth_center * 1000.0;
                
                // Calculate effective boiling point at this pressure
                // Using Clausius-Clapeyron: dT/dP = T*ΔV/L
                let base_boil_temp = 2900.0; // Basalt boiling point at 1 atm
                let pressure_diff = actual_pressure - 101325.0; // Pressure above 1 atm
                
                // Rough estimate: ΔV ≈ 1/ρ_gas - 1/ρ_liquid
                let rho_liquid = 2850.0; // kg/m³ (basalt liquid density)
                let rho_gas = 0.1; // kg/m³ (basalt gas density)
                let delta_v = (1.0 / rho_gas) - (1.0 / rho_liquid);
                let latent_heat = 2000000.0; // J/kg (basalt latent heat of vaporization)
                
                let dt_dp = (base_boil_temp * delta_v) / latent_heat;
                let effective_boil_temp = base_boil_temp + (dt_dp * pressure_diff);
                
                println!("      Cell {}: depth={:.1}km", cell_idx, depth_center);
                println!("         Expected temp: {:.1}K | Actual temp: {:.1}K", expected_temp, actual_temp);
                println!("         Expected pressure: {:.2e} Pa | Actual pressure: {:.2e} Pa", expected_pressure, actual_pressure);
                println!("         Base boiling point: {:.1}K | Effective boiling point: {:.1}K", base_boil_temp, effective_boil_temp);
                println!("         Energy: {:.2e}J | Mass: {:.2e}kg", actual_energy, mass);
                
                if actual_temp.is_nan() || actual_energy == 0.0 {
                    println!("         ❌ PROBLEM: NaN temperature or zero energy!");
                    
                    if actual_pressure < expected_pressure * 0.1 {
                        println!("         🔍 Pressure too low: {:.2e} vs expected {:.2e}", actual_pressure, expected_pressure);
                    }
                    
                    if expected_temp > effective_boil_temp {
                        println!("         🔍 Temperature exceeds effective boiling point: {:.1}K > {:.1}K", expected_temp, effective_boil_temp);
                    } else {
                        println!("         🔍 Temperature should be below boiling point: {:.1}K < {:.1}K", expected_temp, effective_boil_temp);
                    }
                } else {
                    println!("         ✅ Cell initialized correctly");
                }
                println!();
            }
        }
    }

    println!("\n🔬 Analysis:");
    println!("   1. Check if actual pressure matches expected geological pressure");
    println!("   2. Check if effective boiling point is calculated correctly");
    println!("   3. Check if temperature is below the pressure-adjusted boiling point");
    println!("   4. Identify where the pressure/phase calculation fails");
    
    println!("\n📚 Expected Physics:");
    println!("   - At 400km depth: ~13 GPa pressure");
    println!("   - Clausius-Clapeyron should raise boiling point to ~6000K+");
    println!("   - 5000K temperature should still be solid/liquid basalt");
    println!("   - Mass and energy should be non-zero");
}
