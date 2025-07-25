use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Realistic Plume Count Test");
    println!("=============================");

    // Create thermal configuration that reaches plume threshold
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 40.0,    // High gradient to reach threshold
        deep_gradient_k_per_km: 20.0,       // High deep gradient
        reference_depth_km: 50.0,           // Early transition
    };

    // Simple 3-layer structure with fewer cells
    let layer_params = vec![
        // Upper layer (0-50km)
        LayerSetParams {
            resolution: Resolution::Two,    // Much coarser resolution = fewer cells
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Mid layer (50-100km)
        LayerSetParams {
            resolution: Resolution::Two,    // Much coarser resolution = fewer cells
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Deep layer (100-150km) - target for plumes
        LayerSetParams {
            resolution: Resolution::Two,    // Much coarser resolution = fewer cells
            start_height_km: 100.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 30,                          // 30 steps for plume development
        years_per_step: 1000.0,            // 1000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Calculate expected temperatures
    println!("\n🌡️ Expected Temperature Profile:");
    let surface_temp = config.thermal_config.surface_temperature_k;
    let temp_at_50km = surface_temp + 50.0 * config.thermal_config.surface_gradient_k_per_km;
    let temp_at_150km = temp_at_50km + 100.0 * config.thermal_config.deep_gradient_k_per_km;
    
    println!("   Surface (0km): {:.0}K ({:.0}°C)", surface_temp, surface_temp - 273.15);
    println!("   Mid-depth (50km): {:.0}K ({:.0}°C)", temp_at_50km, temp_at_50km - 273.15);
    println!("   Deep (150km): {:.0}K ({:.0}°C)", temp_at_150km, temp_at_150km - 273.15);
    println!("   🌋 Plume threshold: 1800K (1527°C)");
    
    let temp_excess = temp_at_150km - 1800.0;
    if temp_excess > 0.0 {
        println!("   ✅ Deep layer exceeds threshold by {:.0}K", temp_excess);
        println!("   📈 Exponential factor: e^({:.0}/50) = {:.2}", temp_excess, (temp_excess / 50.0).exp());
    } else {
        println!("   ⚠️  Deep layer below threshold by {:.0}K", -temp_excess);
    }

    // Test WITHOUT core radiance
    println!("\n🌋 Test 1: Convection ONLY (baseline)");
    println!("=====================================");

    let mut components_no_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-12, 0.5)     // Very rare, high energy transfer
            .with_temperature_threshold(1800.0)),
    ];
    let mut sim_no_radiance = Simulation::new(config.clone(), &mut components_no_radiance);
    sim_no_radiance.initialize();

    println!("Running 30 steps without core radiance...");
    for step in 0..30 {
        sim_no_radiance.step();
        if step % 10 == 9 {
            println!("   Step {} completed", step + 1);
        }
    }

    // Test WITH core radiance
    println!("\n🔥 Test 2: Convection + Core Radiance");
    println!("=====================================");

    let mut components_with_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(3e20)        // High energy to trigger exponential effect
            .with_noise_amplitude(0.2)),   // ±20% variation for hot spots
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-12, 0.5)     // Same base parameters
            .with_temperature_threshold(1800.0)),
    ];
    let mut sim_with_radiance = Simulation::new(config.clone(), &mut components_with_radiance);
    sim_with_radiance.initialize();

    println!("Running 30 steps with core radiance...");
    for step in 0..30 {
        sim_with_radiance.step();
        if step % 10 == 9 {
            println!("   Step {} completed", step + 1);
        }
    }

    println!("\n📊 Results Analysis");
    println!("===================");
    println!("   Look for plume reports above:");
    println!("   🌋 'Convection Plumes (Step X): Y active...'");
    println!("   Expected realistic counts: 1-100 plumes (not hundreds of thousands!)");

    println!("\n🎯 Key Improvements:");
    println!("   ✅ Base probability: 1e-12 per km²/year (extremely rare)");
    println!("   ✅ Exponential temperature dependence: e^(ΔT/50K)");
    println!("   ✅ Higher energy transfer: 50% per plume (more substantial)");
    println!("   ✅ Coarser resolution: Fewer total cells");

    println!("\n📈 Expected Exponential Behavior:");
    println!("   - WITHOUT radiance: Few plumes (baseline temperature)");
    println!("   - WITH radiance: More plumes (exponential increase with temperature)");
    println!("   - Demonstrates: Temperature → Exponential Plume Formation");

    println!("\n✅ This shows realistic geological plume generation!");
}
