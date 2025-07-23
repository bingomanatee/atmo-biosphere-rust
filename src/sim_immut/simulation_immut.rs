use crate::component::SimComponent;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::events::EventEmitter;
use crate::sim_immut::layer_set_immut::{LayerSetImmut, LayerSetParamsImmut};
use crate::sim::transaction_manager::{CellLocation, Transaction, TransactionManager};
use crate::sim_immut::energy_mass_cell_immut::EnergyMassCellImmut;
use std::collections::HashMap;
use h3o::{CellIndex, Resolution};

/// Immutable simulation configuration
#[derive(Clone)]
pub struct SimulationConfigImmut {
    pub steps: u64,
    pub years_per_step: f64,
    pub warmup_steps: u64,
    pub layer_set_params: Vec<LayerSetParamsImmut>,
    pub surface_temp_k: f64,
}

/// Immutable simulation that uses immutable layer sets for better performance
pub struct SimulationImmut {
    pub state: SimulationState,
    pub step: i64,
    pub steps: u64,
    pub config: SimulationConfigImmut,
    components: HashMap<&'static str, Box<dyn SimComponent>>,
    pub layer_sets: Vec<LayerSetImmut>,
    pub plumes: Vec<crate::component::convection_plume_component::ConvectionPlume>,
    pub next_plume_id: u64,
    pub transaction_manager: TransactionManager,
    pub event_emitter: EventEmitter,
}

pub enum SimulationState {
    Created,
    RunningWarmup,
    Running,
    Paused,
    Stopped,
    Error,
}

impl SimulationImmut {
    pub fn new(config: SimulationConfigImmut, components: &mut Vec<Box<dyn SimComponent>>) -> Self {
        let mut sim = SimulationImmut {
            state: SimulationState::Created,
            step: 0.min((config.warmup_steps as i64) * -1),
            steps: 0,
            config: config,
            components: HashMap::new(),
            layer_sets: Vec::new(),
            plumes: Vec::new(),
            next_plume_id: 1,
            transaction_manager: TransactionManager::new(),
            event_emitter: EventEmitter::new(),
        };
        for comp in components.drain(..) {
            sim.register_box(comp);
        }
        sim.load_layer_sets();
        sim
    }

    pub fn register_box(&mut self, comp_box: Box<dyn SimComponent>) {
        let key = comp_box.key();
        self.components.insert(key, comp_box);
    }

    /// Load immutable layer sets with thermal gradients and pressure adjustments
    pub fn load_layer_sets(&mut self) {
        let mut cumulative_bottom_km = 0.0;
        let mut current_temperature = self.config.surface_temp_k;

        for (layer_index, params) in self.config.layer_set_params.iter().enumerate() {
            // Update start height to be the bottom of the previous layer
            let mut adjusted_params = params.clone();
            adjusted_params.start_height_km = cumulative_bottom_km;

            // Create the immutable layer set
            let layer_set = LayerSetImmut::new(adjusted_params);

            // Apply pressure adjustments FIRST if not the first layer
            let layer_set_with_pressure = if layer_index > 0 {
                let accumulated_mass_per_km2 = self.calculate_accumulated_mass_per_km2(layer_index);
                layer_set.with_pressure_adjustments(accumulated_mass_per_km2)
            } else {
                layer_set
            };

            // Apply thermal gradient LAST using the gradient from layer config (immutable pattern)
            let layer_set_with_thermal = layer_set_with_pressure.with_thermal_gradient(current_temperature, params.thermal_gradient_k_per_km);

            // Final step: reassert mass based on final pressure and temperature to ensure consistency
            let final_layer_set = layer_set_with_thermal.with_final_mass_adjustment();

            // Calculate temperature at bottom of this layer for next layer
            let layer_thickness_km = params.column_count as f64 * params.cell_height_km;
            current_temperature += params.thermal_gradient_k_per_km * layer_thickness_km;

            // Update cumulative bottom for next layer
            cumulative_bottom_km += layer_thickness_km;

            self.layer_sets.push(final_layer_set);
        }

        println!("🌍 Loaded {} immutable layer sets", self.layer_sets.len());
    }

