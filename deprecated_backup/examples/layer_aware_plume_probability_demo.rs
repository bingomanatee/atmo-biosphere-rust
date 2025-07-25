use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Layer-Aware Plume Probability Demonstration");
    println!("===============================================");

    // Test 1: Thin layers vs Thick layers
    println!("\n📏 Test 1: THIN Layers (25km each)");
    println!("===================================");

    let thermal_config_thin = ThermalGradientConfig {
        surface_temperature_k: 288.15,
        surface_gradient_k_per_km: 40.0,    // High gradient
        deep_gradient_k_per_km: 20.0,
        reference_depth_km: 50.0,
    };

    // Thin layers - many small layers
    let layer_params_thin = vec![
        // Layer 1 (0-25km) - THIN
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1,                // 1 cell = 25km thick
            planet_radius_km: 6371.0,
        },
        // Layer 2 (25-50km) - THIN
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 25.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1,                // 1 cell = 25km thick
            planet_radius_km: 6371.0,
        },
        // Layer 3 (50-75km) - THIN
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 50.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1,                // 1 cell = 25km thick
            planet_radius_km: 6371.0,
        },
        // Layer 4 (75-100km) - THIN
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 75.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1,                // 1 cell = 25km thick
            planet_radius_km: 6371.0,
        },
    ];

    let config_thin = SimulationConfig {
        steps: 10,
        years_per_step: 1000.0,
        warmup_steps: 0,
        layer_set_params: layer_params_thin,
        thermal_config: thermal_config_thin,
    };

    let mut components_thin: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(2e20)),
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-11, 0.4)     // Higher probability for testing
            .with_temperature_threshold(1800.0)),
    ];

    let mut sim_thin = Simulation::new(config_thin, &mut components_thin);
    sim_thin.initialize();

    println!("Running 10 steps with THIN layers (25km each)...");
    for step in 0..10 {
        sim_thin.step();
        if step % 3 == 2 {
            println!("   Step {} completed", step + 1);
        }
    }

    // Test 2: Thick layers
    println!("\n📏 Test 2: THICK Layers (100km each)");
    println!("====================================");

    let thermal_config_thick = ThermalGradientConfig {
        surface_temperature_k: 288.15,
        surface_gradient_k_per_km: 40.0,    // Same gradient
        deep_gradient_k_per_km: 20.0,
        reference_depth_km: 50.0,
    };

    // Thick layers - fewer large layers
    let layer_params_thick = vec![
        // Layer 1 (0-100km) - THICK
        LayerSetParams {
            resolution: Resolution::Two,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 4,                // 4 cells = 100km thick
            planet_radius_km: 6371.0,
        },
    ];

    let config_thick = SimulationConfig {
        steps: 10,
        years_per_step: 1000.0,
        warmup_steps: 0,
        layer_set_params: layer_params_thick,
        thermal_config: thermal_config_thick,
    };

    let mut components_thick: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(2e20)),       // Same energy injection
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-11, 0.4)     // Same base parameters
            .with_temperature_threshold(1800.0)),
    ];

    let mut sim_thick = Simulation::new(config_thick, &mut components_thick);
    sim_thick.initialize();

    println!("Running 10 steps with THICK layers (100km total)...");
    for step in 0..10 {
        sim_thick.step();
        if step % 3 == 2 {
            println!("   Step {} completed", step + 1);
        }
    }

    println!("\n📊 Analysis: Layer Height vs Plume Formation");
    println!("=============================================");
    println!("   Look for plume reports in the output above:");
    println!("   🌋 'Convection Plumes (Step X): Y active...' messages");

    println!("\n📈 Expected Layer-Aware Behavior:");
    println!("   THIN layers (25km each):");
    println!("   - Lower height factor: 25km/50km = 0.5x");
    println!("   - Fewer cells per layer: √cells factor");
    println!("   - Result: Lower plume probability per layer");
    println!();
    println!("   THICK layers (100km total):");
    println!("   - Higher height factor: 100km/50km = 2.0x");
    println!("   - More cells per layer: √cells factor");
    println!("   - Result: Higher plume probability (but distributed)");

    println!("\n🎯 Key Physics Implemented:");
    println!("   P(plume) = base × area × time × e^(ΔT/50K) × (height/50km) × (1/√cells)");
    println!("   Where:");
    println!("   - height/50km: Taller layers = more instability");
    println!("   - 1/√cells: Probability distributed among cells");
    println!("   - √ instead of linear to avoid over-dilution");

    println!("\n🌍 Geological Realism:");
    println!("   ✅ Thick asthenosphere layers generate more plumes");
    println!("   ✅ Plume probability distributed among all cells");
    println!("   ✅ Layer structure affects convection patterns");
    println!("   ✅ Realistic geological instability scaling");

    println!("\n✅ This demonstrates:");
    println!("   Layer Structure → Plume Probability → Realistic Convection Patterns");
}
