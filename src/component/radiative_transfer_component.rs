use crate::component::SimComponent;
use crate::sim_immut::simulation_immut::SimulationImmut;
use crate::sim_immut::radiative_transfer::RadiativeTransferConfig;
use crate::transaction_manager_simple::{SimpleTransactionManager, CellLocation};
use crate::energy_mass::energy_mass::EnergyMass;

/// Radiative Transfer Component - handles heat transfer between neighboring cells
/// This should be a component, not built into the simulation engine
#[derive(Debug, Clone)]
pub struct RadiativeTransferComponent {
    /// Configuration for radiative transfer
    config: RadiativeTransferConfig,
    /// Performance tracking
    total_energy_transferred: f64,
    total_transactions: u64,
}

impl RadiativeTransferComponent {
    /// Create new radiative transfer component with default configuration
    pub fn new() -> Self {
        Self {
            config: RadiativeTransferConfig::default(),
            total_energy_transferred: 0.0,
            total_transactions: 0,
        }
    }
    
    /// Get component name
    pub fn name(&self) -> &str {
        "RadiativeTransferComponent"
    }

    /// Create with custom configuration
    pub fn new_with_config(config: RadiativeTransferConfig) -> Self {
        Self {
            config,
            total_energy_transferred: 0.0,
            total_transactions: 0,
        }
    }
    
    /// Calculate heat transfer between two cells
    pub fn calculate_heat_transfer(&self, temp1: f64, temp2: f64, thermal_conductivity: f64,
                              distance: f64, contact_area: f64, time_step_years: f64) -> f64 {
        // Heat transfer equation: Q = k * A * (T1 - T2) / d * t
        // Where k = thermal conductivity, A = contact area, d = distance, t = time
        
        let temp_difference = temp1 - temp2;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = time_step_years * seconds_per_year;
        
        thermal_conductivity * contact_area * temp_difference / distance * time_step_seconds
    }
    
