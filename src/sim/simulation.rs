use crate::component::SimComponent;
use std::collections::HashMap;
use crate::sim::layer_set::{LayerSet, LayerSetParams};

/// Thermal gradient configuration using a quadratic model
#[derive(Clone)]
pub struct ThermalGradientConfig {
    pub surface_temperature_k: f64,
    pub surface_gradient_k_per_km: f64,    // Gradient at surface (e.g., 25 K/km)
    pub deep_gradient_k_per_km: f64,       // Gradient at reference depth (e.g., 10 K/km)
    pub reference_depth_km: f64,           // Depth where gradient reaches deep value (e.g., 200 km)
}

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

    fn step(&mut self) {
        let step = self.step;
        let year = self.current_year();
        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        for (_, comp) in components.iter_mut() {
            comp.update(self, step, year);
        }

        for (_, comp) in components.iter_mut() {
            comp.report(self, step, year);
        }

        // Put the components back
        self.components = components;
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
