pub mod thermal_component;
pub mod pressure_component;
pub mod density_component;
pub mod layer_cell_component;
pub mod binary_pair_component;
pub mod metrics_reporting_component;
pub mod parallel_radiance_component;
pub mod radiance_component;
pub mod thermal_conduction_component;
pub mod vertical_radiance_component;
pub mod column_radiance_component;
pub mod column_plume_component;

pub use binary_pair_component::BinaryPairComponent;
pub use column_radiance_component::ColumnRadianceComponent;
pub use column_plume_component::ColumnPlumeComponent;
pub use density_component::DensityComponent;
pub use layer_cell_component::LayerCellComponent;
pub use metrics_reporting_component::MetricsReportingComponent;
pub use parallel_radiance_component::ParallelRadianceComponent;
pub use pressure_component::PressureComponent;
pub use radiance_component::RadianceComponent;
pub use thermal_component::ThermalComponent;
pub use thermal_conduction_component::ThermalConductionComponent;
pub use vertical_radiance_component::VerticalRadianceComponent;

#[cfg(test)]
mod component_tests {
    use crate::components::LayerCellComponent;
    use crate::simulation::{LayerConfig, PlanetConfig, Simulation, SimulationConfig};
    use h3o::Resolution;

    #[test]
    fn test_component_lifecycle() {

        // Create a minimal simulation for testing component structure
        let config = SimulationConfig {
            planet: PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
                surface_temperature_k: 288.15,
            },
            years_per_step: 1000,
            steps: 2,
            layers: vec![
                LayerConfig {
                    height_per_step_km: 10.0,
                    depth_steps: 1,  // Minimal for testing
                    resolution: Resolution::Three, // Coarse for testing
                    name: "Test Layer".to_string(),
                    temperature_gradient_k_per_km: 25.0,
                },
            ],
        };

        let mut sim = Simulation::new(config);
        sim.initialize_cells();

        // Test component lifecycle
        sim.add_component(Box::new(LayerCellComponent::new()));
        sim.initialize_components();
        sim.step();
        sim.complete_components();

        // Verify final stats
        let stats = sim.get_stats();

        // Assertions
        assert_eq!(sim.components.len(), 1);
        assert_eq!(stats.current_step, 1);
        assert!(stats.total_cells > 0);
        assert_eq!(stats.years_simulated, 1000);
    }
}
