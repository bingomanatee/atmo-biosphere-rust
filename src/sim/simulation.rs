use crate::component::SimComponent;
use std::collections::HashMap;
use crate::sim::layer_set::{LayerSet, LayerSetParams};
use crate::sim::transaction_manager::{TransactionManager, Transaction, CellLocation};
use crate::energy_mass::energy_mass::EnergyMass;
use crate::profiling::component_profiler::ComponentProfiler;
use crate::events::{EventEmitter, SimulationEvent};

/// Thermal gradient configuration using a quadratic model
#[derive(Clone)]
pub struct ThermalGradientConfig {
    pub surface_temperature_k: f64,
    pub surface_gradient_k_per_km: f64,    // Gradient at surface (e.g., 25 K/km)
    pub deep_gradient_k_per_km: f64,       // Gradient at reference depth (e.g., 10 K/km)
    pub reference_depth_km: f64,           // Depth where gradient reaches deep value (e.g., 200 km)
}

#[derive(Clone)]
pub struct SimulationConfig {
    pub steps: u64,
    pub years_per_step: f64,
    pub warmup_steps: u64,
    pub layer_set_params: Vec<LayerSetParams>,
    pub thermal_config: ThermalGradientConfig,
}

pub struct Simulation {
    state: SimulationState,
    step: i64,
    steps: u64,
    config: SimulationConfig,
    components: HashMap<&'static str, Box<dyn SimComponent>>,
    pub layer_sets: Vec<LayerSet>,
    pub profiler: ComponentProfiler,
    /// Global plume storage - plumes can be created by any component
    pub plumes: Vec<crate::component::convection_plume_component::ConvectionPlume>,
    /// Next plume ID for unique identification
    pub next_plume_id: u64,
    /// Transaction manager for coordinated mass/energy transfers
    pub transaction_manager: TransactionManager,
    /// Event emitter for decoupled monitoring and logging
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
            profiler: ComponentProfiler::new(),
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

    pub fn load_layer_sets(&mut self) {
        let mut cumulative_bottom_km = 0.0;

        for (layer_index, params) in self.config.layer_set_params.iter().enumerate() {
            // Update start height to be the bottom of the previous layer
            let mut adjusted_params = params.clone();
            adjusted_params.start_height_km = cumulative_bottom_km;

            // Create the layer set with thermal configuration
            let mut layer_set = LayerSet::new_with_thermal_config(
                &adjusted_params,
                &self.config.thermal_config,
            );

            // Calculate pressure adjustments for this layer set
            if layer_index > 0 {
                // Get accumulated mass per km² from all layers above
                let accumulated_mass_per_km2 = self.calculate_accumulated_mass_per_km2(layer_index);
                layer_set.adjust_pressures_for_accumulated_mass(accumulated_mass_per_km2);
            }

            // Update cumulative bottom for next layer
            cumulative_bottom_km += params.column_count as f64 * params.cell_height_km;

            self.layer_sets.push(layer_set);
        }

        // After all layers are created, perform final pressure adjustment pass
        self.adjust_all_pressures_for_mass_above();
    }

    /// Calculate temperature at a given depth using the configured thermal gradient segments
    pub fn calculate_temperature_at_depth(&self, depth_km: f64) -> f64 {
        self.config.thermal_config.calculate_temperature_at_depth(depth_km)
    }

