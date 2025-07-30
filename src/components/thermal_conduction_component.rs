use crate::binary_pair::{BinaryPair, BinaryPairId, BinaryPairType};
use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::material::material::MaterialPhases;
use crate::material::materials_loader::MaterialsLoader;
use crate::simulation::Component;
use crate::simulation::GeologicalCellData;
use std::sync::Arc;

/// Thermal Conduction Component - implements heat transfer between neighboring cells
/// Uses binary pairs for efficient thermal conduction calculations
/// Based on Fourier's law: q = -k * A * (dT/dx)
pub struct ThermalConductionComponent {
    /// Default thermal conductivity for unknown materials (W/m/K)
    pub default_conductivity: f64,
    /// Time step scaling factor for stability
    pub time_step_factor: f64,
}

impl ThermalConductionComponent {
    pub fn new() -> Self {
        Self {
            default_conductivity: 3.0, // Typical rock conductivity W/m/K
            time_step_factor: 0.1,     // Conservative time step for stability
        }
    }
    
    pub fn with_conductivity(conductivity: f64) -> Self {
        Self {
            default_conductivity: conductivity,
            time_step_factor: 0.1,
        }
    }
    
    /// Calculate thermal conductivity for a cell based on its material and conditions
    fn get_thermal_conductivity(&self, cell_data: &GeologicalCellData, location: &CellLocation) -> f64 {
        // Get material name based on layer (simplified)
        let material_name = match location.layer_set_index() {
            0 => "granite",    // Crust
            1 => "basalt",     // Upper mantle
            2 => "peridotite", // Lower mantle
            _ => "iron",       // Core
        };
        
        // Get material thermal conductivity
        if let Ok(material) = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid) {
            // Thermal conductivity varies with temperature and pressure
            let base_conductivity = material.thermal_conductivity_w_per_m_k as f64;
            
            // Temperature effect: conductivity decreases with temperature
            let temp_factor = 1.0 - (cell_data.temperature_k - 273.15) / 2000.0; // Decrease ~50% at 2000K
            
            // Pressure effect: conductivity increases slightly with pressure
            let pressure_factor = 1.0 + (cell_data.pressure_pa - 101325.0) / 1e9; // Small increase with pressure
            
            (base_conductivity * temp_factor * pressure_factor).max(0.1) // Minimum conductivity
        } else {
            self.default_conductivity
        }
    }
    
    /// Calculate heat transfer between two cells via a binary pair
    fn calculate_heat_transfer(&self, 
                              pair: &BinaryPair, 
                              cell_a_data: &GeologicalCellData,
                              cell_b_data: &GeologicalCellData,
                              cell_a_location: &CellLocation,
                              cell_b_location: &CellLocation,
                              time_step_years: f64) -> (f64, f64) {
        
        // Get thermal conductivities for both cells
        let k_a = self.get_thermal_conductivity(cell_a_data, cell_a_location);
        let k_b = self.get_thermal_conductivity(cell_b_data, cell_b_location);
        
        // Use harmonic mean for interface conductivity
        let k_interface = 2.0 * k_a * k_b / (k_a + k_b);
        
        // Calculate thermal conductance for this pair
        let conductance = pair.thermal_conductance(k_interface); // W/K
        
        // Temperature difference
        let temp_diff = cell_b_data.temperature_k - cell_a_data.temperature_k; // K
        
        // Heat flow rate (W)
        let heat_flow_rate = conductance * temp_diff;
        
        // Convert time step from years to seconds
        let time_step_seconds = time_step_years * 365.25 * 24.0 * 3600.0;
        
        // Total energy transfer (J)
        let energy_transfer = heat_flow_rate * time_step_seconds * self.time_step_factor;
        
        // Energy change for each cell (positive = gaining energy)
        let energy_change_a = energy_transfer;  // Cell A gains energy
        let energy_change_b = -energy_transfer; // Cell B loses energy
        
        (energy_change_a, energy_change_b)
    }
}

