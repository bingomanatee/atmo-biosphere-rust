use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌍 Crust-to-Asthenosphere Convection Demo (0-300km)");
    println!("===================================================");

    // Create realistic thermal configuration for crust-to-asthenosphere
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface temperature
        surface_gradient_k_per_km: 25.0,    // 25K/km in crust (realistic)
        deep_gradient_k_per_km: 10.0,       // 10K/km in asthenosphere (lower)
        reference_depth_km: 100.0,          // Transition at 100km depth
    };

    // Create realistic crust-to-asthenosphere layer sets (0-300km)
    let layer_params = vec![
        // Crust (0-50km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 10.0,           // 10km thick cells
            material_name: "basalt".to_string(),
            column_count: 5,                // 5 cells = 50km total
            planet_radius_km: 6371.0,
        },
        // Upper Mantle/Lithosphere (50-150km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 50.0,
            cell_height_km: 20.0,           // 20km thick cells
            material_name: "basalt".to_string(),
            column_count: 5,                // 5 cells = 100km total
            planet_radius_km: 6371.0,
        },
        // Asthenosphere (150-300km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 150.0,
            cell_height_km: 30.0,           // 30km thick cells
            material_name: "basalt".to_string(),
            column_count: 5,                // 5 cells = 150km total
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 10,                          // 10 simulation steps
        years_per_step: 10000.0,           // 10,000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    println!("\n🏗️ Layer Configuration:");
    println!("   LayerSet 0 (Crust):           0-50km   (5 cells × 10km)");
    println!("   LayerSet 1 (Upper Mantle):    50-150km (5 cells × 20km)");
    println!("   LayerSet 2 (Asthenosphere):   150-300km (5 cells × 30km)");
    println!("   Total depth: 300km");
    println!("   Total cells per column: 15");

    // Calculate expected temperatures at key depths
    println!("\n🌡️ Expected Temperature Profile:");
    let depths = [0.0, 25.0, 50.0, 100.0, 150.0, 200.0, 250.0, 300.0];
    for depth in depths {
        let temp_k = if depth <= config.thermal_config.reference_depth_km {
            config.thermal_config.surface_temperature_k + 
            depth * config.thermal_config.surface_gradient_k_per_km
        } else {
            let temp_at_ref = config.thermal_config.surface_temperature_k + 
                config.thermal_config.reference_depth_km * config.thermal_config.surface_gradient_k_per_km;
            temp_at_ref + (depth - config.thermal_config.reference_depth_km) * config.thermal_config.deep_gradient_k_per_km
        };
        let temp_c = temp_k - 273.15;
        println!("   {}km: {:.0}K ({:.0}°C)", depth, temp_k, temp_c);
    }

    println!("\n🌋 Convection Expectations:");
    println!("   - Plume threshold: 1800K");
    println!("   - Expected plume depth: ~200-300km (asthenosphere)");
    println!("   - Transport direction: Asthenosphere → Upper Mantle → Crust");
    println!("   - Energy redistribution: Deep heat moves upward");

    // Create components
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::with_seed(42)),
    ];

    println!("\n🚀 Creating simulation...");
    let mut sim = Simulation::new(config, &mut components);
    
    println!("✓ Simulation created with {} layer sets", sim.layer_sets.len());
    
    // Show actual layer structure
    for (i, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("   LayerSet {}: {} H3 columns, start_height = {}km", 
            i, layer_set.layers.len(), layer_set.start_height_km);
    }

    println!("\n🔧 Initializing simulation...");
    sim.initialize();
    println!("✓ Simulation initialized");

    println!("\n📊 Energy Analysis Framework:");
    println!("   This configuration enables studying:");
    println!("   1. Crustal thermal evolution (0-50km)");
    println!("   2. Lithospheric convection (50-150km)");
    println!("   3. Asthenospheric plume generation (150-300km)");
    println!("   4. Inter-layer energy transport");
    println!("   5. Realistic geological timescales");

    println!("\n⚡ Energy Distribution Potential:");
    println!("   - Crust: Receives energy from below, loses to surface");
    println!("   - Upper Mantle: Transition zone, moderate convection");
    println!("   - Asthenosphere: Primary convection source, hottest layer");

    println!("\n✅ Demo setup completed!");
    println!("\n📝 To run full energy analysis:");
    println!("   cargo test test_energy_distribution_with_and_without_convection -- --nocapture");
    println!("\n🎯 This 300km configuration provides:");
    println!("   ✓ Realistic geological depth range");
    println!("   ✓ Proper crust-mantle-asthenosphere structure");
    println!("   ✓ Appropriate cell resolution for each layer");
    println!("   ✓ Convection-relevant temperature gradients");
    println!("   ✓ Inter-layer energy transport capability");
}
