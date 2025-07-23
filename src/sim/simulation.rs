use crate::component::SimComponent;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::events::EventEmitter;
use crate::sim::layer_set::{LayerSet, LayerSetParams};
use crate::sim::transaction_manager::{CellLocation, Transaction, TransactionManager};
use std::collections::HashMap;

/// Thermal gradient configuration using a quadratic model
#[derive(Clone)]
pub struct SimulationConfig {
    pub steps: u64,
    pub years_per_step: f64,
    pub warmup_steps: u64,
    pub layer_set_params: Vec<LayerSetParams>,
    pub surface_temp_k: f64,
}

pub struct Simulation {
    pub state: SimulationState,
    pub step: i64,
    pub steps: u64,
    pub config: SimulationConfig,
    components: HashMap<&'static str, Box<dyn SimComponent>>,
    pub layer_sets: Vec<LayerSet>,
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

impl Simulation {
    pub fn new(config: SimulationConfig, components: &mut Vec<Box<dyn SimComponent>>) -> Self {
        let mut sim = Simulation {
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
    pub fn start_temp_at_depth(&self, depth: f64) -> f64 {
        let mut rel_depth: f64 = depth;
        let start_temp = self.config.surface_temp_k;
        let mut temp = start_temp;
        for layer_set in self.config.layer_set_params.iter() {
            let total_height = layer_set.cell_height_km * layer_set.cells_per_column as f64;

            if rel_depth > total_height {
                temp += layer_set.thermal_gradient_k_per_km * total_height;
                rel_depth -= total_height;
            } else {
                return temp + layer_set.thermal_gradient_k_per_km * rel_depth;
            }
        }
        temp
    }

    pub fn register_box(&mut self, comp_box: Box<dyn SimComponent>) {
        let key = comp_box.key();
        self.components.insert(key, comp_box);
    }

    pub fn load_layer_sets(&mut self) {
        let mut cumulative_bottom_km = 0.0;

        for (layer_index, params) in self.config.layer_set_params.iter().enumerate() {
            // Update start height to be the bottom of the previous layer
            let mut adjusted_params = params.clone();
            adjusted_params.start_height_km = cumulative_bottom_km;

            // Create the layer set with thermal configuration
            let mut layer_set = LayerSet::new(
                &adjusted_params,
                self.start_temp_at_depth(cumulative_bottom_km),
            );

            // Calculate pressure adjustments for this layer set
            if layer_index > 0 {
                // Get accumulated mass per km² from all layers above
                let accumulated_mass_per_km2 = self.calculate_accumulated_mass_per_km2(layer_index);
                layer_set.adjust_pressures_for_accumulated_mass(accumulated_mass_per_km2);
            }

            // Update cumulative bottom for next layer
            cumulative_bottom_km += params.cells_per_column as f64 * params.cell_height_km;

            self.layer_sets.push(layer_set);
        }

        // After all layers are created, perform final pressure adjustment pass
        self.adjust_all_pressures_for_mass_above();

        // Apply thermal gradient across all layer sets (second pass)
        self.apply_thermal_gradient_across_all_layers();
    }

    /// Get years per step for components
    pub fn years_per_step(&self) -> f64 {
        self.config.years_per_step
    }

    /// Get current simulation step
    pub fn current_step(&self) -> i64 {
        self.step
    }

    /// Get current simulation year
    pub fn current_year(&self) -> i64 {
        self.step * self.config.years_per_step as i64
    }

    /// Advance simulation by one step (for manual stepping)
    pub fn advance_step(&mut self) {
        self.step += 1;
    }

    /// Create a new plume and add it to the simulation
    pub fn create_plume(
        &mut self,
        source_layer_index: usize,
        source_cell_index: h3o::CellIndex,
        position: (f64, f64),
        initial_depth_km: f64,
        total_energy_joules: f64,
        total_mass_kg: f64,
        temperature_k: f64,
        velocity_km_per_year: f64,
        buoyancy_force: f64,
        radius_km: f64,
    ) -> u64 {
        let plume_id = self.next_plume_id;
        self.next_plume_id += 1;

        let plume = crate::component::convection_plume_component::ConvectionPlume::new(
            plume_id,
            source_layer_index,
            source_cell_index,
            position,
            initial_depth_km,
            total_energy_joules,
            total_mass_kg,
            temperature_k,
            velocity_km_per_year,
            buoyancy_force,
            radius_km,
        );

        self.plumes.push(plume);
        plume_id
    }

    /// Get the number of active plumes
    pub fn plume_count(&self) -> usize {
        self.plumes.len()
    }
    fn run(&mut self) {
        match self.state {
            SimulationState::Created => {
                self.initialize();
                if self.config.warmup_steps == 0 {
                    self.state = SimulationState::Running;
                } else {
                    self.state = SimulationState::RunningWarmup;
                }
                self.step();
            }
            _ => todo!(),
        }
    }

    pub fn initialize(&mut self) {
        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        for (_, comp) in components.iter_mut() {
            comp.initialize(self);
        }

        // Put the components back
        self.components = components;
    }

    pub fn step(&mut self) {
        self.step_with_debug(false);
    }

    pub fn step_with_debug(&mut self, enable_transaction_debug: bool) {
        let step = self.step;
        let year = self.current_year();
        let years_per_step = self.years_per_step();

        println!(
            "\n🔄 Step {}: Year {} ({:.0} years/step)",
            step, year, years_per_step
        );

        // 1. Record baseline snapshots for transaction validation
        self.record_all_baselines();

        // 2. Run components to generate transactions
        self.run_components_with_transactions(step, year as f64);

        // 3. Validate and regulate transactions
        let regulated_transactions = if enable_transaction_debug {
            self.transaction_manager
                .validate_and_regulate_transactions_with_debug(years_per_step, true)
        } else {
            self.transaction_manager
                .validate_and_regulate_transactions(years_per_step)
        };

        // 3.5. Check if hotspots caused scaling and adapt if needed
        let scaling_detected = self.detect_hotspot_scaling(&regulated_transactions);
        if scaling_detected {
            self.adapt_overpowered_hotspots();
        }

        // 4. Apply regulated transactions to simulation
        self.apply_regulated_transactions(&regulated_transactions);

        // 5. Apply legacy pending energy changes (for components not yet converted)
        self.apply_all_pending_energy_changes();

        // Increment step counter
        self.step += 1;
    }

    /// Record baseline snapshots for all cells
    fn record_all_baselines(&mut self) {
        // Collect all cell locations first to avoid borrowing issues
        let mut cell_locations = Vec::new();
        for (layer_index, layer_set) in self.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for depth_index in 0..column.cells.len() {
                    cell_locations.push((layer_index, *h3_cell, depth_index));
                }
            }
        }

        // Now record baselines for all collected locations
        for (layer_index, h3_cell, depth_index) in cell_locations {
            self.record_cell_baseline(layer_index, h3_cell, depth_index);
        }
    }