impl Component for ThermalConductionComponent {
    fn name(&self) -> &'static str {
        "ThermalConductionComponent"
    }
    
    fn initialize(&mut self, sim: &mut crate::simulation::Simulation) {
        println!("🔥 Thermal Conduction Component initialized");
        println!("   - Default conductivity: {:.1} W/m/K", self.default_conductivity);
        println!("   - Time step factor: {:.2}", self.time_step_factor);
        
        let cells_count = sim.get_geological_cells().len();
        println!("   - Total cells: {}", cells_count);
    }
    
    fn step(&self, coll_mgr: Arc<CollectionsManager>, actor: &mut Actor, _step: u32, _year: f64) {
        // Get binary pairs for thermal conduction
        let binary_pairs = match coll_mgr.get::<BinaryPairId, BinaryPair>("binary_pairs") {
            Some(pairs) => pairs,
            None => {
                println!("    ThermalConductionComponent: No binary pairs found, skipping");
                return;
            }
        };
        
        // Get geological cells
        let geological_cells = match coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells") {
            Some(cells) => cells,
            None => {
                println!("    ThermalConductionComponent: No geological cells found, skipping");
                return;
            }
        };
        
        println!("    ThermalConductionComponent: Processing {} binary pairs for heat transfer", 
                 binary_pairs.len());
        
        let time_step_years = coll_mgr.config.years_per_step as f64;
        let mut heat_transfers = 0;
        
        // Process each binary pair for thermal conduction
        for entry in binary_pairs.iter() {
            let pair = entry.value();
            let (cell_a_loc, cell_b_loc) = pair.get_cells();
            
            // Get cell data for both cells
            if let (Some(cell_a_data), Some(cell_b_data)) = (
                geological_cells.get(&cell_a_loc),
                geological_cells.get(&cell_b_loc)
            ) {
                // Calculate heat transfer between the cells
                let (energy_change_a, energy_change_b) = self.calculate_heat_transfer(
                    pair, 
                    cell_a_data, 
                    cell_b_data, 
                    &cell_a_loc, 
                    &cell_b_loc, 
                    time_step_years
                );
                
                // Apply energy changes if significant
                if energy_change_a.abs() > 1e6 { // Minimum 1 MJ threshold
                    actor.add("geological_cells", cell_a_loc, "energy_joules", energy_change_a);
                    actor.add("geological_cells", cell_b_loc, "energy_joules", energy_change_b);
                    heat_transfers += 1;
                }
            }
        }
        
        println!("    ThermalConductionComponent: Applied {} significant heat transfers", heat_transfers);
    }
    
    fn complete(&mut self, sim: &crate::simulation::Simulation) {
        println!("🔥 Thermal Conduction Component completed");
        
        // Calculate some thermal statistics
        let cells = sim.get_geological_cells();
        let mut total_energy = 0.0;
        let mut min_temp = f64::INFINITY;
        let mut max_temp = 0.0;
        
        for entry in cells.iter() {
            let cell_data = entry.value();
            total_energy += cell_data.energy_mass.energy_joules();
            min_temp = min_temp.min(cell_data.temperature_k);
            max_temp = max_temp.max(cell_data.temperature_k);
        }
        
        println!("   - Total thermal energy: {:.2e} J", total_energy);
        println!("   - Temperature range: {:.1}K to {:.1}K ({:.1}°C to {:.1}°C)", 
                 min_temp, max_temp, min_temp - 273.15, max_temp - 273.15);
        println!("   - Default conductivity used: {:.1} W/m/K", self.default_conductivity);
    }
}

impl Default for ThermalConductionComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy_mass::EnergyMass;
    use h3o::LatLng;

    #[test]
    fn test_thermal_conduction_component_creation() {
        let component = ThermalConductionComponent::new();
        
        assert_eq!(component.name(), "ThermalConductionComponent");
        assert_eq!(component.default_conductivity, 3.0);
        assert_eq!(component.time_step_factor, 0.1);
    }
    
    #[test]
    fn test_thermal_conductivity_calculation() {
        let component = ThermalConductionComponent::new();
        
        let cell_data = GeologicalCellData {
            energy_mass: EnergyMass::new(1000.0, 1000.0),
            temperature_k: 300.0,
            pressure_pa: 101325.0,
            density_kg_m3: 2500.0,
        };
        
        let location = CellLocation::new(
            0, 
            LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two),
            0
        );
        
        let conductivity = component.get_thermal_conductivity(&cell_data, &location);
        
        // Should return a reasonable conductivity value
        assert!(conductivity > 0.0);
        assert!(conductivity < 100.0); // Reasonable upper bound
    }
    
    #[test]
    fn test_heat_transfer_calculation() {
        let component = ThermalConductionComponent::new();
        
        // Create two cells with different temperatures
        let hot_cell = GeologicalCellData {
            energy_mass: EnergyMass::new(1000.0, 1000.0),
            temperature_k: 400.0, // Hot cell
            pressure_pa: 101325.0,
            density_kg_m3: 2500.0,
        };
        
        let cold_cell = GeologicalCellData {
            energy_mass: EnergyMass::new(1000.0, 1000.0),
            temperature_k: 300.0, // Cold cell
            pressure_pa: 101325.0,
            density_kg_m3: 2500.0,
        };
        
        let h3_cell_a = LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two);
        let h3_cell_b = LatLng::new(0.1, 0.0).unwrap().to_cell(h3o::Resolution::Two);
        
        let location_a = CellLocation::new(0, h3_cell_a, 0);
        let location_b = CellLocation::new(0, h3_cell_b, 0);
        
        // Create a binary pair
        let pair = BinaryPair::horizontal(location_a, location_b, 10.0, 100.0);
        
        let (energy_change_a, energy_change_b) = component.calculate_heat_transfer(
            &pair, &hot_cell, &cold_cell, &location_a, &location_b, 1000.0 // 1000 years
        );
        
        // Hot cell should lose energy (negative), cold cell should gain energy (positive)
        assert!(energy_change_a > 0.0); // Cell A (cold) gains energy
        assert!(energy_change_b < 0.0); // Cell B (hot) loses energy
        assert!((energy_change_a + energy_change_b).abs() < 1e-6); // Energy conservation
    }
}
