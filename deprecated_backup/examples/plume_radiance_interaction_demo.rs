use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Plume-Radiance Interaction Demonstration");
    println!("===========================================");

    // Create realistic thermal configuration
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km gradient
        deep_gradient_k_per_km: 10.0,       // 10K/km at depth
        reference_depth_km: 50.0,           // Transition at 50km
    };

    let layer_params = vec![
        // Upper layer (0-50km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km total
            planet_radius_km: 6371.0,
        },
        // Deep layer (50-100km) - will receive core radiance
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km total
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 5,                           // 5 steps for comparison
        years_per_step: 1000.0,            // 1000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    println!("\n🏗️ Configuration:");
    println!("   - 2 layers: Upper (0-50km) and Deep (50-100km)");
    println!("   - 5 simulation steps of 1000 years each");
    println!("   - Plume threshold: 1800K (default)");

    // Helper function to count plumes in a component
    fn count_plumes(components: &[Box<dyn SimComponent>]) -> usize {
        for component in components {
            if component.key() == "convection_plumes" {
                // We need to access the plume count somehow
                // For now, we'll rely on the printed output
                return 0; // Placeholder - we'll see plume counts in output
            }
        }
        0
    }

    // Test 1: ONLY Convection (no core radiance)
    println!("\n🌋 Test 1: Convection ONLY (no core radiance)");
    println!("----------------------------------------------");
    
    let mut components_convection_only: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::with_seed(42)),
    ];
    let mut sim_convection_only = Simulation::new(config.clone(), &mut components_convection_only);
    sim_convection_only.initialize();

    println!("Running 5 steps with convection only...");
    for step in 0..5 {
        sim_convection_only.step();
        println!("   Step {} completed", step + 1);
    }

    // Test 2: Convection + Core Radiance
    println!("\n🔥 Test 2: Convection + Core Radiance");
    println!("------------------------------------");

    let mut components_with_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(5e19)        // Significant energy injection
            .with_noise_amplitude(0.15)    // ±15% spatial variation
            .with_spatial_scale(0.1)),     // Coarse features
        Box::new(ConvectionPlumeComponent::with_seed(42)), // Same seed for comparison
    ];
    let mut sim_with_radiance = Simulation::new(config.clone(), &mut components_with_radiance);
    sim_with_radiance.initialize();

    println!("Running 5 steps with convection + core radiance...");
    for step in 0..5 {
        sim_with_radiance.step();
        println!("   Step {} completed", step + 1);
    }

    println!("\n📊 Analysis:");
    println!("   Look at the plume reports above to compare:");
    println!("   1. Number of active plumes in each case");
    println!("   2. Total plume energy");
    println!("   3. Average plume temperature");
    println!("   4. Plume formation frequency");

    println!("\n🔬 Expected Results:");
    println!("   - WITHOUT radiance: Fewer plumes (limited by initial thermal energy)");
    println!("   - WITH radiance: More plumes (energy injection creates hot spots)");
    println!("   - Higher temperatures in deep layer should trigger more plume formation");
    println!("   - Spatial variation in radiance should create localized hot spots");

    println!("\n🎯 Key Interactions:");
    println!("   1. Core radiance heats deep layer cells");
    println!("   2. Hot cells (>1800K) trigger plume formation");
    println!("   3. Spatial variation creates multiple hot spots");
    println!("   4. More hot spots = more plumes");
    println!("   5. Plumes transport energy upward, creating realistic convection");

    println!("\n✅ This demonstrates the geological feedback loop:");
    println!("   Core Energy → Hot Spots → Plume Formation → Energy Transport");
}
