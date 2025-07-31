use crate::binary_pair_builder::BinaryPairBuilder;
use crate::collections::{Actor, CollectionsManager};
use crate::simulation::{Component, Simulation, SimulationConfig};

/// Binary Pair Component - builds and manages binary associations between cells
/// Creates vertical pairs (above/below) and horizontal pairs (H3 neighbors)
/// Based on the deprecated binary pairing system for efficient geological operations
pub struct BinaryPairComponent {
    pub builder: BinaryPairBuilder,
    pub pairs_built: bool,
    pub total_pairs: usize,
}

impl BinaryPairComponent {
    pub fn new() -> Self {
        Self {
            builder: BinaryPairBuilder::new(),
            pairs_built: false,
            total_pairs: 0,
        }
    }
}

impl Component for BinaryPairComponent {
    fn name(&self) -> &'static str {
        "BinaryPairComponent"
    }
    
    fn initialize(&mut self, coll_mgr: &mut CollectionsManager, config: &SimulationConfig) {
        // Build all binary pairs during initialization
        println!("🔗 BinaryPairComponent: Building binary pairs...");
        match self.builder.build_all_pairs(coll_mgr, &config.layers) {
            Ok(total_pairs) => {
                self.total_pairs = total_pairs;
                self.pairs_built = true;
                println!("   ✅ Built {} binary pairs successfully", total_pairs);
            }
            Err(e) => {
                self.pairs_built = false;
                println!("   ❌ Failed to build binary pairs: {:?}", e);
            }
        }
    }

    fn step(&self, _coll_mgr: &CollectionsManager, _actor: &mut Actor, _step: u32, _year: f64, _config: &SimulationConfig) {
        // Binary pairs are static - no processing needed during steps
        // They were built during initialization
    }
    
    fn complete(&mut self, _sim: &Simulation, _config: &SimulationConfig) {
        // Component cleanup - no output needed
    }
}

impl Default for BinaryPairComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{LayerConfig, PlanetConfig, Simulation, SimulationConfig};
    use h3o::Resolution;

    #[test]
    fn test_binary_pair_component_creation() {
        let component = BinaryPairComponent::new();
        
        assert_eq!(component.name(), "BinaryPairComponent");
        assert!(!component.pairs_built);
        assert_eq!(component.total_pairs, 0);
    }
    
    #[test]
    fn test_binary_pair_component_with_simulation() {
        // Create a small simulation for testing
        let config = SimulationConfig {
            planet: PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
                surface_temperature_k: 288.15,
            },
            years_per_step: 1000,
            steps: 1,
            layers: vec![
                LayerConfig {
                    height_per_step_km: 10.0,
                    depth_steps: 2,  // Small for testing
                    resolution: Resolution::Three, // Coarse for testing
                    name: "Test Crust".to_string(),
                    temperature_gradient_k_per_km: 25.0,
                },
            ],
        };
        
        let mut sim = Simulation::new(config.clone());
        sim.initialize_cells();
        
        // Test component initialization
        let mut component = BinaryPairComponent::new();

        // Initialize cells first
        sim.initialize_cells();

        // Get collections manager and config for proper initialization
        let coll_mgr = &mut sim.coll_mgr;
        component.initialize(coll_mgr, &config);

        // Should have built some pairs
        assert!(component.pairs_built);
        assert!(component.total_pairs > 0);
        
        // Test passed
    }
}