    /// Run components with transaction generation
    fn run_components_with_transactions(&mut self, step: i64, year: f64) {
        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        // Each component generates transactions instead of direct changes
        for (component_name, comp) in components.iter_mut() {
            comp.step(self, step, year as i64);
        }

        // Put the components back
        self.components = components;
    }

    /// Detect if hotspot-related transactions were scaled (indicating overpowered hotspots)
    fn detect_hotspot_scaling(
        &self,
        regulated_transactions: &[crate::sim::transaction_manager::Transaction],
    ) -> bool {
        let hotspot_scaled_count = regulated_transactions
            .iter()
            .filter(|tx| {
                (tx.source.contains("CoreRadiance") || tx.source.contains("Hotspot"))
                    && tx.description.contains("SCALED")
            })
            .count();

        let total_hotspot_transactions = regulated_transactions
            .iter()
            .filter(|tx| tx.source.contains("CoreRadiance") || tx.source.contains("Hotspot"))
            .count();

        if total_hotspot_transactions > 0 {
            let scaling_percentage =
                (hotspot_scaled_count as f64 / total_hotspot_transactions as f64) * 100.0;

            if scaling_percentage > 25.0 {
                // If more than 25% of hotspot transactions were scaled
                println!(
                    "🚨 Hotspot scaling detected: {}/{} transactions scaled ({:.1}%)",
                    hotspot_scaled_count, total_hotspot_transactions, scaling_percentage
                );
                return true;
            }
        }

        false
    }

    /// Adapt overpowered components by calling their adaptation methods
    fn adapt_overpowered_hotspots(&mut self) {
        println!("🔥 Adapting overpowered components...");

        // Call adapt_if_overpowered on all components
        let mut components = std::mem::take(&mut self.components);

        for (component_name, component) in components.iter_mut() {
            component.adapt_if_overpowered(self, true);
            println!("   ✅ Checked component: {}", component_name);
        }

        self.components = components;

        println!("🎯 Component adaptation complete");
    }

    /// Generate performance report for all components (final - ends simulation)
    pub fn generate_performance_report(&mut self) -> String {
        "Performance profiling disabled".to_string()
    }

