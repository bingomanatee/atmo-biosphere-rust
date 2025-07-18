use crate::component::SimComponent;
use std::collections::HashMap;
use crate::sim::layer_set::{LayerSet, LayerSetParams};
use crate::energy_mass::energy_mass::EnergyMass;
use crate::profiling::component_profiler::ComponentProfiler;

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

    fn current_year(&self) -> i64 {
        self.step * self.config.years_per_step as i64
    }

    pub fn step(&mut self) {
        let step = self.step;
        let year = self.current_year();

        // Start simulation timing if first step
        if step == 0 {
            self.profiler.start_simulation();
        }

        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);
        let mut profiler = std::mem::take(&mut self.profiler);

        // Each component handles its own internal organization with timing
        for (component_name, comp) in components.iter_mut() {
            let start = std::time::Instant::now();
            comp.step(self, step, year);
            let duration = start.elapsed();
            profiler.record_method_call(component_name, "step", duration);
        }

        // Apply all pending energy changes from components using proper energy methods
        let start = std::time::Instant::now();
        self.apply_all_pending_energy_changes();
        let duration = start.elapsed();
        profiler.record_method_call("simulation", "apply_pending_energy_changes", duration);

        // Put the components and profiler back
        self.components = components;
        self.profiler = profiler;

        // Increment step counter
        self.step += 1;
    }

    /// Generate performance report for all components
    pub fn generate_performance_report(&mut self) -> String {
        self.profiler.end_simulation();
        self.profiler.generate_report()
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