    /// Calculate accumulated mass per km² from all layers above the given layer index
    fn calculate_accumulated_mass_per_km2(&self, layer_index: usize) -> f64 {
        let mut accumulated_mass_per_km2 = 0.0;

        for i in 0..layer_index {
            if let Some(layer_set) = self.layer_sets.get(i) {
                accumulated_mass_per_km2 += layer_set.total_mass_per_km2();
            }
        }

        accumulated_mass_per_km2
    }

    /// Run a single simulation step (immutable pattern)
    pub fn step(&mut self) {
        self.transaction_manager.set_current_step(self.step);

        // TODO: Process components (requires component trait adaptation for immutable simulation)
        // For now, skip component processing to test basic immutable structure
        println!("   🔄 Step {}: Immutable simulation step (components disabled for now)", self.step + 1);

        // Apply transactions to create new layer sets (immutable pattern)
        self.apply_transactions_immutably();

        self.step += 1;
        self.steps += 1;
    }

    /// Apply transactions to create new immutable layer sets
    fn apply_transactions_immutably(&mut self) {
        // Get all committed transactions from the transaction manager
        let transactions = self.transaction_manager.get_committed_transactions_for_step(self.step);

        if transactions.is_empty() {
            return;
        }

        // Re-index transactions by source cell location for efficient lookup
        let mut transactions_by_cell: HashMap<CellLocation, Vec<&Transaction>> = HashMap::new();

        for transaction in &transactions {
            // Group by source cell
            transactions_by_cell
                .entry(transaction.source_cell.clone())
                .or_insert_with(Vec::new)
                .push(transaction);

            // Group by target cell if it exists
            if let Some(ref target_cell) = transaction.target_cell {
                transactions_by_cell
                    .entry(target_cell.clone())
                    .or_insert_with(Vec::new)
                    .push(transaction);
            }
        }

        // Apply changes immutably - create new layer sets with updated cells
        let mut new_layer_sets = Vec::new();

        for (layer_set_index, layer_set) in self.layer_sets.iter().enumerate() {
            let mut new_layer_set = layer_set.clone();

            // Apply changes to each cell in this layer set
            for (h3_cell_index, column) in &mut new_layer_set.layers {
                let mut new_cells = Vec::new();

                for (depth_index, cell) in column.cells.iter().enumerate() {
                    let location = CellLocation::new(layer_set_index, *h3_cell_index, depth_index);

                    // Check if this cell has any transactions
                    if let Some(cell_transactions) = transactions_by_cell.get(&location) {
                        // Calculate net energy and mass changes for this cell
                        let mut net_energy_delta = 0.0;
                        let mut net_mass_delta = 0.0;

                        for transaction in cell_transactions {
                            // If this cell is the source, add the deltas
                            if transaction.source_cell == location {
                                net_energy_delta += transaction.energy_delta_joules;
                                net_mass_delta += transaction.mass_delta_kg;
                            }
                            // If this cell is the target, subtract the deltas
                            if transaction.target_cell.as_ref() == Some(&location) {
                                net_energy_delta -= transaction.energy_delta_joules;
                                net_mass_delta -= transaction.mass_delta_kg;
                            }
                        }

                        // Apply net changes immutably
                        let mut new_cell = cell.clone();
                        if net_energy_delta != 0.0 {
                            new_cell = new_cell.with_energy_delta(net_energy_delta);
                        }
                        if net_mass_delta != 0.0 {
                            new_cell = new_cell.with_mass_delta(net_mass_delta);
                        }
                        new_cells.push(new_cell);
                    } else {
                        // No transactions for this cell, just clone it
                        new_cells.push(cell.clone());
                    }
                }

                column.cells = new_cells;
            }

            new_layer_sets.push(new_layer_set);
        }

        // Replace old layer sets with new ones
        self.layer_sets = new_layer_sets;

        println!("🔄 Applied {} transactions immutably across {} layer sets",
                 transactions.len(), self.layer_sets.len());
    }

    /// Get total number of cells across all layer sets
    pub fn total_cells(&self) -> usize {
        self.layer_sets.iter()
            .map(|layer_set| {
                layer_set.layers.values()
                    .map(|column| column.cells.len())
                    .sum::<usize>()
            })
            .sum()
    }

    /// Get layer set by index
    pub fn get_layer_set(&self, index: usize) -> Option<&LayerSetImmut> {
        self.layer_sets.get(index)
    }

    /// Get mutable layer set by index (for component access)
    pub fn get_layer_set_mut(&mut self, index: usize) -> Option<&mut LayerSetImmut> {
        self.layer_sets.get_mut(index)
    }

