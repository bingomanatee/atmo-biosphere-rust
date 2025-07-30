pub mod thermal_component;
pub mod pressure_component;
pub mod density_component;
pub mod layer_cell_component;
pub mod binary_pair_component;

pub use thermal_component::ThermalComponent;
pub use pressure_component::PressureComponent;
pub use density_component::DensityComponent;
pub use layer_cell_component::LayerCellComponent;
pub use binary_pair_component::BinaryPairComponent;

#[cfg(test)]
mod component_tests {
    use crate::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
    use crate::components::LayerCellComponent;
    use h3o::Resolution;

    #[test]
    fn test_component_lifecycle() {
        println!("\n🔧 Component Lifecycle Test");

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

        println!("✅ Simulation created with {} cells", sim.get_geological_cells().len());

        // Test component lifecycle
        println!("\n🔧 Testing Component Lifecycle:");

        // Add a component
        println!("1. Adding LayerCellComponent...");
        sim.add_component(Box::new(LayerCellComponent::new()));
        println!("   ✅ Component added");

        // Test initialize phase
        println!("\n2. Testing initialize phase...");
        sim.initialize_components();
        println!("   ✅ Initialize phase completed");

        // Test step phase
        println!("\n3. Testing step phase...");
        sim.step();
        println!("   ✅ Step phase completed");

        // Test complete phase
        println!("\n4. Testing complete phase...");
        sim.complete_components();
        println!("   ✅ Complete phase completed");

        // Show final stats
        let stats = sim.get_stats();
        println!("\n📊 Final Results:");
        println!("  Components: {}", sim.components.len());
        println!("  Steps completed: {}", stats.current_step);
        println!("  Total cells: {}", stats.total_cells);
        println!("  Years simulated: {}", stats.years_simulated);

        // Assertions
        assert_eq!(sim.components.len(), 1);
        assert_eq!(stats.current_step, 1);
        assert!(stats.total_cells > 0);
        assert_eq!(stats.years_simulated, 1000);

        println!("\n🎉 Component structure test completed successfully!");
        println!("✅ All lifecycle phases (initialize → step → complete) working");
    }
}
