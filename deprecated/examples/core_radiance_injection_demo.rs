use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🔥 Core Radiance Energy Injection Demonstration");
    println!("===============================================");

    // Create simple 2-layer configuration for focused testing
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,
        surface_gradient_k_per_km: 25.0,
        deep_gradient_k_per_km: 10.0,
        reference_depth_km: 50.0,
    };

    let layer_params = vec![
        // Upper layer (0-25km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1, // Single cell for simplicity
            planet_radius_km: 6371.0,
        },
        // Deep layer (25-50km) - will receive core radiance
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 25.0,
            cell_height_km: 25.0,
            material_name: "basalt".to_string(),
            column_count: 1, // Single cell for simplicity
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 3,
        years_per_step: 1000.0,
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    println!("\n🏗️ Configuration:");
    println!("   - 2 layers: Upper (0-25km) and Deep (25-50km)");
    println!("   - 3 simulation steps of 1000 years each");
    println!("   - Core radiance targets deepest layer only");

    // Helper function to calculate total energy in a layer
    fn calculate_layer_total_energy(sim: &Simulation, layer_index: usize) -> f64 {
        if let Some(layer_set) = sim.layer_sets.get(layer_index) {
            let mut total_energy = 0.0;
            for column in layer_set.layers.values() {
                for cell in &column.cells {
                    total_energy += cell.energy_joules();
                }
            }
            total_energy
        } else {
            0.0
        }
    }

    // Test WITHOUT core radiance
    println!("\n📊 Test 1: WITHOUT Core Radiance");
    println!("--------------------------------");
    
    let mut components_no_radiance: Vec<Box<dyn SimComponent>> = vec![];
    let mut sim_no_radiance = Simulation::new(config.clone(), &mut components_no_radiance);
    sim_no_radiance.initialize();

    // Record initial energies
    let initial_upper_energy = calculate_layer_total_energy(&sim_no_radiance, 0);
    let initial_deep_energy = calculate_layer_total_energy(&sim_no_radiance, 1);
    
    println!("Initial energies:");
    println!("   Upper layer (0-25km): {:.2e} J", initial_upper_energy);
    println!("   Deep layer (25-50km):  {:.2e} J", initial_deep_energy);

    // Run simulation for 3 steps
    for step in 0..3 {
        sim_no_radiance.step();
        let step_deep_energy = calculate_layer_total_energy(&sim_no_radiance, 1);
        println!("   Step {}: Deep layer = {:.2e} J", step + 1, step_deep_energy);
    }

    let final_upper_energy_no_rad = calculate_layer_total_energy(&sim_no_radiance, 0);
    let final_deep_energy_no_rad = calculate_layer_total_energy(&sim_no_radiance, 1);

    println!("Final energies (no radiance):");
    println!("   Upper layer: {:.2e} J (change: {:.2e} J)", 
        final_upper_energy_no_rad, final_upper_energy_no_rad - initial_upper_energy);
    println!("   Deep layer:  {:.2e} J (change: {:.2e} J)", 
        final_deep_energy_no_rad, final_deep_energy_no_rad - initial_deep_energy);

    // Test WITH core radiance
    println!("\n🔥 Test 2: WITH Core Radiance");
    println!("-----------------------------");

    let mut components_with_radiance: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(1e20)        // 1e20 J per cell per year
            .with_noise_amplitude(0.0)     // No noise for predictable results
            .with_spatial_scale(0.1)),
    ];
    let mut sim_with_radiance = Simulation::new(config.clone(), &mut components_with_radiance);
    sim_with_radiance.initialize();

    // Record initial energies (should be same as before)
    let initial_upper_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 0);
    let initial_deep_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 1);

    println!("Initial energies:");
    println!("   Upper layer (0-25km): {:.2e} J", initial_upper_energy_rad);
    println!("   Deep layer (25-50km):  {:.2e} J", initial_deep_energy_rad);

    // Run simulation for 3 steps
    for step in 0..3 {
        sim_with_radiance.step();
        let step_deep_energy = calculate_layer_total_energy(&sim_with_radiance, 1);
        println!("   Step {}: Deep layer = {:.2e} J", step + 1, step_deep_energy);
    }

    let final_upper_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 0);
    let final_deep_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 1);

    println!("Final energies (with radiance):");
    println!("   Upper layer: {:.2e} J (change: {:.2e} J)", 
        final_upper_energy_rad, final_upper_energy_rad - initial_upper_energy_rad);
    println!("   Deep layer:  {:.2e} J (change: {:.2e} J)", 
        final_deep_energy_rad, final_deep_energy_rad - initial_deep_energy_rad);

    // Calculate expected energy injection
    let expected_energy_per_step = 1e20 * 1000.0; // base_energy * years_per_step
    let expected_total_injection = expected_energy_per_step * 3.0; // 3 steps
    let actual_deep_energy_increase = final_deep_energy_rad - initial_deep_energy_rad;

    println!("\n📈 Energy Injection Analysis");
    println!("----------------------------");
    println!("Expected energy injection per step: {:.2e} J", expected_energy_per_step);
    println!("Expected total injection (3 steps): {:.2e} J", expected_total_injection);
    println!("Actual deep layer energy increase:  {:.2e} J", actual_deep_energy_increase);
    
    if expected_total_injection > 0.0 {
        println!("Injection efficiency: {:.1}%", 
            (actual_deep_energy_increase / expected_total_injection) * 100.0);
    }

    // Compare with no-radiance case
    let no_rad_deep_change = final_deep_energy_no_rad - initial_deep_energy;
    let rad_deep_change = final_deep_energy_rad - initial_deep_energy_rad;
    let radiance_effect = rad_deep_change - no_rad_deep_change;

    println!("\n🔬 Radiance Effect Comparison");
    println!("-----------------------------");
    println!("Deep layer change WITHOUT radiance: {:.2e} J", no_rad_deep_change);
    println!("Deep layer change WITH radiance:    {:.2e} J", rad_deep_change);
    println!("Net radiance effect:                {:.2e} J", radiance_effect);
    
    if rad_deep_change != 0.0 {
        println!("Radiance contribution: {:.1}%", 
            (radiance_effect / rad_deep_change) * 100.0);
    }

    println!("\n✅ Core Radiance Energy Injection Results:");
    if actual_deep_energy_increase > 0.0 {
        println!("   ✓ Energy successfully injected into deepest layer");
    } else {
        println!("   ❌ No energy injection detected");
    }
    
    if radiance_effect > 0.0 {
        println!("   ✓ Clear difference between with/without radiance");
    } else {
        println!("   ❌ No significant radiance effect detected");
    }

    if expected_total_injection > 0.0 && actual_deep_energy_increase > expected_total_injection * 0.8 {
        println!("   ✓ Injection amount matches expected values (>80%)");
    } else {
        println!("   ⚠️  Injection amount lower than expected");
    }

    println!("\n🎯 Summary:");
    println!("   This demo clearly shows core radiance injecting energy");
    println!("   specifically into the deepest layer of the simulation,");
    println!("   with quantifiable energy increases over time.");
}