    /// Generate intermediate performance report (reusable - doesn't end simulation)
    pub fn generate_intermediate_report(&self) -> String {
        "Performance profiling disabled".to_string()
    }

    /// Get current performance summary (lightweight, reusable)
    pub fn get_performance_summary(&self) -> String {
        "Performance profiling disabled".to_string()
    }

    /// Get component-specific performance data (reusable)
    pub fn get_component_performance(&self, component_name: &str) -> Option<String> {
        None // Performance profiling disabled
    }

    /// Print performance report to console
    pub fn print_performance_report(&mut self) {
        let report = self.generate_performance_report();
        println!("{}", report);
    }

    /// Reset profiling data
    pub fn reset_profiling(&mut self) {
        // Performance profiling disabled
    }

    /// Add an event listener to the simulation
    pub fn add_event_listener<L: crate::events::event_listener::EventListener + Send + 'static>(
        &mut self,
        listener: L,
    ) {
        self.event_emitter.add_listener(listener);
    }

    /// Calculate actual overhead mass from all cells above the specified cell
    pub fn calculate_overhead_mass_for_cell(
        &self,
        target_layer: usize,
        target_h3: h3o::CellIndex,
        target_depth: usize,
    ) -> f64 {
        let mut total_overhead_mass_kg = 0.0;

        // Sum mass from all layer sets above the target layer
        for layer_index in 0..target_layer {
            if let Some(layer_set) = self.layer_sets.get(layer_index) {
                if let Some(column) = layer_set.layers.get(&target_h3) {
                    for cell in &column.cells {
                        total_overhead_mass_kg += cell.mass_kg();
                    }
                }
            }
        }

        // Sum mass from cells above in the same layer set
        if let Some(layer_set) = self.layer_sets.get(target_layer) {
            if let Some(column) = layer_set.layers.get(&target_h3) {
                for depth_index in 0..target_depth {
                    if let Some(cell) = column.cells.get(depth_index) {
                        total_overhead_mass_kg += cell.mass_kg();
                    }
                }
            }
        }

        // Convert to mass per m² using cell area
        if let Some(cell) = self.get_cell(target_layer, target_h3, target_depth) {
            let area_km2 = cell.area();
            let area_m2 = area_km2 * 1e6; // Convert km² to m²
            total_overhead_mass_kg / area_m2
        } else {
            0.0
        }
    }

    /// Gateway method: Propose a transaction between cells
    pub fn propose_transaction(
        &mut self,
        component_name: &str,
        source_layer: usize,
        source_h3: h3o::CellIndex,
        source_depth: usize,
        target_layer: Option<usize>,
        target_h3: Option<h3o::CellIndex>,
        target_depth: Option<usize>,
        energy_delta_joules: f64,
        mass_delta_kg: f64,
        description: &str,
    ) {
        let source_location = CellLocation::new(source_layer, source_h3, source_depth);

        let target_location =
            if let (Some(layer), Some(h3), Some(depth)) = (target_layer, target_h3, target_depth) {
                Some(CellLocation::new(layer, h3, depth))
            } else {
                None
            };

        let transaction = Transaction {
            source: component_name.to_string(),
            source_cell: source_location,
            target_cell: target_location,
            energy_delta_joules,
            mass_delta_kg,
            description: description.to_string(),
            step_id: self.step,
        };

        self.transaction_manager.propose_transaction(transaction);
    }

    /// Gateway method: Propose energy-only transaction (common case)
    pub fn propose_energy_transaction(
        &mut self,
        component_name: &str,
        layer: usize,
        h3_cell: h3o::CellIndex,
        depth: usize,
        energy_delta_joules: f64,
        description: &str,
    ) {
        self.propose_transaction(
            component_name,
            layer,
            h3_cell,
            depth,
            None,
            None,
            None,
            energy_delta_joules,
            0.0,
            description,
        );
    }

    /// Gateway method: Propose mass transfer between adjacent cells
    pub fn propose_mass_transfer(
        &mut self,
        component_name: &str,
        from_layer: usize,
        from_h3: h3o::CellIndex,
        from_depth: usize,
        to_layer: usize,
        to_h3: h3o::CellIndex,
        to_depth: usize,
        energy_delta_joules: f64,
        mass_delta_kg: f64,
        description: &str,
    ) {
        self.propose_transaction(
            component_name,
            from_layer,
            from_h3,
            from_depth,
            Some(to_layer),
            Some(to_h3),
            Some(to_depth),
            energy_delta_joules,
            mass_delta_kg,
            description,
        );
    }

