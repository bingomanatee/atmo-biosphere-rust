pub mod thermal_component;
pub mod pressure_component;
pub mod density_component;
pub mod layer_cell_component;
pub mod binary_pair_component;

pub use binary_pair_component::BinaryPairComponent;
pub use density_component::DensityComponent;
pub use layer_cell_component::LayerCellComponent;
pub use pressure_component::PressureComponent;
pub use thermal_component::ThermalComponent;

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
            },
            years_per_step: 1000,
            steps: 2,
            layers: vec![
                LayerConfig {
                    height_per_step_km: 10.0,
                    depth_steps: 1,  // Minimal for testing
                    resolution: Resolution::Three, // Coarse for testing
                    name: "Test Layer".to_string(),
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
