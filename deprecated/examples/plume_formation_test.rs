use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Plume Formation Test: Does Core Radiance Trigger Plumes?");
    println!("===========================================================");

    // Create thermal configuration that starts closer to plume threshold
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 35.0,    // Higher gradient to get closer to 1800K
        deep_gradient_k_per_km: 25.0,       // Higher deep gradient
        reference_depth_km: 30.0,           // Earlier transition
    };

    let layer_params = vec![
        // Upper layer (0-25km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1,                // Single cell for simplicity
            planet_radius_km: 6371.0,
        },
        // Deep layer (25-50km) - target for core radiance
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 25.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1,                // Single cell for simplicity
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 10,                          // More steps for heating
        years_per_step: 5000.0,            // Longer time steps
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Calculate expected temperatures
    println!("\n🌡️ Expected Initial Temperatures:");
    let surface_temp = config.thermal_config.surface_temperature_k;
    let temp_at_25km = surface_temp + 25.0 * config.thermal_config.surface_gradient_k_per_km;
    let temp_at_50km = temp_at_25km + 25.0 * config.thermal_config.deep_gradient_k_per_km;
    
    println!("   Surface (0km): {:.1}K ({:.1}°C)", surface_temp, surface_temp - 273.15);
    println!("   Mid-depth (25km): {:.1}K ({:.1}°C)", temp_at_25km, temp_at_25km - 273.15);
    println!("   Deep (50km): {:.1}K ({:.1}°C)", temp_at_50km, temp_at_50km - 273.15);
    println!("   Plume threshold: 1800K (1527°C)");
    
    if temp_at_50km < 1800.0 {
        println!("   ⚠️  Initial deep temperature is below plume threshold");
        println!("   🔥 Core radiance needed to reach plume formation temperature");
    } else {
        println!("   ✅ Initial deep temperature may already trigger plumes");
    }

    // Test WITHOUT core radiance
    println!("\n🌋 Test 1: Convection ONLY");
    println!("---------------------------");
    
    let mut components_no_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::with_seed(42)),
    ];
    let mut sim_no_radiance = Simulation::new(config.clone(), &mut components_no_radiance);
    sim_no_radiance.initialize();

    println!("Running 10 steps (50,000 years total)...");
    for step in 0..10 {
        sim_no_radiance.step();
        if step % 2 == 0 {
            println!("   Step {} completed", step + 1);
        }
    }

    // Test WITH high-energy core radiance
    println!("\n🔥 Test 2: Convection + HIGH ENERGY Core Radiance");
    println!("--------------------------------------------------");

    let mut components_with_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(1e21)        // VERY high energy injection (10x higher)
            .with_noise_amplitude(0.2)     // ±20% spatial variation for hot spots
            .with_spatial_scale(0.05)),    // Finer features for localized hot spots
        Box::new(ConvectionPlumeComponent::with_seed(42)),
    ];
    let mut sim_with_radiance = Simulation::new(config.clone(), &mut components_with_radiance);
    sim_with_radiance.initialize();

    println!("Running 10 steps (50,000 years total)...");
    for step in 0..10 {
        sim_with_radiance.step();
        if step % 2 == 0 {
            println!("   Step {} completed", step + 1);
        }
    }

    println!("\n📊 Results Analysis:");
    println!("   Look for plume reports in the output above:");
    println!("   - '🌋 Convection Plumes (Step X): Y active...' indicates plume formation");
    println!("   - Compare plume counts between the two tests");
    println!("   - Higher energy injection should create more/hotter plumes");

    println!("\n🔬 What We're Testing:");
    println!("   1. Can core radiance heat cells above 1800K plume threshold?");
    println!("   2. Do heated cells trigger more plume formation?");
    println!("   3. Does spatial variation create multiple plume sites?");
    println!("   4. How does energy injection affect plume characteristics?");

    println!("\n🎯 Expected Outcome:");
    println!("   - WITHOUT radiance: Few or no plumes (insufficient heating)");
    println!("   - WITH radiance: More plumes (energy injection creates hot spots)");
    println!("   - Demonstrates realistic geological feedback loop");
}