    /// Gateway method: Get cell reference for components to read state
    pub fn get_cell(
        &self,
        layer: usize,
        h3_cell: h3o::CellIndex,
        depth: usize,
    ) -> Option<&crate::sim::energy_mass_cell::EnergyMassCell> {
        self.layer_sets
            .get(layer)?
            .layers
            .get(&h3_cell)?
            .cells
            .get(depth)
    }

    /// Gateway method: Get cell baseline for transaction validation
    pub fn record_cell_baseline(&mut self, layer: usize, h3_cell: h3o::CellIndex, depth: usize) {
        if let Some(cell) = self.get_cell(layer, h3_cell, depth) {
            let location = CellLocation::new(layer, h3_cell, depth);
            // Calculate initial overhead mass from current pressure
            let current_pressure = cell.pressure_pa();
            let initial_overhead_mass_kg_per_m2 = (current_pressure
                - crate::constants::REFERENCE_PRESSURE_PA)
                / crate::constants::GRAVITY_M_S2;

            let snapshot = crate::sim::transaction_manager::CellSnapshot {
                location: location.clone(),
                mass_kg: cell.mass_kg(),
                energy_joules: cell.energy_joules(),
                temperature_kelvin: cell.temperature_kelvin(),
                initial_overhead_mass_kg_per_m2,
            };
            self.transaction_manager
                .record_baseline_snapshot(location, snapshot);
        }
    }

    /// Record baseline snapshots of all cells for transaction validation
    fn record_baseline_snapshots(&mut self) {
        use crate::energy_mass::energy_mass::EnergyMass;
        use crate::sim::transaction_manager::{CellLocation, CellSnapshot};

        for (layer_set_index, layer_set) in self.layer_sets.iter().enumerate() {
            for (h3_cell_index, column) in &layer_set.layers {
                for (depth_index, cell) in column.cells.iter().enumerate() {
                    let location = CellLocation::new(layer_set_index, *h3_cell_index, depth_index);
                    // Calculate initial overhead mass from current pressure
                    let current_pressure = cell.pressure_pa();
                    let initial_overhead_mass_kg_per_m2 = (current_pressure
                        - crate::constants::REFERENCE_PRESSURE_PA)
                        / crate::constants::GRAVITY_M_S2;

                    let snapshot = CellSnapshot {
                        location: location.clone(),
                        mass_kg: cell.mass_kg(),
                        energy_joules: cell.energy_joules(),
                        temperature_kelvin: cell.temperature_kelvin(),
                        initial_overhead_mass_kg_per_m2,
                    };
                    self.transaction_manager
                        .record_baseline_snapshot(location, snapshot);
                }
            }
        }
    }