    /// Process radiative transfer between all neighboring cells
    fn process_radiative_transfer(&mut self, sim: &SimulationImmut, 
                                 simple_manager: &mut SimpleTransactionManager) {
        let mut transaction_count = 0;
        let mut total_energy_transferred = 0.0;
        
        // Process horizontal heat transfer (between adjacent H3 cells)
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                // Get neighbors of this H3 cell
                let neighbors = h3_cell.grid_disk::<Vec<_>>(1);
                
                for neighbor_h3 in neighbors {
                    if let Some(neighbor_column) = layer_set.layers.get(&neighbor_h3) {
                        // Transfer heat between corresponding cells in columns
                        for (cell_idx, cell) in column.cells.iter().enumerate() {
                            if let Some(neighbor_cell) = neighbor_column.cells.get(cell_idx) {
                                let heat_transfer = self.calculate_heat_transfer(
                                    cell.temperature_kelvin(),
                                    neighbor_cell.temperature_kelvin(),
                                    2.5, // Thermal conductivity (W/m·K)
                                    60000.0, // Distance between H3 cells (~60km)
                                    1e9, // Contact area (m²)
                                    sim.config.years_per_step
                                );
                                
                                if heat_transfer.abs() > 1e15 { // Minimum energy threshold
                                    // Create transactions for energy transfer
                                    let source_location = CellLocation {
                                        layer_set_index: layer_set_idx,
                                        h3_cell: *h3_cell,
                                        cell_index: cell_idx,
                                    };
                                    
                                    let target_location = CellLocation {
                                        layer_set_index: layer_set_idx,
                                        h3_cell: neighbor_h3,
                                        cell_index: cell_idx,
                                    };
                                    
                                    // Energy flows from hot to cold
                                    simple_manager.add_energy_delta(source_location, -heat_transfer, "radiative_transfer");
                                    simple_manager.add_energy_delta(target_location, heat_transfer, "radiative_transfer");
                                    
                                    transaction_count += 2;
                                    total_energy_transferred += heat_transfer.abs();
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Process vertical heat transfer (between cells in same column)
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for cell_idx in 0..column.cells.len().saturating_sub(1) {
                    let upper_cell = &column.cells[cell_idx];
                    let lower_cell = &column.cells[cell_idx + 1];
                    
                    let heat_transfer = self.calculate_heat_transfer(
                        lower_cell.temperature_kelvin(),
                        upper_cell.temperature_kelvin(),
                        3.0, // Vertical thermal conductivity
                        10000.0, // Vertical distance (~10km)
                        3.6e9, // Cell area (m²)
                        sim.config.years_per_step
                    );
                    
                    if heat_transfer.abs() > 1e15 {
                        let upper_location = CellLocation {
                            layer_set_index: layer_set_idx,
                            h3_cell: *h3_cell,
                            cell_index: cell_idx,
                        };
                        
                        let lower_location = CellLocation {
                            layer_set_index: layer_set_idx,
                            h3_cell: *h3_cell,
                            cell_index: cell_idx + 1,
                        };
                        
                        simple_manager.add_energy_delta(lower_location, -heat_transfer, "radiative_transfer");
                        simple_manager.add_energy_delta(upper_location, heat_transfer, "radiative_transfer");
                        
                        transaction_count += 2;
                        total_energy_transferred += heat_transfer.abs();
                    }
                }
            }
        }
        
        // Process surface-to-space heat transfer (surface cooling)
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            if layer_set_idx == 0 { // Only surface layer
                for (h3_cell, column) in &layer_set.layers {
                    if let Some(surface_cell) = column.cells.first() {
                        // Stefan-Boltzmann radiation to space
                        let stefan_boltzmann = 5.670374419e-8; // W/m²·K⁴
                        let emissivity = 0.95;
                        let surface_temp = surface_cell.temperature_kelvin();
                        let space_temp = 2.7_f64; // Cosmic background temperature
                        
                        let radiated_power = stefan_boltzmann * emissivity * 
                            (surface_temp.powi(4) - space_temp.powi(4)); // W/m²
                        
                        let cell_area = 3.6e9; // m²
                        let seconds_per_year = 365.25 * 24.0 * 3600.0;
                        let energy_loss = radiated_power * cell_area * 
                            sim.config.years_per_step * seconds_per_year; // Joules
                        
                        if energy_loss > 1e15 {
                            let surface_location = CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell: *h3_cell,
                                cell_index: 0,
                            };
                            
                            simple_manager.add_energy_delta(surface_location, -energy_loss, "surface_radiation");
                            transaction_count += 1;
                            total_energy_transferred += energy_loss;
                        }
                    }
                }
            }
        }
        
        // Update component statistics
        self.total_energy_transferred += total_energy_transferred;
        self.total_transactions += transaction_count;
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (f64, u64) {
        (self.total_energy_transferred, self.total_transactions)
    }
}

impl Default for RadiativeTransferComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl SimComponent for RadiativeTransferComponent {
    fn key(&self) -> &'static str {
        "RadiativeTransferComponent"
    }
    
    fn initialize(&mut self, _sim: &mut SimulationImmut) {
        println!("🌡️ Radiative Transfer Component initialized");
        println!("   - Horizontal heat transfer: H3 neighbor cells");
        println!("   - Vertical heat transfer: Column cells");
        println!("   - Surface radiation: Stefan-Boltzmann to space");
    }
    
    fn step(&mut self, sim: &mut SimulationImmut, _step: i64, _year: i64) {
        // This is where we would integrate with the simple transaction system
        // For now, we'll use the existing built-in radiative transfer
        // TODO: Replace built-in system with this component

        // Note: We need access to SimpleTransactionManager here
        // This requires updating the SimComponent trait or simulation architecture
    }

    fn complete(&mut self, _sim: &SimulationImmut) {
        println!("🌡️ Radiative Transfer Component completed");
        println!("   - Total energy transferred: {:.2e} J", self.total_energy_transferred);
        println!("   - Total transactions: {}", self.total_transactions);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
    use crate::sim_immut::layer_set_immut::default_layer_set_params_immut;
    use h3o::Resolution;
    
    #[test]
    fn test_radiative_transfer_component_creation() {
        let component = RadiativeTransferComponent::new();
        assert_eq!(component.name(), "RadiativeTransferComponent");
        assert_eq!(component.total_energy_transferred, 0.0);
        assert_eq!(component.total_transactions, 0);
    }
    
    #[test]
    fn test_heat_transfer_calculation() {
        let component = RadiativeTransferComponent::new();
        
        // Test heat transfer between two cells
        let heat_transfer = component.calculate_heat_transfer(
            400.0, // Hot cell (400K)
            300.0, // Cold cell (300K)
            2.5,   // Thermal conductivity
            60000.0, // Distance (60km)
            1e9,   // Contact area
            1000.0 // Time step (1000 years)
        );
        
        assert!(heat_transfer > 0.0, "Heat should flow from hot to cold");
        println!("Heat transfer: {:.2e} J", heat_transfer);
    }
}