    /// Get access to the thermal configuration for testing
    pub fn thermal_config(&self) -> &ThermalGradientConfig {
        &self.config.thermal_config
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
    pub fn create_plume(&mut self,
                       source_layer_index: usize,
                       source_cell_index: h3o::CellIndex,
                       position: (f64, f64),
                       initial_depth_km: f64,
                       total_energy_joules: f64,
                       total_mass_kg: f64,
                       temperature_k: f64,
                       velocity_km_per_year: f64,
                       buoyancy_force: f64,
                       radius_km: f64) -> u64 {
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
            },
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

        println!("\n🔄 Step {}: Year {} ({:.0} years/step)", step, year, years_per_step);

        // Start simulation timing if first step
        if step == 0 {
            self.profiler.start_simulation();
        }

        // 1. Record baseline snapshots for transaction validation
        let start = std::time::Instant::now();
        self.record_all_baselines();
        let duration = start.elapsed();
        self.profiler.record_method_call("simulation", "record_baselines", duration);

        // 2. Run components to generate transactions
        let start = std::time::Instant::now();
        self.run_components_with_transactions(step, year);
        let duration = start.elapsed();
        self.profiler.record_method_call("simulation", "run_components", duration);

        // 3. Validate and regulate transactions
        let start = std::time::Instant::now();
        let regulated_transactions = if enable_transaction_debug {
            self.transaction_manager.validate_and_regulate_transactions_with_debug(years_per_step, true)
        } else {
            self.transaction_manager.validate_and_regulate_transactions(years_per_step)
        };
        let duration = start.elapsed();
        self.profiler.record_method_call("simulation", "validate_transactions", duration);

        // 3.5. Check if hotspots caused scaling and adapt if needed
        let scaling_detected = self.detect_hotspot_scaling(&regulated_transactions);
        if scaling_detected {
            self.adapt_overpowered_hotspots();
        }

        // 4. Apply regulated transactions to simulation
        let start = std::time::Instant::now();
        self.apply_regulated_transactions(&regulated_transactions);
        let duration = start.elapsed();
        self.profiler.record_method_call("simulation", "apply_transactions", duration);

        // 5. Apply legacy pending energy changes (for components not yet converted)
        let start = std::time::Instant::now();
        self.apply_all_pending_energy_changes();
        let duration = start.elapsed();
        self.profiler.record_method_call("simulation", "apply_pending_energy_changes", duration);

        // Increment step counter
        self.step += 1;
    }

