use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌋 Convection Measurement Demo: Isolate Convection Effects");
    println!("===========================================================");

    // Realistic thermal gradient for 250km depth
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 30.0,    // 30K/km in crust/lithosphere
        deep_gradient_k_per_km: 5.0,        // 5K/km in asthenosphere
        reference_depth_km: 80.0,           // Transition at lithosphere-asthenosphere boundary
    };

    // Realistic geological layer structure (0-250km)
    let layer_params = vec![
        // Continental Crust (0-35km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 0.0,
            cell_height_km: 17.5,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 35km
            planet_radius_km: 6371.0,
        },
        // Lithospheric Mantle (35-80km)
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 35.0,
            cell_height_km: 22.5,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 45km
            planet_radius_km: 6371.0,
        },
        // Upper Asthenosphere (80-120km) - Main convection zone
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 80.0,
            cell_height_km: 20.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 40km
            planet_radius_km: 6371.0,
        },
        // Mid Asthenosphere (120-160km) - Main convection zone
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 120.0,
            cell_height_km: 20.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 40km
            planet_radius_km: 6371.0,
        },
        // Lower Asthenosphere (160-200km) - Main convection zone
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 160.0,
            cell_height_km: 20.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 40km
            planet_radius_km: 6371.0,
        },
        // Deep Transition Zone (200-250km) - Energy injection target
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 200.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 3,                           // 3 steps for measurement
        years_per_step: 2000.0,            // 2000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config: thermal_config.clone(),
    };

    println!("\n🎯 Experimental Design:");
    println!("========================");
    println!("1. 📊 Measure baseline convection (no energy injection)");
    println!("2. 🔥 Apply core radiance → measure plume response");
    println!("3. 🔄 Reset deep layer temperature → measure convection transport only");
    println!("4. 📈 Compare convection effects with and without energy injection");

    // Helper function to reset deep layer temperature to baseline
    fn reset_deep_layer_temperature(sim: &mut Simulation, thermal_config: &ThermalGradientConfig) {
        let deep_layer_index = sim.layer_sets.len() - 1; // Last layer (200-250km)
        
        if let Some(deep_layer) = sim.layer_sets.get_mut(deep_layer_index) {
            for column in deep_layer.layers.values_mut() {
                for cell in column.cells.iter_mut() {
                    let depth_center = cell.top_km + cell.height_km / 2.0;
                    let baseline_temp = thermal_config.calculate_temperature_at_depth(depth_center);
                    
                    // Reset to baseline temperature while preserving mass and pressure
                    let mass = cell.mass_kg();
                    let specific_heat = 1000.0; // Approximate basalt specific heat
                    let baseline_energy = mass * specific_heat * baseline_temp;
                    
                    cell.set_energy_joules(baseline_energy);
                }
            }
        }
        
        println!("🔄 Deep layer temperature reset to baseline thermal gradient");
    }

    // Helper function to count plumes
    fn count_plumes(components: &[Box<dyn SimComponent>]) -> usize {
        for component in components {
            if component.key() == "convection_plumes" {
                // We'll rely on printed output for plume counts
                return 0; // Placeholder
            }
        }
        0
    }

    // Test 1: BASELINE (no energy injection)
    println!("\n🌋 Test 1: BASELINE Convection (no energy injection)");
    println!("===================================================");

    let mut components_baseline: Vec<Box<dyn SimComponent>> = vec![
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-10, 0.4)     // Higher probability for measurement
            .with_temperature_threshold(1800.0)),
    ];
    let mut sim_baseline = Simulation::new(config.clone(), &mut components_baseline);
    sim_baseline.initialize();

    println!("Running 3 steps with baseline convection only...");
    for step in 0..3 {
        sim_baseline.step();
        println!("   Step {} completed", step + 1);
    }

    // Test 2: ENERGY INJECTION + CONVECTION MEASUREMENT
    println!("\n🔥 Test 2: Energy Injection + Convection Response");
    println!("=================================================");

    let mut components_with_injection: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(2e20)        // Moderate energy injection
            .with_noise_amplitude(0.1)),   // ±10% variation
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-10, 0.4)     // Same parameters for comparison
            .with_temperature_threshold(1800.0)),
    ];
    let mut sim_with_injection = Simulation::new(config.clone(), &mut components_with_injection);
    sim_with_injection.initialize();

    println!("Step 1: Apply energy injection and measure plume response...");
    sim_with_injection.step();
    
    println!("Step 2: Reset deep layer temperature and measure convection transport...");
    reset_deep_layer_temperature(&mut sim_with_injection, &thermal_config);
    sim_with_injection.step();
    
    println!("Step 3: Continue measuring convection effects...");
    sim_with_injection.step();

    println!("\n📊 Analysis: Convection Measurement Results");
    println!("===========================================");
    println!("   Look for plume reports in the output above:");
    println!("   🌋 'Convection Plumes (Step X): Y active...' messages");
    println!("   Compare plume counts between baseline and injection tests:");

    println!("\n🎯 Expected Results:");
    println!("   📊 BASELINE: Natural convection from thermal gradient");
    println!("   🔥 INJECTION Step 1: Enhanced plume formation from energy injection");
    println!("   🔄 INJECTION Step 2: Convection transport after temperature reset");
    println!("   📈 INJECTION Step 3: Continued convection effects");

    println!("\n🔬 Key Measurements:");
    println!("   1. 🌋 Plume count increase due to energy injection");
    println!("   2. 🌊 Heat transport efficiency from convection");
    println!("   3. 📊 Energy redistribution patterns");
    println!("   4. ⏱️ Convection response time to energy injection");

    println!("\n✅ This isolates convection effects by:");
    println!("   - Measuring baseline convection without injection");
    println!("   - Triggering enhanced convection with energy injection");
    println!("   - Resetting temperature to isolate transport effects");
    println!("   - Comparing convection efficiency between scenarios");

    println!("\n🎯 Next Steps:");
    println!("   After measuring convection response, we can implement:");
    println!("   - Cell-to-cell radiance/conduction system");
    println!("   - Realistic heat transport mechanisms");
    println!("   - Coupled convection-conduction models");
}
