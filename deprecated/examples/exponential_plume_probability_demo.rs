use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Exponential Plume Probability Demonstration");
    println!("==============================================");

    // Create a realistic multi-layer structure but smaller for focused testing
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 30.0,    // 30K/km gradient (higher for testing)
        deep_gradient_k_per_km: 15.0,       // 15K/km at depth
        reference_depth_km: 100.0,          // Transition at 100km
    };

    let layer_params = vec![
        // Upper layer (0-50km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Mid layer (50-100km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Deep layer (100-150km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 100.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
        // Deepest layer (150-200km) - will receive core radiance
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 150.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 20,                          // 20 steps for progression
        years_per_step: 1000.0,            // 1000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Calculate expected temperatures
    println!("\n🌡️ Expected Temperature Profile:");
    let surface_temp = config.thermal_config.surface_temperature_k;
    let temp_at_100km = surface_temp + 100.0 * config.thermal_config.surface_gradient_k_per_km;
    let temp_at_200km = temp_at_100km + 100.0 * config.thermal_config.deep_gradient_k_per_km;
    
    println!("   Surface (0km): {:.0}K ({:.0}°C)", surface_temp, surface_temp - 273.15);
    println!("   Mid-depth (100km): {:.0}K ({:.0}°C)", temp_at_100km, temp_at_100km - 273.15);
    println!("   Deep (200km): {:.0}K ({:.0}°C)", temp_at_200km, temp_at_200km - 273.15);
    println!("   🌋 Plume threshold: 1800K (1527°C)");
    
    let temp_excess_at_200km = temp_at_200km - 1800.0;
    if temp_excess_at_200km > 0.0 {
        println!("   ✅ Deep layer exceeds threshold by {:.0}K", temp_excess_at_200km);
        println!("   📈 Exponential factor: e^({:.0}/50) = {:.2}", temp_excess_at_200km, (temp_excess_at_200km / 50.0).exp());
    } else {
        println!("   ⚠️  Deep layer below threshold by {:.0}K", -temp_excess_at_200km);
    }

    println!("\n🏗️ Layer Structure (4 layers, 200km total):");
    println!("   0. Upper (0-50km): 2 cells × 25km");
    println!("   1. Mid (50-100km): 2 cells × 25km");
    println!("   2. Deep (100-150km): 2 cells × 25km");
    println!("   3. Deepest (150-200km): 2 cells × 25km");

    // Test 1: Moderate energy injection
    println!("\n🔥 Test 1: MODERATE Core Radiance (baseline)");
    println!("============================================");

    let mut components_moderate: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(1e20)        // Moderate energy injection
            .with_noise_amplitude(0.1)),   // ±10% variation
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-9, 0.3)  // Rare plumes, substantial energy
            .with_temperature_threshold(1800.0)), // Standard threshold
    ];
    let mut sim_moderate = Simulation::new(config.clone(), &mut components_moderate);
    sim_moderate.initialize();

    println!("Running 20 steps with moderate energy injection...");
    for step in 0..20 {
        sim_moderate.step();
        if step % 5 == 4 {
            println!("   Step {} completed", step + 1);
        }
    }

    // Test 2: High energy injection (should trigger exponential increase)
    println!("\n🔥 Test 2: HIGH Core Radiance (exponential trigger)");
    println!("===================================================");

    let mut components_high: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(5e20)        // 5x higher energy injection
            .with_noise_amplitude(0.15)),  // ±15% variation for hot spots
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-9, 0.3)  // Same base parameters
            .with_temperature_threshold(1800.0)), // Same threshold
    ];
    let mut sim_high = Simulation::new(config.clone(), &mut components_high);
    sim_high.initialize();

    println!("Running 20 steps with high energy injection...");
    for step in 0..20 {
        sim_high.step();
        if step % 5 == 4 {
            println!("   Step {} completed", step + 1);
        }
    }

    println!("\n📊 Analysis: Exponential Temperature Dependence");
    println!("===============================================");
    println!("   Look for plume reports in the output above:");
    println!("   🌋 'Convection Plumes (Step X): Y active...' messages");
    println!("   Compare plume counts between moderate and high energy:");

    println!("\n📈 Expected Exponential Behavior:");
    println!("   - Moderate energy: Few plumes (near threshold)");
    println!("   - High energy: Many more plumes (exponential increase)");
    println!("   - Temperature excess drives exponential probability:");
    println!("     * +50K excess → 2.7x more plumes");
    println!("     * +100K excess → 7.4x more plumes");
    println!("     * +150K excess → 20x more plumes");

    println!("\n🎯 Key Physics:");
    println!("   P(plume) = base_prob × area × time × e^(ΔT/50K)");
    println!("   Where ΔT = cell_temperature - 1800K");
    println!("   This models realistic geological instability!");

    println!("\n✅ This demonstrates:");
    println!("   Temperature → Exponential Plume Formation → Realistic Convection");
}
