use crate::simulation::Component;
use crate::collections::{CollectionsManager, Actor};
use crate::cell_location::CellLocation;
use crate::simulation::GeologicalCellData;

/// Density component - uses replace (absolute values) instead of add
pub struct DensityComponent;

impl DensityComponent {
    pub fn new() -> Self {
        Self
    }
    
    fn calculate_new_density(&self, cell_data: &GeologicalCellData) -> f64 {
        // Calculate new density based on temperature and pressure
        let base_density = 2500.0;
        let temp_factor = 1.0 - (cell_data.temperature_k - 300.0) / 10000.0;
        let pressure_factor = 1.0 + (cell_data.pressure_pa - 101325.0) / 1000000.0;
        
        (base_density * temp_factor * pressure_factor).max(1000.0) // Minimum density
    }
}

impl Component for DensityComponent {
    fn process(&self, manager: &CollectionsManager, actor: &mut Actor) {
        let cells = manager.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        
        println!("    DensityComponent: Processing {} cells directly", cells.len());
        
        for entry in cells.iter() {
            let (location, cell_data) = (entry.key(), entry.value());
            
            let new_density = self.calculate_new_density(cell_data);
            
            // Use replace (absolute value) instead of add (delta)
            actor.replace("GEOLOGICAL_CELLS", *location, "density_kg_m3", new_density);
        }
    }
    
    fn name(&self) -> &'static str {
        "DensityComponent"
    }
}
