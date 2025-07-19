#[cfg(test)]
mod convection_simulation_tests {
    use crate::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
    use crate::sim::layer_set::LayerSetParams;
    use crate::component::{SimComponent, ConvectionPlumeComponent};
    use crate::energy_mass::energy_mass::EnergyMass;
    use h3o::Resolution;

    /// Test configuration for multi-layer convection simulations
    fn create_test_simulation_config() -> SimulationConfig {
        // Create thermal configuration with high deep temperatures to trigger plumes
        let thermal_config = ThermalGradientConfig {
            surface_temperature_k: 288.15,      // 15°C surface
            surface_gradient_k_per_km: 35.0,    // High gradient to create hot deep layers
            deep_gradient_k_per_km: 20.0,       // Still significant at depth
            reference_depth_km: 50.0,           // Transition at 50km
        };

        // Create multiple layer sets to test inter-layer transport
        let layer_params = vec![
            // Surface layer (0-50km)
            LayerSetParams {
                resolution: Resolution::Four,
                start_height_km: 0.0,
                cell_height_km: 10.0,           // 10km thick cells
                material_name: "basalt".to_string(),
                column_count: 5,                // 5 cells per column (50km total)
                planet_radius_km: 6371.0,
            },
            // Upper mantle layer (50-150km)
            LayerSetParams {
                resolution: Resolution::Four,
                start_height_km: 50.0,
                cell_height_km: 20.0,           // 20km thick cells
                material_name: "basalt".to_string(),
                column_count: 5,                // 5 cells per column (100km total)
                planet_radius_km: 6371.0,
            },
            // Lower mantle layer (150-350km)
            LayerSetParams {
                resolution: Resolution::Four,
                start_height_km: 150.0,
                cell_height_km: 40.0,           // 40km thick cells
                material_name: "basalt".to_string(),
                column_count: 5,                // 5 cells per column (200km total)
                planet_radius_km: 6371.0,
            },
        ];

        SimulationConfig {
            steps: 100,                         // 100 steps for testing
            years_per_step: 1000.0,            // 1000 years per step
            warmup_steps: 0,
            layer_set_params: layer_params,
            thermal_config,
        }
    }

    /// Calculate convection metrics for a layer set
    #[derive(Debug, Clone)]
    struct LayerConvectionMetrics {
        layer_set_index: usize,
        total_energy_joules: f64,
        average_temperature_k: f64,
        temperature_variance: f64,
        energy_variance: f64,
        max_temperature_k: f64,
        min_temperature_k: f64,
    }

    fn calculate_layer_metrics(sim: &Simulation) -> Vec<LayerConvectionMetrics> {
        let mut metrics = Vec::new();

        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            let mut temperatures = Vec::new();
            let mut energies = Vec::new();

            // Collect all cell data
            for column in layer_set.layers.values() {
                for cell in &column.cells {
                    let temp = cell.temperature_kelvin();
                    let energy = cell.energy_joules();
                    temperatures.push(temp);
                    energies.push(energy);
                }
            }

            if !temperatures.is_empty() {
                let total_energy: f64 = energies.iter().sum();
                let avg_temp: f64 = temperatures.iter().sum::<f64>() / temperatures.len() as f64;
                let avg_energy: f64 = energies.iter().sum::<f64>() / energies.len() as f64;

                // Calculate variance
                let temp_variance: f64 = temperatures.iter()
                    .map(|&t| (t - avg_temp).powi(2))
                    .sum::<f64>() / temperatures.len() as f64;

                let energy_variance: f64 = energies.iter()
                    .map(|&e| (e - avg_energy).powi(2))
                    .sum::<f64>() / energies.len() as f64;

                let max_temp = temperatures.iter().fold(0.0f64, |acc, &x| acc.max(x));
                let min_temp = temperatures.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));

