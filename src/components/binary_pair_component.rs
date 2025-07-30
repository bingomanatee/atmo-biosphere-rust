use crate::simulation::Component;
use crate::collections::Actor;
use crate::binary_pair_builder::BinaryPairBuilder;
use std::sync::Arc;

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
    
    fn initialize(&mut self, sim: &mut crate::simulation::Simulation) {
        println!("🔗 Binary Pair Component initializing...");
        
        // Build all binary pairs during initialization
        match self.builder.build_all_pairs(&mut sim.coll_mgr) {
            Ok(total_pairs) => {
                self.total_pairs = total_pairs;
                self.pairs_built = true;
                println!("✅ Binary Pair Component initialized with {} pairs", total_pairs);
            }
            Err(e) => {
                println!("❌ Failed to build binary pairs: {}", e);
                self.pairs_built = false;
            }
        }
    }
    
    fn step(&self, _coll_mgr: &crate::collections::CollectionsManager, _actor: &mut Actor, step: u32, _year: f64) {
        // Binary pairs are static - no processing needed during steps
        if step == 1 && self.pairs_built {
            println!("🔗 BinaryPairComponent: {} pairs available for geological operations", self.total_pairs);
        }
    }
    
    fn complete(&mut self, _sim: &crate::simulation::Simulation) {
        println!("🔗 Binary Pair Component completed");
        if self.pairs_built {
            println!("   - Total pairs created: {}", self.total_pairs);
            println!("   - Pairs available for thermal/mass transfer operations");
        } else {
            println!("   - No pairs were built (initialization failed)");
        }
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
    use crate::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
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
            },
            years_per_step: 1000,
            steps: 1,
            layers: vec![
                LayerConfig {
                    height_per_step_km: 10.0,
                    depth_steps: 2,  // Small for testing
                    resolution: Resolution::Three, // Coarse for testing
                    name: "Test Crust".to_string(),
                },
            ],
        };
        
        let mut sim = Simulation::new(config);
        sim.initialize_cells();
        
        // Test component initialization
        let mut component = BinaryPairComponent::new();
        component.initialize(&mut sim);
        
        // Should have built some pairs
        assert!(component.pairs_built);
        assert!(component.total_pairs > 0);
        
        println!("Test simulation created {} binary pairs", component.total_pairs);
    }
}