    /// Record baseline snapshots for all cells
    fn record_all_baselines(&mut self) {
        for (layer_index, layer_set) in self.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for depth_index in 0..column.cells.len() {
                    self.record_cell_baseline(layer_index, *h3_cell, depth_index);
                }
            }
        }
    }

    /// Run components with transaction generation
    fn run_components_with_transactions(&mut self, step: i64, year: f64) {
        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);
        let mut profiler = std::mem::take(&mut self.profiler);

        // Each component generates transactions instead of direct changes
        for (component_name, comp) in components.iter_mut() {
            let start = std::time::Instant::now();
            comp.step(self, step, year);
            let duration = start.elapsed();
            profiler.record_method_call(component_name, "step", duration);
        }

        // Put the components and profiler back
        self.components = components;
        self.profiler = profiler;
    }

    /// Detect if hotspot-related transactions were scaled (indicating overpowered hotspots)
    fn detect_hotspot_scaling(&self, regulated_transactions: &[crate::sim::transaction_manager::Transaction]) -> bool {
        let hotspot_scaled_count = regulated_transactions
            .iter()
            .filter(|tx| {
                (tx.source.contains("CoreRadiance") || tx.source.contains("Hotspot")) &&
                tx.description.contains("SCALED")
            })
            .count();

        let total_hotspot_transactions = regulated_transactions
            .iter()
            .filter(|tx| tx.source.contains("CoreRadiance") || tx.source.contains("Hotspot"))
            .count();

        if total_hotspot_transactions > 0 {
            let scaling_percentage = (hotspot_scaled_count as f64 / total_hotspot_transactions as f64) * 100.0;

            if scaling_percentage > 25.0 { // If more than 25% of hotspot transactions were scaled
                println!("🚨 Hotspot scaling detected: {}/{} transactions scaled ({:.1}%)",
                    hotspot_scaled_count, total_hotspot_transactions, scaling_percentage);
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
        self.profiler.end_simulation();
        self.profiler.generate_report()
    }

    /// Generate intermediate performance report (reusable - doesn't end simulation)
    pub fn generate_intermediate_report(&self) -> String {
        self.profiler.generate_report()
    }

    /// Get current performance summary (lightweight, reusable)
    pub fn get_performance_summary(&self) -> String {
        let mut report = String::new();
        let component_summary = self.profiler.get_component_summary();

        report.push_str("📊 Current Performance Summary\n");
        report.push_str("==============================\n");

        // Sort components by total time
        let mut components: Vec<_> = component_summary.values().collect();
        components.sort_by(|a, b| b.total_time().cmp(&a.total_time()));

        for (rank, component) in components.iter().enumerate() {
            report.push_str(&format!("{}. {}: {:.2} ms\n",
                rank + 1, component.component_name, component.total_time_ms()));
        }

        report
    }

    /// Get component-specific performance data (reusable)
    pub fn get_component_performance(&self, component_name: &str) -> Option<String> {
        let component_summary = self.profiler.get_component_summary();

        if let Some(metrics) = component_summary.get(component_name) {
            let mut report = String::new();
            report.push_str(&format!("🔧 {} Performance\n", component_name));
            report.push_str(&format!("Total time: {:.2} ms\n", metrics.total_time_ms()));
            report.push_str(&format!("Methods: {}\n", metrics.methods.len()));

            let mut methods: Vec<_> = metrics.methods.iter().collect();
            methods.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));

            for (method_name, method_metrics) in methods.iter().take(5) {
                let method_time_ms = method_metrics.total_time.as_secs_f64() * 1000.0;
                report.push_str(&format!("  {}: {:.2} ms ({} calls)\n",
                    method_name, method_time_ms, method_metrics.call_count));
            }

            Some(report)
        } else {
            None
        }
    }

    /// Print performance report to console
    pub fn print_performance_report(&mut self) {
        let report = self.generate_performance_report();
        println!("{}", report);
    }

    /// Reset profiling data
    pub fn reset_profiling(&mut self) {
        self.profiler.reset();
    }

    /// Get profiler reference for manual timing
    pub fn profiler_mut(&mut self) -> &mut ComponentProfiler {
        &mut self.profiler
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

        let target_location = if let (Some(layer), Some(h3), Some(depth)) = (target_layer, target_h3, target_depth) {
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
            layer, h3_cell, depth,
            None, None, None,
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
            from_layer, from_h3, from_depth,
            Some(to_layer), Some(to_h3), Some(to_depth),
            energy_delta_joules,
            mass_delta_kg,
            description,
        );
    }

    /// Gateway method: Get cell reference for components to read state
    pub fn get_cell(&self, layer: usize, h3_cell: h3o::CellIndex, depth: usize) -> Option<&crate::sim::energy_mass_cell::EnergyMassCell> {
        self.layer_sets.get(layer)?
            .layers.get(&h3_cell)?
            .cells.get(depth)
    }

    /// Gateway method: Get cell baseline for transaction validation
    pub fn record_cell_baseline(&mut self, layer: usize, h3_cell: h3o::CellIndex, depth: usize) {
        if let Some(cell) = self.get_cell(layer, h3_cell, depth) {
            let location = CellLocation::new(layer, h3_cell, depth);
            let snapshot = crate::sim::transaction_manager::CellSnapshot {
                location: location.clone(),
                mass_kg: cell.mass_kg(),
                energy_joules: cell.energy_joules(),
                temperature_kelvin: cell.temperature_kelvin(),
                pressure_pa: cell.pressure_pa(),
            };
            self.transaction_manager.record_baseline_snapshot(location, snapshot);
        }
    }

    /// Record baseline snapshots of all cells for transaction validation
    fn record_baseline_snapshots(&mut self) {
        use crate::sim::transaction_manager::{CellSnapshot, CellLocation};
        use crate::energy_mass::energy_mass::EnergyMass;

        for (layer_set_index, layer_set) in self.layer_sets.iter().enumerate() {
            for (h3_cell_index, column) in &layer_set.layers {
                for (depth_index, cell) in column.cells.iter().enumerate() {
                    let location = CellLocation::new(layer_set_index, *h3_cell_index, depth_index);
                    let snapshot = CellSnapshot {
                        location: location.clone(),
                        mass_kg: cell.mass_kg(),
                        energy_joules: cell.energy_joules(),
                        temperature_kelvin: cell.temperature_kelvin(),
                        pressure_pa: cell.pressure_pa(),
                    };
                    self.transaction_manager.record_baseline_snapshot(location, snapshot);
                }
            }
        }
    }

    /// Apply regulated transactions to the simulation using 3D cell locations
    fn apply_regulated_transactions(&mut self, transactions: &[crate::sim::transaction_manager::Transaction]) {
        use crate::energy_mass::energy_mass::EnergyMass;
        use crate::sim::transaction_manager::CellLocation;
        use std::collections::HashMap;

        // Group transactions by 3D cell location for efficient application
        let mut energy_changes: HashMap<CellLocation, f64> = HashMap::new();
        let mut mass_changes: HashMap<CellLocation, f64> = HashMap::new();

        for transaction in transactions {
            // Apply to source cell
            if transaction.energy_delta_joules != 0.0 {
                *energy_changes.entry(transaction.source_cell.clone()).or_insert(0.0) += transaction.energy_delta_joules;
            }
            if transaction.mass_delta_kg != 0.0 {
                *mass_changes.entry(transaction.source_cell.clone()).or_insert(0.0) += transaction.mass_delta_kg;
            }

            // Apply to target cell if it exists
            if let Some(ref target_cell) = transaction.target_cell {
                if transaction.energy_delta_joules != 0.0 {
                    *energy_changes.entry(target_cell.clone()).or_insert(0.0) -= transaction.energy_delta_joules;
                }
                if transaction.mass_delta_kg != 0.0 {
                    *mass_changes.entry(target_cell.clone()).or_insert(0.0) -= transaction.mass_delta_kg;
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

        println!("💾 Applied {} transactions: {:.2e}J, {:.2e}kg across {} cells",
            transactions.len(), total_energy_applied, total_mass_applied, cells_modified);
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
}

impl ThermalGradientConfig {
    /// Create a realistic Earth-like thermal gradient configuration
    /// Uses quadratic model: 25 K/km at surface decreasing to 10 K/km at 200 km depth
    pub fn earth_like(surface_temperature_k: f64) -> Self {
        Self {
            surface_temperature_k,
            surface_gradient_k_per_km: 25.0,
            deep_gradient_k_per_km: 10.0,
            reference_depth_km: 200.0,
        }
    }

    /// Calculate temperature at a given depth using quadratic gradient model
    ///
    /// The gradient decreases quadratically from surface_gradient to deep_gradient:
    /// gradient(depth) = surface_gradient - (surface_gradient - deep_gradient) * (depth/ref_depth)²
    ///
    /// Temperature is the integral of this gradient:
    /// T(depth) = surface_temp + surface_gradient*depth - (surface_gradient - deep_gradient) * depth³/(3*ref_depth²)
    pub fn calculate_temperature_at_depth(&self, depth_km: f64) -> f64 {
        if depth_km <= 0.0 {
            return self.surface_temperature_k;
        }

        let d = depth_km;
        let d_ref = self.reference_depth_km;
        let grad_surf = self.surface_gradient_k_per_km;
        let grad_deep = self.deep_gradient_k_per_km;

        // Clamp depth to reference depth to avoid negative gradients
        let effective_depth = d.min(d_ref);

        // Quadratic gradient model: T = T₀ + grad_surf*d - (grad_surf - grad_deep) * d³/(3*d_ref²)
        let linear_term = grad_surf * effective_depth;
        let quadratic_term = (grad_surf - grad_deep) * effective_depth.powi(3) / (3.0 * d_ref.powi(2));

        let temperature = self.surface_temperature_k + linear_term - quadratic_term;

        // If depth exceeds reference depth, continue with constant deep gradient
        if d > d_ref {
            let extra_depth = d - d_ref;
            temperature + grad_deep * extra_depth
        } else {
            temperature
        }
    }

    /// Calculate the gradient at a specific depth
    pub fn gradient_at_depth(&self, depth_km: f64) -> f64 {
        if depth_km <= 0.0 {
            return self.surface_gradient_k_per_km;
        }

        let d = depth_km.min(self.reference_depth_km);
        let d_ref = self.reference_depth_km;
        let grad_surf = self.surface_gradient_k_per_km;
        let grad_deep = self.deep_gradient_k_per_km;

        // Quadratic decrease: gradient(d) = grad_surf - (grad_surf - grad_deep) * (d/d_ref)²
        let factor = (d / d_ref).powi(2);
        grad_surf - (grad_surf - grad_deep) * factor
    }
}
