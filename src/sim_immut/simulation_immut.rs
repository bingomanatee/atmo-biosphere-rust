use crate::component::SimComponent;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::events::EventEmitter;
use crate::sim_immut::layer_set_immut::{LayerSetImmut, ImmutableLayerSetParams};
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
    pub layer_set_params: Vec<ImmutableLayerSetParams>,
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

        // Define thermal gradients for each layer set (K/km)
        let layer_gradients = vec![25.0, 15.0, 10.0, 5.0];

        for (layer_index, params) in self.config.layer_set_params.iter().enumerate() {
            // Update start height to be the bottom of the previous layer
            let mut adjusted_params = params.clone();
            adjusted_params.start_height_km = cumulative_bottom_km;

            // Get gradient for this layer
            let gradient_k_per_km = layer_gradients.get(layer_index).copied().unwrap_or(5.0);

            // Create the immutable layer set
            let layer_set = LayerSetImmut::new(adjusted_params);

            // Apply thermal gradient (immutable pattern)
            let layer_set_with_thermal = layer_set.with_thermal_gradient(current_temperature, gradient_k_per_km);

            // Apply pressure adjustments if not the first layer
            let final_layer_set = if layer_index > 0 {
                let accumulated_mass_per_km2 = self.calculate_accumulated_mass_per_km2(layer_index);
                layer_set_with_thermal.with_pressure_adjustments(accumulated_mass_per_km2)
            } else {
                layer_set_with_thermal
            };

            // Calculate temperature at bottom of this layer for next layer
            let layer_thickness_km = params.column_count as f64 * params.cell_height_km;
            current_temperature += gradient_k_per_km * layer_thickness_km;

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

        // Group transactions by layer set and cell location
        let mut energy_changes: HashMap<CellLocation, f64> = HashMap::new();
        let mut mass_changes: HashMap<CellLocation, f64> = HashMap::new();

        for transaction in &transactions {
            // Apply to source cell
            if transaction.energy_delta_joules != 0.0 {
                *energy_changes
                    .entry(transaction.source_cell.clone())
                    .or_insert(0.0) += transaction.energy_delta_joules;
            }
            if transaction.mass_delta_kg != 0.0 {
                *mass_changes
                    .entry(transaction.source_cell.clone())
                    .or_insert(0.0) += transaction.mass_delta_kg;
            }

            // Apply to target cell if it exists
            if let Some(ref target_cell) = transaction.target_cell {
                if transaction.energy_delta_joules != 0.0 {
                    *energy_changes.entry(target_cell.clone()).or_insert(0.0) -=
                        transaction.energy_delta_joules;
                }
                if transaction.mass_delta_kg != 0.0 {
                    *mass_changes.entry(target_cell.clone()).or_insert(0.0) -=
                        transaction.mass_delta_kg;
                }
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
                    let mut new_cell = cell.clone();

                    // Apply energy changes
                    if let Some(&energy_delta) = energy_changes.get(&location) {
                        new_cell = new_cell.with_energy_delta(energy_delta);
                    }

                    // Apply mass changes
                    if let Some(&mass_delta) = mass_changes.get(&location) {
                        new_cell = new_cell.with_mass_delta(mass_delta);
                    }

                    new_cells.push(new_cell);
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

/// Helper function to create default immutable layer set parameters
pub fn default_immutable_layer_set_params(resolution: h3o::Resolution, planet_radius_km: f64) -> Vec<ImmutableLayerSetParams> {
    vec![
        ImmutableLayerSetParams {
            resolution,
            start_height_km: 0.0,
            cell_height_km: 5.0,
            material_name: "basalt".to_string(),
            column_count: 5,
            planet_radius_km,
        },
        ImmutableLayerSetParams {
            resolution,
            start_height_km: 50.0,
            cell_height_km: 10.0,
            material_name: "granite".to_string(),
            column_count: 10,
            planet_radius_km,
        },
        ImmutableLayerSetParams {
            resolution,
            start_height_km: 150.0,
            cell_height_km: 15.0,
            material_name: "basalt".to_string(),
            column_count: 5,
            planet_radius_km,
        },
        ImmutableLayerSetParams {
            resolution,
            start_height_km: 225.0,
            cell_height_km: 20.0,
            material_name: "granite".to_string(),
            column_count: 3,
            planet_radius_km,
        },
    ]
}
