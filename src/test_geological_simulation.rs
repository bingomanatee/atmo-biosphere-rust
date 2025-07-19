#[cfg(test)]
mod tests {
    use crate::component::conduction_component::ConductionComponent;
    use crate::component::SimComponent;
    use crate::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
    use crate::sim::layer_set::LayerSetParams;
    use h3o::Resolution;

    /// Create a realistic geological simulation
    fn create_geological_simulation() -> Simulation {
        println!("🌍 Creating geological simulation...");

        // Earth-like thermal gradient
        let thermal_config = ThermalGradientConfig {
            surface_temperature_k: 288.15,      // 15°C surface
            surface_gradient_k_per_km: 25.0,    // 25K/km in crust
            deep_gradient_k_per_km: 10.0,       // 10K/km in mantle
            reference_depth_km: 200.0,          // Transition at 200km
        };

        // Realistic geological layers (0-300km)
        let layer_params = vec![
            // Crust: 0-50km
            LayerSetParams {
                resolution: Resolution::Two,
                start_height_km: 0.0,
                cell_height_km: 25.0,
                material_name: "basalt".to_string(),
                column_count: 2,                 // 50km total
                planet_radius_km: 6371.0,
            },
            // Upper mantle: 50-150km
            LayerSetParams {
                resolution: Resolution::One,
                start_height_km: 50.0,
                cell_height_km: 50.0,
                material_name: "granite".to_string(),
                column_count: 2,                 // 100km total
                planet_radius_km: 6371.0,
            },
            // Lower mantle: 150-300km
            LayerSetParams {
                resolution: Resolution::Zero,
                start_height_km: 150.0,
                cell_height_km: 75.0,
                material_name: "basalt".to_string(),
                column_count: 2,                 // 150km total
                planet_radius_km: 6371.0,
            },
        ];

        let config = SimulationConfig {
            steps: 20,                           // Shorter for testing
            years_per_step: 5000.0,             // 5000 years per step
            warmup_steps: 0,
            layer_set_params: layer_params,
            thermal_config,
        };

        // Core components
        let mut components: Vec<Box<dyn SimComponent>> = vec![
            Box::new(ConductionComponent::new()),       // Heat flow only for now
        ];

        Simulation::new(config, &mut components)
    }

    #[test]
    fn test_geological_simulation() {
        println!("🧪 Testing complete geological simulation");

        let mut sim = create_geological_simulation();
        sim.initialize();

        println!("🚀 Running 20 steps (100,000 years)...");

        // Run simulation
        for step in 0..20 {
            sim.step();
            if step % 5 == 4 {
                println!("   Step {}/20 complete ({} years)", step + 1, sim.current_year());
            }
        }

        println!("✅ Simulation complete: {} years", sim.current_year());

        // Basic validation
        assert!(sim.current_step() == 20);
        assert!(sim.current_year() == 100000);
        assert!(sim.layer_sets.len() == 3);

        println!("🎯 Geological simulation test passed!");
    }
}
