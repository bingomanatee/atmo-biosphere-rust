use crate::simulation::Component;
use crate::collections::{Actor, CollectionsManager};
use crate::cell_location::CellLocation;
use crate::simulation::GeologicalCellData;

/// Pressure component - simple processing (no sub-chunking)
pub struct PressureComponent;

impl PressureComponent {
    pub fn new() -> Self {
        Self
    }
    
    fn calculate_pressure_change(&self, cell_data: &GeologicalCellData, location: &CellLocation) -> f64 {
        // Pressure changes based on depth and temperature
        let depth_factor = (location.depth_index() + 1) as f64;
        let temp_factor = cell_data.temperature_k / 300.0; // Normalized temperature
        depth_factor * 1000.0 * temp_factor
    }
}

impl Component for PressureComponent {
    fn name(&self) -> &'static str {
        "PressureComponent"
    }

    fn initialize(&mut self, sim: &mut crate::simulation::Simulation) {
        println!("💨 Pressure Component initialized");
        println!("   - Total cells: {}", sim.get_geological_cells().len());
    }

    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, _step: u32, _year: f64) {
        let cells = coll_mgr.get::<crate::cell_location::CellLocation, crate::simulation::GeologicalCellData>("geological_cells")
            .expect("geological_cells collection should exist");
        
        println!("    PressureComponent: Processing {} cells directly", cells.len());
        
        for entry in cells.iter() {
            let (location, cell_data) = (entry.key(), entry.value());
            
            let pressure_delta = self.calculate_pressure_change(cell_data, location);
            
            actor.add("GEOLOGICAL_CELLS", *location, "pressure_pa", pressure_delta);
        }
    }

    fn complete(&mut self, sim: &crate::simulation::Simulation) {
        println!("💨 Pressure Component completed");
        println!("   - Final total cells: {}", sim.get_geological_cells().len());
    }
}
