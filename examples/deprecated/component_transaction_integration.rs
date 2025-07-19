// Example showing how components integrate with the transaction system

use atmo_biosphere_rust::sim::simulation::Simulation;
use atmo_biosphere_rust::component::SimComponent;

/// Example thermal conduction component using transaction gateway
pub struct ThermalConductionComponent {
    pub name: String,
}

impl ThermalConductionComponent {
    pub fn new() -> Self {
        Self {
            name: "ThermalConduction".to_string(),
        }
    }
}

impl SimComponent for ThermalConductionComponent {
    fn step(&mut self, simulation: &mut Simulation, _step: i64, _year: f64) {
        // Example: Thermal conduction between adjacent cells
        
        for (layer_index, layer_set) in simulation.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for depth_index in 0..column.cells.len().saturating_sub(1) {
                    // Get current and next cell
                    if let (Some(current_cell), Some(next_cell)) = (
                        simulation.get_cell(layer_index, *h3_cell, depth_index),
                        simulation.get_cell(layer_index, *h3_cell, depth_index + 1)
                    ) {
                        // Calculate temperature difference
                        let temp_diff = current_cell.temperature_kelvin() - next_cell.temperature_kelvin();
                        
                        if temp_diff.abs() > 1.0 {
                            // Calculate energy transfer based on temperature gradient
                            let energy_transfer = temp_diff * 1e15; // Simple thermal conduction
                            
                            // Use gateway method to propose transaction
                            simulation.propose_mass_transfer(
                                &self.name,
                                layer_index, *h3_cell, depth_index,      // From
                                layer_index, *h3_cell, depth_index + 1,  // To
                                -energy_transfer,  // Energy leaves source
                                0.0,              // No mass transfer
                                &format!("Thermal conduction: {:.1}K gradient", temp_diff),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Example core radiance component using transaction gateway
pub struct CoreRadianceComponent {
    pub name: String,
    pub base_wattage: f64,
}

impl CoreRadianceComponent {
    pub fn new(base_wattage: f64) -> Self {
        Self {
            name: "CoreRadiance".to_string(),
            base_wattage,
        }
    }
}

impl SimComponent for CoreRadianceComponent {
    fn step(&mut self, simulation: &mut Simulation, _step: i64, year: f64) {
        // Exponential cooling: F0 * exp(-t/tau)
        let tau = 3.5e6; // 3.5 million years
        let cooling_factor = (-year / tau).exp();
        let current_wattage = self.base_wattage * cooling_factor;
        
        // Apply to deepest cells in each column
        for (layer_index, layer_set) in simulation.layer_sets.iter().enumerate() {
            // Only apply to deepest layers (layer 3+)
            if layer_index >= 3 {
                for (h3_cell, column) in &layer_set.layers {
                    if let Some(deepest_index) = column.cells.len().checked_sub(1) {
                        // Energy input per cell
                        let energy_input = current_wattage * simulation.years_per_step() * 365.25 * 24.0 * 3600.0;
                        
                        // Use gateway method for energy-only transaction
                        simulation.propose_energy_transaction(
                            &self.name,
                            layer_index,
                            *h3_cell,
                            deepest_index,
                            energy_input,
                            &format!("Core radiance: {:.2e}W", current_wattage),
                        );
                    }
                }
            }
        }
    }
}

/// Example convection plume component using transaction gateway
pub struct ConvectionPlumeComponent {
    pub name: String,
}

impl ConvectionPlumeComponent {
    pub fn new() -> Self {
        Self {
            name: "ConvectionPlume".to_string(),
        }
    }
}

impl SimComponent for ConvectionPlumeComponent {
    fn step(&mut self, simulation: &mut Simulation, _step: i64, _year: f64) {
        // Example: Simple convection between layer sets
        
        for layer_index in 1..simulation.layer_sets.len() {
            let layer_set = &simulation.layer_sets[layer_index];
            
            for (h3_cell, column) in &layer_set.layers {
                // Check for density inversion (simplified)
                if let Some(source_cell) = simulation.get_cell(layer_index, *h3_cell, 0) {
                    if let Some(target_cell) = simulation.get_cell(layer_index - 1, *h3_cell, 0) {
                        let source_density = source_cell.mass_kg() / 1e12; // Simplified density
                        let target_density = target_cell.mass_kg() / 1e12;
                        
                        // If lower layer is less dense, create plume
                        if source_density < target_density * 0.95 {
                            let energy_transfer = source_cell.energy_joules() * 0.001; // 0.1%
                            let mass_transfer = source_cell.mass_kg() * 0.0001; // 0.01%
                            
                            // Use gateway method for plume transport
                            simulation.propose_mass_transfer(
                                &self.name,
                                layer_index, *h3_cell, 0,      // From deeper layer
                                layer_index - 1, *h3_cell, 0,  // To shallower layer
                                -energy_transfer,
                                -mass_transfer,
                                &format!("Convection plume: density {:.2e} -> {:.2e}", source_density, target_density),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    println!("🧪 Component Transaction Integration Example");
    println!("Shows how components use gateway methods to generate transactions\n");
    
    // This example shows the API - actual simulation setup would be more complex
    println!("📋 Gateway Methods Available to Components:");
    println!("   • propose_transaction() - Full transaction with all parameters");
    println!("   • propose_energy_transaction() - Energy-only changes (common)");
    println!("   • propose_mass_transfer() - Mass/energy transfer between cells");
    println!("   • get_cell() - Read cell state for calculations");
    println!("   • record_cell_baseline() - Set baseline for validation");
    
    println!("\n🔧 Component Integration Pattern:");
    println!("   1. Component reads cell states using get_cell()");
    println!("   2. Component calculates desired changes");
    println!("   3. Component proposes transactions via gateway methods");
    println!("   4. Simulation validates and regulates all transactions");
    println!("   5. Simulation applies regulated transactions atomically");
    
    println!("\n🎯 Benefits:");
    println!("   ✅ Components don't need transaction system knowledge");
    println!("   ✅ Clean, simple API for common operations");
    println!("   ✅ Automatic 3D cell location handling");
    println!("   ✅ Built-in validation and regulation");
    println!("   ✅ Parallel processing of all transactions");
    
    println!("\n✅ Component Transaction Integration Example Completed!");
}