    /// Apply regulated transactions to the simulation using 3D cell locations
    fn apply_regulated_transactions(
        &mut self,
        transactions: &[crate::sim::transaction_manager::Transaction],
    ) {
        use crate::energy_mass::energy_mass::EnergyMass;
        use crate::sim::transaction_manager::CellLocation;
        use std::collections::HashMap;

        // Group transactions by 3D cell location for efficient application
        let mut energy_changes: HashMap<CellLocation, f64> = HashMap::new();
        let mut mass_changes: HashMap<CellLocation, f64> = HashMap::new();

        for transaction in transactions {
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

        // Apply all changes atomically using 3D coordinates
        let mut total_energy_applied = 0.0;
        let mut total_mass_applied = 0.0;
        let mut cells_modified = 0;

        for (layer_set_index, layer_set) in self.layer_sets.iter_mut().enumerate() {
            for (h3_cell_index, column) in &mut layer_set.layers {
                for (depth_index, cell) in column.cells.iter_mut().enumerate() {
                    let location = CellLocation::new(layer_set_index, *h3_cell_index, depth_index);
                    let mut cell_modified = false;

                    // Apply energy changes
                    if let Some(&energy_delta) = energy_changes.get(&location) {
                        if energy_delta > 0.0 {
                            cell.add_energy_joules(energy_delta);
                        } else if energy_delta < 0.0 {
                            cell.remove_energy_joules(-energy_delta);
                        }
                        total_energy_applied += energy_delta.abs();
                        cell_modified = true;
                    }

                    // Apply mass changes
                    if let Some(&mass_delta) = mass_changes.get(&location) {
                        cell.add_mass_kg(mass_delta);
                        total_mass_applied += mass_delta.abs();
                        cell_modified = true;
                    }

                    if cell_modified {
                        cells_modified += 1;
                    }
                }
            }
        }

        println!(
            "💾 Applied {} transactions: {:.2e}J, {:.2e}kg across {} cells",
            transactions.len(),
            total_energy_applied,
            total_mass_applied,
            cells_modified
        );
    }

    /// Apply all pending energy changes from components using proper energy methods
    /// This ensures proper material state updates, phase transitions, and energy bank handling
    fn apply_all_pending_energy_changes(&mut self) {
        for layer_set in &mut self.layer_sets {
            for column in layer_set.layers.values_mut() {
                for cell in &mut column.cells {
                    let pending_change = cell.pending_energy_change();
                    if pending_change != 0.0 {
                        if pending_change > 0.0 {
                            // Add energy using proper method (handles material state, phase transitions, etc.)
                            cell.add_energy_joules(pending_change);
                        } else {
                            // Remove energy using proper method
                            cell.remove_energy_joules(-pending_change);
                        }
                        // Reset pending changes after applying
                        cell.reset_pending_energy_changes();
                    }
                }
            }
        }
    }

    /// Calculate accumulated mass per km² from all layers above the specified layer index
    fn calculate_accumulated_mass_per_km2(&self, layer_index: usize) -> f64 {
        let mut total_mass_per_km2 = 0.0;

        // Sum mass from all layers above this one
        for i in 0..layer_index {
            if let Some(layer_set) = self.layer_sets.get(i) {
                total_mass_per_km2 += layer_set.calculate_average_mass_per_km2();
            }
        }

        total_mass_per_km2
    }

    /// Adjust pressures in all layer sets to account for mass of cells above
    fn adjust_all_pressures_for_mass_above(&mut self) {
        // We need to process layers from top to bottom, accumulating mass
        let mut accumulated_mass_per_km2 = 0.0;

        for layer_index in 0..self.layer_sets.len() {
            // Update pressures in this layer based on accumulated mass from above
            if let Some(layer_set) = self.layer_sets.get_mut(layer_index) {
                layer_set.adjust_pressures_for_accumulated_mass(accumulated_mass_per_km2);

                // Add this layer's mass to the accumulation for layers below
                accumulated_mass_per_km2 += layer_set.calculate_average_mass_per_km2();
            }
        }
    }

    /// Apply layer-specific thermal gradients to each layer set
    /// Each layer set has its own thermal gradient (25, 15, 10, 5 K/km)
    fn apply_thermal_gradient_across_all_layers(&mut self) {
        

        println!("🌡️ Applying layer-specific thermal gradients...");

        // Track temperature from previous layer set
        let mut current_temperature = self.config.surface_temp_k;

        // Apply gradients layer by layer
        for (layer_set_index, layer_set) in self.layer_sets.iter_mut().enumerate() {
            println!(
                "   Layer Set {}: gradient {:.1} K/km, starting temp {:.1}K ({:.1}°C)",
                layer_set_index,
                layer_set.thermal_gradient_k_per_km,
                current_temperature,
                current_temperature - 273.15
            );

            let layer_start_temperature = current_temperature;

            for (h3_cell, column) in &mut layer_set.layers {
                for (depth_index, cell) in column.cells.iter_mut().enumerate() {
                    // Calculate depth within this layer set
                    let depth_in_layer_km =
                        cell.top_km - layer_set.start_height_km + cell.height_km / 2.0;

                    // Calculate temperature: start_temp + gradient * depth_in_layer
                    let cell_temperature = layer_start_temperature
                        + layer_set.thermal_gradient_k_per_km * depth_in_layer_km;

                    // Create new cell with correct temperature (immutable pattern)
                    let new_cell = crate::sim::energy_mass_cell::EnergyMassCell::with_temperature(
                        cell,
                        cell_temperature,
                    );
                    *cell = new_cell;

                    // Debug: thermal gradient applied to cell
                }
            }

            // Calculate temperature at bottom of this layer set for next layer
            let layer_thickness_km = layer_set
                .layers
                .values()
                .next()
                .map(|column| column.cells.len() as f64 * column.cells[0].height_km)
                .unwrap_or(0.0);
            current_temperature =
                layer_start_temperature + layer_set.thermal_gradient_k_per_km * layer_thickness_km;

            println!(
                "     Layer Set {} complete: bottom temp {:.1}K ({:.1}°C)",
                layer_set_index,
                current_temperature,
                current_temperature - 273.15
            );
        }

        println!("✅ Layer-specific thermal gradients applied successfully");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy_mass::energy_mass::EnergyMass;
    use crate::sim::layer_set::LayerSetParams;
    use h3o::Resolution;
}
