use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Realistic Plume Generation Test: 400km Deep, Multiple Layers");
    println!("================================================================");

    // Realistic thermal gradient for 400km depth
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km in crust
        deep_gradient_k_per_km: 8.0,        // 8K/km in deep asthenosphere
        reference_depth_km: 150.0,          // Transition at 150km
    };

    // Create realistic multi-layer structure to 400km depth
    let layer_params = vec![
        // Crust (0-50km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Upper Mantle (50-100km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 50.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Lithosphere-Asthenosphere Boundary (100-150km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 100.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Upper Asthenosphere (150-200km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 150.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Mid Asthenosphere (200-250km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 200.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Lower Asthenosphere (250-300km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 250.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Deep Asthenosphere (300-350km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 300.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Deepest Asthenosphere (350-400km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 350.0,
            cell_height_km: 25.0,           // 25km cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 50,                          // 50 steps as requested
        years_per_step: 1000.0,            // 1000 years per step (50,000 years total)
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Calculate expected temperatures at key depths
    println!("\n🌡️ Expected Temperature Profile (400km deep):");
    let surface_temp = config.thermal_config.surface_temperature_k;
    let temp_at_150km = surface_temp + 150.0 * config.thermal_config.surface_gradient_k_per_km;
    let temp_at_400km = temp_at_150km + 250.0 * config.thermal_config.deep_gradient_k_per_km;
    
    println!("   Surface (0km): {:.0}K ({:.0}°C)", surface_temp, surface_temp - 273.15);
    println!("   Crust (50km): {:.0}K ({:.0}°C)", surface_temp + 50.0 * 25.0, surface_temp + 50.0 * 25.0 - 273.15);
    println!("   Upper Mantle (100km): {:.0}K ({:.0}°C)", surface_temp + 100.0 * 25.0, surface_temp + 100.0 * 25.0 - 273.15);
    println!("   Asthenosphere (150km): {:.0}K ({:.0}°C)", temp_at_150km, temp_at_150km - 273.15);
    println!("   Mid Asthenosphere (250km): {:.0}K ({:.0}°C)", temp_at_150km + 100.0 * 8.0, temp_at_150km + 100.0 * 8.0 - 273.15);
    println!("   Deep Asthenosphere (400km): {:.0}K ({:.0}°C)", temp_at_400km, temp_at_400km - 273.15);
    println!("   🌋 Plume threshold: 1800K (1527°C)");
    
    if temp_at_400km > 1800.0 {
        println!("   ✅ Deep temperatures should trigger plume formation!");
    } else {
        println!("   ⚠️  May need core radiance to reach plume threshold");
    }

    println!("\n🏗️ Layer Structure (8 LayerSets, 16 total cells):");
    println!("   0. Crust (0-50km): 2 cells × 25km");
    println!("   1. Upper Mantle (50-100km): 2 cells × 25km");
    println!("   2. LAB (100-150km): 2 cells × 25km");
    println!("   3. Upper Asthenosphere (150-200km): 2 cells × 25km");
    println!("   4. Mid Asthenosphere (200-250km): 2 cells × 25km");
    println!("   5. Lower Asthenosphere (250-300km): 2 cells × 25km");
    println!("   6. Deep Asthenosphere (300-350km): 2 cells × 25km");
    println!("   7. Deepest Asthenosphere (350-400km): 2 cells × 25km");
    println!("   → 7 inter-layer boundaries for plume generation!");

    // Test 1: ONLY Convection (no core radiance)
    println!("\n🌋 Test 1: Convection ONLY (8 layers, 50 steps)");
    println!("================================================");
    
    let mut components_convection_only: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::with_seed(42)),
    ];
    let mut sim_convection_only = Simulation::new(config.clone(), &mut components_convection_only);
    sim_convection_only.initialize();

    println!("Running 50 steps × 1000 years = 50,000 years total...");
    for step in 0..50 {
        sim_convection_only.step();
        if step % 10 == 9 {
            println!("   Step {} completed ({} years)", step + 1, (step + 1) * 1000);
        }
    }

    // Test 2: Convection + Core Radiance
    println!("\n🔥 Test 2: Convection + Core Radiance (8 layers, 50 steps)");
    println!("===========================================================");

    let mut components_with_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(2e20)        // Substantial energy injection
            .with_noise_amplitude(0.15)    // ±15% spatial variation
            .with_spatial_scale(0.08)      // Coarse features for hot spots
            .with_geological_drift()),     // Add temporal drift
        Box::new(ConvectionPlumeComponent::with_seed(42)), // Same seed for comparison
    ];
    let mut sim_with_radiance = Simulation::new(config.clone(), &mut components_with_radiance);
    sim_with_radiance.initialize();

    println!("Running 50 steps × 1000 years = 50,000 years total...");
    for step in 0..50 {
        sim_with_radiance.step();
        if step % 10 == 9 {
            println!("   Step {} completed ({} years)", step + 1, (step + 1) * 1000);
        }
    }

    println!("\n📊 Analysis: Look for Plume Reports");
    println!("===================================");
    println!("   Search output above for:");
    println!("   🌋 'Convection Plumes (Step X): Y active...' messages");
    println!("   Compare plume counts between the two tests:");
    println!("   - WITHOUT radiance: Baseline plume formation");
    println!("   - WITH radiance: Enhanced plume formation");

    println!("\n🎯 Expected Results:");
    println!("   - More layers = more plume generation opportunities");
    println!("   - Longer simulation = time for plumes to develop");
    println!("   - Core radiance = additional energy for hot spots");
    println!("   - Spatial variation = multiple plume initiation sites");
    println!("   - Temporal drift = evolving plume patterns over time");

    println!("\n✅ This demonstrates realistic geological convection:");
    println!("   Deep Energy → Multi-Layer Heating → Plume Formation → Energy Transport");
}