                metrics.push(LayerConvectionMetrics {
                    layer_set_index: layer_idx,
                    total_energy_joules: total_energy,
                    average_temperature_k: avg_temp,
                    temperature_variance: temp_variance,
                    energy_variance: energy_variance,
                    max_temperature_k: max_temp,
                    min_temperature_k: min_temp,
                });
            }
        }

        metrics
    }

    #[test]
    fn test_simulation_with_and_without_convection() {
        println!("\n🧪 Testing Simulation With and Without Convection");
        println!("=================================================");

        let config = create_test_simulation_config();

        // Test 1: Simulation WITHOUT convection
        println!("\n📊 Running simulation WITHOUT convection...");
        let mut components_no_convection: Vec<Box<dyn SimComponent>> = vec![];
        let mut sim_no_convection = Simulation::new(config.clone(), &mut components_no_convection);
        sim_no_convection.initialize();

        let initial_metrics_no_conv = calculate_layer_metrics(&sim_no_convection);
        
        // Run simulation
        for step in 0..config.steps {
            sim_no_convection.step();
        }

        let final_metrics_no_conv = calculate_layer_metrics(&sim_no_convection);

        // Test 2: Simulation WITH convection
        println!("\n🌋 Running simulation WITH convection...");
        let mut components_with_convection: Vec<Box<dyn SimComponent>> = vec![
            Box::new(ConvectionPlumeComponent::with_seed(12345)),
        ];
        let mut sim_with_convection = Simulation::new(config.clone(), &mut components_with_convection);
        sim_with_convection.initialize();

        let initial_metrics_with_conv = calculate_layer_metrics(&sim_with_convection);

        // Run simulation
        for step in 0..config.steps {
            sim_with_convection.step();
        }

        let final_metrics_with_conv = calculate_layer_metrics(&sim_with_convection);

        // Analysis and reporting
        println!("\n📈 CONVECTION ANALYSIS RESULTS");
        println!("==============================");

        for layer_idx in 0..config.layer_set_params.len() {
            println!("\n🌍 Layer Set {} Analysis:", layer_idx);
            
            if let (Some(no_conv), Some(with_conv)) = (
                final_metrics_no_conv.get(layer_idx),
                final_metrics_with_conv.get(layer_idx)
            ) {
                println!("   WITHOUT Convection:");
                println!("     - Avg Temperature: {:.1}K", no_conv.average_temperature_k);
                println!("     - Temp Variance: {:.1}K²", no_conv.temperature_variance);
                println!("     - Temp Range: {:.1}K - {:.1}K", no_conv.min_temperature_k, no_conv.max_temperature_k);
                println!("     - Total Energy: {:.2e}J", no_conv.total_energy_joules);

                println!("   WITH Convection:");
                println!("     - Avg Temperature: {:.1}K", with_conv.average_temperature_k);
                println!("     - Temp Variance: {:.1}K²", with_conv.temperature_variance);
                println!("     - Temp Range: {:.1}K - {:.1}K", with_conv.min_temperature_k, with_conv.max_temperature_k);
                println!("     - Total Energy: {:.2e}J", with_conv.total_energy_joules);

                // Calculate convection effects
                let temp_variance_ratio = with_conv.temperature_variance / no_conv.temperature_variance.max(1.0);
                let energy_variance_ratio = with_conv.energy_variance / no_conv.energy_variance.max(1.0);
                let avg_temp_change = with_conv.average_temperature_k - no_conv.average_temperature_k;

                println!("   CONVECTION EFFECTS:");
                println!("     - Temperature variance ratio: {:.2}x", temp_variance_ratio);
                println!("     - Energy variance ratio: {:.2}x", energy_variance_ratio);
                println!("     - Average temperature change: {:.1}K", avg_temp_change);

                // Assertions to verify convection is working
                if layer_idx > 0 { // Deeper layers should show more convection effects
                    assert!(temp_variance_ratio > 1.1, 
                        "Layer {} should show increased temperature variance with convection", layer_idx);
                }
            }
        }

        println!("\n✅ Convection simulation test completed!");
    }
}
