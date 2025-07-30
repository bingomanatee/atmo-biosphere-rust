use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::simulation::{Component, GeologicalCellData, Simulation, SimulationConfig};


/// Thermal component that can sub-chunk large cell counts
pub struct ThermalComponent {
    pub chunk_threshold: usize, // Sub-chunk if more than this many cells
}

impl ThermalComponent {
    pub fn new() -> Self {
        Self {
            chunk_threshold: 1000, // Default threshold
        }
    }
    
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            chunk_threshold: threshold,
        }
    }
    
    fn calculate_thermal_change(&self, cell_data: &GeologicalCellData, location: &CellLocation) -> f64 {
        // Simple thermal calculation
        let base_heating = if location.layer_set_index() == 0 {
            // Surface layer: solar heating
            5.0
        } else {
            // Deeper layers: geothermal heating
            2.0 + location.depth_index() as f64 * 0.5
        };
        
        // Temperature-dependent factor
        let temp_factor = 1.0 - (cell_data.temperature_k - 300.0) / 1000.0;
        base_heating * temp_factor.max(0.1)
    }
}

impl Component for ThermalComponent {
    fn name(&self) -> &'static str {
        "ThermalComponent"
    }

    fn initialize(&mut self, sim: &mut Simulation, _config: &SimulationConfig) {
        println!("🔥 Thermal Component initialized");
        println!("   - Chunk threshold: {} cells", self.chunk_threshold);
        println!("   - Total cells: {}", sim.get_geological_cells().len());
    }

    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, _step: u32, _year: f64, _config: &SimulationConfig) {
        let cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
            .expect("geological_cells collection should exist");
        let cell_count = cells.len();
        
        if cell_count > self.chunk_threshold {
            println!("    ThermalComponent: Sub-chunking {} cells (threshold: {})", 
                     cell_count, self.chunk_threshold);
            
            // Sub-chunk for large cell counts
            let cell_pairs: Vec<_> = cells.iter().collect();
            let _chunk_size = (cell_count / num_cpus::get()).max(1); // For future sub-chunking
            

            
            // For now, process directly even with many cells
            // TODO: Add sub-chunking with crossbeam scope passed from simulation
            for entry in cell_pairs {
                let (location, cell_data) = (entry.key(), entry.value());

                let temp_delta = self.calculate_thermal_change(cell_data, location);
                let energy_delta = temp_delta * 1000.0;

                actor.add("GEOLOGICAL_CELLS", *location, "temperature_k", temp_delta);
                actor.add("GEOLOGICAL_CELLS", *location, "energy_joules", energy_delta);
            }

            
        } else {
            println!("    ThermalComponent: Processing {} cells directly (below threshold)", cell_count);
            
            // Simple processing for small cell counts
            for entry in cells.iter() {
                let (location, cell_data) = (entry.key(), entry.value());
                
                let temp_delta = self.calculate_thermal_change(cell_data, location);
                let energy_delta = temp_delta * 1000.0;
                
                actor.add("GEOLOGICAL_CELLS", *location, "temperature_k", temp_delta);
                actor.add("GEOLOGICAL_CELLS", *location, "energy_joules", energy_delta);
            }
        }
    }

    fn complete(&mut self, sim: &Simulation, _config: &SimulationConfig) {
        println!("🔥 Thermal Component completed");
        println!("   - Final total cells: {}", sim.get_geological_cells().len());
        println!("   - Chunk threshold was: {} cells", self.chunk_threshold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy_mass::EnergyMass;
    use h3o::Resolution;

    #[test]
    fn test_thermal_component_creation() {
        // Simple unit test - no deep simulation initialization
        let thermal = ThermalComponent::new();
        assert_eq!(thermal.chunk_threshold, 1000);
        assert_eq!(thermal.name(), "ThermalComponent");

        let thermal_custom = ThermalComponent::with_threshold(500);
        assert_eq!(thermal_custom.chunk_threshold, 500);
        assert_eq!(thermal_custom.name(), "ThermalComponent");
    }
    
    #[test]
    fn test_thermal_calculation() {
        let thermal = ThermalComponent::new();

        // Test thermal calculation logic
        let cell_data = GeologicalCellData {
            energy_mass: EnergyMass::new(1000.0, 1000.0),
            temperature_k: 300.0,
            pressure_pa: 101325.0,
            density_kg_m3: 2500.0,
        };

        let location = CellLocation::new(
            0,
            h3o::LatLng::new(0.0, 0.0).unwrap().to_cell(Resolution::Two),
            0
        );

        let temp_delta = thermal.calculate_thermal_change(&cell_data, &location);

        // Should return some positive heating for surface layer
        assert!(temp_delta > 0.0);
        assert!(temp_delta < 10.0); // Reasonable range
    }
}