    /// Get years per step from configuration
    pub fn years_per_step(&self) -> f64 {
        self.config.years_per_step
    }

    /// Get current simulation step
    pub fn current_step(&self) -> i64 {
        self.step
    }

    /// Get current simulation year
    pub fn current_year(&self) -> f64 {
        self.step as f64 * self.config.years_per_step
    }

    /// Initialize the simulation (compatibility method)
    pub fn initialize(&mut self) {
        // Immutable simulation is initialized in constructor
        println!("🌍 Immutable simulation initialized with {} layer sets", self.layer_sets.len());
    }

    /// Get simulation state
    pub fn get_state(&self) -> &SimulationState {
        &self.state
    }

    /// Set simulation state
    pub fn set_state(&mut self, state: SimulationState) {
        self.state = state;
    }

    /// Run multiple simulation steps
    pub fn run(&mut self, steps: u64) {
        self.set_state(SimulationState::Running);

        for _ in 0..steps {
            self.step();

            if matches!(self.state, SimulationState::Stopped | SimulationState::Error) {
                break;
            }
        }

        self.set_state(SimulationState::Paused);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use h3o::Resolution;
    use crate::energy_mass::energy_mass::EnergyMass;
    use crate::sim_immut::layer_set_immut::default_layer_set_params_immut;

    #[test]
    fn test_thermal_gradient_fix() {
        println!("\n🌡️ Testing Thermal Gradient Fix");
        println!("===============================");

        // Create immutable simulation with default geological layers
        let config = SimulationConfigImmut {
            steps: 1,
            years_per_step: 1000.0,
            warmup_steps: 0,
            surface_temp_k: 288.15, // 15°C surface temperature
            layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        };

        let mut components: Vec<Box<dyn SimComponent>> = vec![];
        let sim = SimulationImmut::new(config, &mut components);

        // Check that thermal gradients are working correctly
        println!("🔍 Checking thermal gradients in each layer set:");

        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            if let Some((_, column)) = layer_set.layers.iter().next() {
                if let (Some(first_cell), Some(last_cell)) = (column.cells.first(), column.cells.last()) {
                    let first_temp = first_cell.temperature_kelvin();
                    let last_temp = last_cell.temperature_kelvin();

                    println!("   Layer {}: {:.1}K → {:.1}K ({:.1}°C → {:.1}°C)",
                             layer_idx, first_temp, last_temp,
                             first_temp - 273.15, last_temp - 273.15);

                    // Verify temperatures are reasonable with 0.5 K/km gradients
                    if layer_idx == 0 {
                        // Layer 0: 288K + 0.5K/km * 25km = 300.5K max
                        assert!(first_temp > 280.0, "Layer 0 first cell too cold: {:.1}K", first_temp);
                        assert!(first_temp < 320.0, "Layer 0 first cell too hot: {:.1}K", first_temp);
                        assert!(last_temp > 290.0, "Layer 0 last cell too cold: {:.1}K", last_temp);
                        assert!(last_temp < 330.0, "Layer 0 last cell too hot: {:.1}K", last_temp);
                    } else {
                        // Deeper layers should have moderate temperatures (not 1.0K)
                        assert!(first_temp > 250.0, "Layer {} first cell too cold: {:.1}K", layer_idx, first_temp);
                        assert!(first_temp < 500.0, "Layer {} first cell too hot: {:.1}K", layer_idx, first_temp);
                        assert!(last_temp > 250.0, "Layer {} last cell too cold: {:.1}K", layer_idx, last_temp);
                        assert!(last_temp < 600.0, "Layer {} last cell too hot: {:.1}K", layer_idx, last_temp);
                    }

                    // Temperature should increase with depth within each layer
                    assert!(last_temp >= first_temp, "Temperature should increase with depth in layer {}", layer_idx);
                }
            }
        }

        println!("✅ Thermal gradient fix verified!");
        println!("   - All layers have realistic temperatures");
        println!("   - No more 1.0K temperatures in deep layers");
        println!("   - Temperature increases with depth as expected");
    }

    fn get_layer_name(layer_idx: usize) -> &'static str {
        match layer_idx {
            0 => "Crust",
            1 => "Upper Mantle",
            2 => "Lower Mantle",
            3 => "Asthenosphere",
            _ => "Unknown Layer",
        }
    }
}