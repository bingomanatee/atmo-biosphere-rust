use crate::component::SimComponent;
use crate::deprecated::sim::Simulation;
use crate::deprecated::sim::energy_mass_cell::EnergyMassCell;
use crate::energy_mass::energy_mass::EnergyMass;

/// Radiative cooling component that removes energy from surface layers to space
///
/// This component simulates blackbody radiation cooling to the vacuum of space.
/// Surface and near-surface layers lose energy based on Stefan-Boltzmann law.
#[derive(Debug, Clone)]
pub struct RadiativeCoolingComponent {
    /// Stefan-Boltzmann constant (W⋅m⁻²⋅K⁻⁴)
    stefan_boltzmann_constant: f64,

    /// Surface emissivity (0.0 to 1.0) - how efficiently surface radiates
    surface_emissivity: f64,

    /// Maximum cooling depth (km) - how deep radiative cooling affects
    max_cooling_depth_km: f64,

    /// Cooling efficiency factor with depth (exponential decay)
    depth_cooling_factor: f64,

    /// Minimum temperature for cooling (K) - prevents cooling below cosmic background
    min_temperature_k: f64,

    /// Performance tracking
    performance_ms: f64,

    /// Total energy radiated to space this step (J)
    total_energy_radiated_j: f64,
}

impl RadiativeCoolingComponent {
    /// Create a new radiative cooling component with realistic parameters
    pub fn new() -> Self {
        Self {
            stefan_boltzmann_constant: 5.670374419e-8, // W⋅m⁻²⋅K⁻⁴
            surface_emissivity: 0.95,      // Rock emissivity ~0.9-0.98
            max_cooling_depth_km: 0.05,    // Radiative cooling affects top ~50m
            depth_cooling_factor: 20.0,    // Exponential decay with depth
            min_temperature_k: 2.7,        // Cosmic microwave background temperature
            performance_ms: 0.0,
            total_energy_radiated_j: 0.0,
        }
    }
    
    /// Create a radiative cooling component with custom parameters
    pub fn with_parameters(
        surface_emissivity: f64,
        max_cooling_depth_km: f64,
        depth_cooling_factor: f64,
        min_temperature_k: f64,
    ) -> Self {
        Self {
            stefan_boltzmann_constant: 5.670374419e-8,
            surface_emissivity,
            max_cooling_depth_km,
            depth_cooling_factor,
            min_temperature_k,
            performance_ms: 0.0,
            total_energy_radiated_j: 0.0,
        }
    }
    
    /// Calculate blackbody radiation power using Stefan-Boltzmann law
    /// P = ε × σ × A × T⁴ (W)
    fn calculate_blackbody_radiation(&self, temperature_k: f64, area_m2: f64) -> f64 {
        if temperature_k <= self.min_temperature_k {
            return 0.0; // No cooling below cosmic background temperature
        }

        // Stefan-Boltzmann law: P = ε × σ × A × T⁴
        self.surface_emissivity
            * self.stefan_boltzmann_constant
            * area_m2
            * temperature_k.powi(4)
    }

    /// Calculate cooling efficiency at depth
    /// Surface cools most efficiently, efficiency decreases exponentially with depth
    fn calculate_cooling_efficiency_at_depth(&self, depth_km: f64) -> f64 {
        if depth_km > self.max_cooling_depth_km {
            return 0.0; // No radiative cooling beyond max depth
        }

        // Exponential decay with depth: efficiency = exp(-factor × depth)
        (-self.depth_cooling_factor * depth_km).exp()
    }

    /// Calculate energy loss rate using "Space as Layer -1" thermal transfer
    fn calculate_energy_loss_rate(&self, cell: &EnergyMassCell, depth_km: f64) -> f64 {
        let cell_temperature_k = cell.temperature_kelvin();
        let area_m2 = cell.area() * 1e6; // Convert km² to m²

        if cell_temperature_k <= self.min_temperature_k {
            return 0.0; // No cooling below cosmic background
        }

        // Only apply to surface layers (< 50m depth)
        if depth_km > 0.05 {
            return 0.0;
        }

        // Cell thickness in meters
        let cell_thickness_m = 10.0;
        let thermal_conductivity = 2.5; // W/(m⋅K) for rock

        // Space as "Layer -1": T = 2.7K, mass = 0, energy = 0
        let space_temperature = self.min_temperature_k; // 2.7K cosmic background

        // Solve for surface temperature where: heat_conduction_in = S-B_radiation_out
        // k × A × (T_cell - T_surface) / thickness = ε × σ × A × T_surface⁴
        let surface_temp = self.solve_surface_temperature_balance(
            cell_temperature_k,
            cell_thickness_m,
            thermal_conductivity,
            area_m2
        );

        // Heat flow rate from cell to surface (this becomes the cooling rate)
        let theoretical_heat_flow = thermal_conductivity * area_m2 *
            (cell_temperature_k - surface_temp) / cell_thickness_m;

        // Apply very conservative geological time scale limiting
        let geological_time_factor = 0.00001; // Limit to 0.001% of theoretical rate
        let actual_cooling_rate = theoretical_heat_flow * geological_time_factor;

        // Additional safety limit: very conservative max cooling rate
        let max_cooling_rate = area_m2 * 10.0; // Only 10 W/m² max

        return actual_cooling_rate.min(max_cooling_rate).max(0.0);
    }

    /// Solve for surface temperature where conduction in = S-B radiation out
    fn solve_surface_temperature_balance(
        &self,
        cell_temp: f64,
        thickness: f64,
        thermal_conductivity: f64,
        area: f64,
    ) -> f64 {
        let conduction_coeff = thermal_conductivity * area / thickness;
        let radiation_coeff = self.surface_emissivity * self.stefan_boltzmann_constant * area;

        // Iteratively solve: k×A×(T_cell - T_surf)/d = ε×σ×A×T_surf⁴
        let mut surface_temp = cell_temp * 0.7; // Initial guess: surface cooler than bulk

        for _ in 0..10 { // Newton-Raphson iterations
            let heat_conduction_in = conduction_coeff * (cell_temp - surface_temp);
            let heat_radiation_out = radiation_coeff * surface_temp.powi(4);
            let imbalance = heat_conduction_in - heat_radiation_out;

            if imbalance.abs() < 1.0 {
                break; // Converged
            }

            // Newton-Raphson step: f'(T) = -conduction_coeff - 4×radiation_coeff×T³
            let derivative = -conduction_coeff - 4.0 * radiation_coeff * surface_temp.powi(3);
            if derivative.abs() > 1e-10 {
                surface_temp -= imbalance / derivative;
            } else {
                // Fallback: simple adjustment
                surface_temp += if imbalance > 0.0 { 1.0 } else { -1.0 };
            }

            // Keep surface temperature reasonable
            surface_temp = surface_temp.clamp(self.min_temperature_k, cell_temp);
        }

        surface_temp
    }
    
    /// Apply radiative cooling to a single cell
    fn apply_radiative_cooling_to_cell(
        &mut self,
        cell: &mut EnergyMassCell,
        depth_km: f64,
        years_per_step: f64,
    ) {
        // Validation: Check input parameters
        debug_assert!(years_per_step > 0.0, "Years per step must be positive");
        debug_assert!(depth_km >= 0.0, "Depth cannot be negative");

        let initial_temp = cell.temperature_kelvin();
        let initial_energy = cell.energy_joules();

        // Validation: Check initial state
        debug_assert!(initial_temp > 0.0, "Temperature must be positive");
        debug_assert!(initial_energy >= 0.0, "Energy cannot be negative");

        // Calculate energy loss rate (W)
        let energy_loss_rate_w = self.calculate_energy_loss_rate(cell, depth_km);

        if energy_loss_rate_w <= 0.0 {
            return; // No cooling
        }

        // Validation: Check cooling rate is reasonable
        let max_reasonable_rate = initial_energy / (years_per_step * 365.25 * 24.0 * 3600.0) * 0.1; // Max 10% energy loss per step
        if energy_loss_rate_w > max_reasonable_rate {
            eprintln!("Warning: Radiative cooling rate ({:.2e} W) exceeds 10% of cell energy per step", energy_loss_rate_w);
        }

        // Convert to energy lost per time step (J)
        let seconds_per_step = years_per_step * 365.25 * 24.0 * 3600.0;
        let energy_lost_joules = energy_loss_rate_w * seconds_per_step;

        // Remove energy from the cell (cooling) but don't go below minimum temperature
        let current_energy = cell.energy_joules();

        // Calculate minimum energy corresponding to cosmic background temperature
        use crate::material::materials_loader::MaterialsLoader;
        let material_name = "basalt"; // Assume basalt for surface
        let material = MaterialsLoader::get_phase_properties(material_name, crate::material::MaterialPhases::Solid)
            .unwrap_or_else(|_| panic!("Material {} not found", material_name));
        let mass_kg = cell.mass_kg();
        let min_energy = mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * self.min_temperature_k;

        // Limit energy loss to prevent going below minimum temperature
        let max_allowable_loss = (current_energy - min_energy).max(0.0);
        let actual_energy_lost = energy_lost_joules.min(max_allowable_loss);
        let new_energy = current_energy - actual_energy_lost;

        cell.set_energy_joules(new_energy);

        // Validation: Check final state
        let final_temp = cell.temperature_kelvin();
        let final_energy = cell.energy_joules();

        debug_assert!(final_temp >= self.min_temperature_k,
                     "Temperature ({:.1}K) dropped below cosmic background ({:.1}K)",
                     final_temp, self.min_temperature_k);
        debug_assert!(final_energy >= 0.0, "Energy cannot be negative after cooling");
        debug_assert!(final_temp <= initial_temp + 1.0,
                     "Temperature should not increase during cooling (was {:.1}K, now {:.1}K)",
                     initial_temp, final_temp);

        // Validation: Check cooling amount is reasonable
        let temp_change = initial_temp - final_temp;
        let max_reasonable_cooling = 100.0; // Max 100K cooling per step
        if temp_change > max_reasonable_cooling {
            eprintln!("Warning: Excessive cooling of {:.1}K in one step (from {:.1}K to {:.1}K)",
                     temp_change, initial_temp, final_temp);
        }

        // Track total energy radiated to space (actual amount lost)
        self.total_energy_radiated_j += actual_energy_lost;
    }
}

impl SimComponent for RadiativeCoolingComponent {
    fn key(&self) -> &'static str {
        "radiative_cooling"
    }

    fn initialize(&mut self, _sim: &mut Simulation) {
        // Reset performance tracking
        self.performance_ms = 0.0;
        self.total_energy_radiated_j = 0.0;
    }

    fn step(&mut self, sim: &mut Simulation, _step: i64, _year: i64) {
        let start_time = std::time::Instant::now();

        // Validation: Check simulation state
        debug_assert!(sim.config.years_per_step > 0.0, "Years per step must be positive");
        debug_assert!(!sim.layer_sets.is_empty(), "Simulation must have layer sets");

        // Reset energy tracking for this step
        self.total_energy_radiated_j = 0.0;
        let mut cells_processed = 0;
        let mut total_initial_energy = 0.0;

        // Calculate years per step from simulation config
        let years_per_step = sim.config.years_per_step;

        // Apply radiative cooling to surface and near-surface layers
        for (layer_idx, layer_set) in sim.layer_sets.iter_mut().enumerate() {
            // Get the layer parameters for this layer set
            let layer_params = &sim.config.layer_set_params[layer_idx];

            for (_h3_cell, column) in &mut layer_set.layers {
                for (cell_idx, cell) in column.cells.iter_mut().enumerate() {
                    // Calculate depth from surface
                    let cell_depth_km = layer_set.start_height_km + (cell_idx as f64 * layer_params.cell_height_km);

                    // Track initial state for validation
                    total_initial_energy += cell.energy_joules();
                    cells_processed += 1;

                    // Only apply radiative cooling to shallow depths
                    if cell_depth_km <= self.max_cooling_depth_km {
                        self.apply_radiative_cooling_to_cell(
                            cell,
                            cell_depth_km,
                            years_per_step,
                        );
                    }
                }
            }
        }

        self.performance_ms = start_time.elapsed().as_millis() as f64;

        // Validation: Check energy conservation and reasonable behavior
        if cells_processed > 0 {
            let avg_energy_per_cell = total_initial_energy / cells_processed as f64;
            let energy_loss_fraction = self.total_energy_radiated_j / total_initial_energy;

            // Warn if we're losing more than 10% of total energy per step
            if energy_loss_fraction > 0.1 {
                eprintln!("Warning: Radiative cooling removed {:.1}% of total energy in one step",
                         energy_loss_fraction * 100.0);
            }

            // Validation: Check performance is reasonable
            if self.performance_ms > 1000.0 {
                eprintln!("Warning: Radiative cooling took {:.1}ms (may be too slow)", self.performance_ms);
            }
        }
    }

    fn complete(&mut self, _sim: &Simulation) {
        // Print final statistics
        println!("Radiative Cooling Component completed:");
        println!("  Total energy radiated to space: {:.2e} J", self.total_energy_radiated_j);
        println!("  Average performance: {:.2} ms per step", self.performance_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_radiative_cooling_component_creation() {
        let component = RadiativeCoolingComponent::new();
        assert_eq!(component.stefan_boltzmann_constant, 5.670374419e-8);
        assert_eq!(component.surface_emissivity, 0.95);
        assert_eq!(component.max_cooling_depth_km, 0.05);
    }

    #[test]
    fn test_blackbody_radiation_calculation() {
        let component = RadiativeCoolingComponent::new();

        // Test at room temperature (300K)
        let power_300k = component.calculate_blackbody_radiation(300.0, 1.0); // 1 m²
        assert!(power_300k > 0.0);

        // Test at higher temperature (600K) - should be much higher (T⁴ law)
        let power_600k = component.calculate_blackbody_radiation(600.0, 1.0);
        assert!(power_600k > power_300k * 10.0); // Should be ~16x higher (2⁴)

        // Test below cosmic background - should be zero
        let power_cold = component.calculate_blackbody_radiation(2.0, 1.0);
        assert_eq!(power_cold, 0.0);
    }

    #[test]
    fn test_cooling_efficiency_at_depth() {
        let component = RadiativeCoolingComponent::new();

        let surface_efficiency = component.calculate_cooling_efficiency_at_depth(0.0);
        assert_eq!(surface_efficiency, 1.0); // Full efficiency at surface

        let deep_efficiency = component.calculate_cooling_efficiency_at_depth(0.1);
        assert_eq!(deep_efficiency, 0.0); // No cooling beyond max depth

        let mid_efficiency = component.calculate_cooling_efficiency_at_depth(0.025);
        assert!(mid_efficiency > 0.0 && mid_efficiency < 1.0); // Partial efficiency
    }

    #[test]
    fn test_surface_cooling_over_time() {
        use crate::sim::simulation::{Simulation, SimulationConfig};
        use crate::sim::layer_set::{LayerSetParams};
        use h3o::Resolution;

        println!("\n🌌 Testing Surface Cooling Over Time");
        println!("====================================");

        // Create a simple simulation with hot surface
        let layer_params = LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 0.0,
            cell_height_km: 0.01, // Very thin cells (10m) to see cooling effect
            material_name: "basalt".to_string(),
            cells_per_column: 5, // Only 5 cells (50m total depth)
            planet_radius_km: 6371.0,
            thermal_gradient_k_per_km: 0.0, // No thermal gradient for clean test
            name: "Test Crust".to_string(),
        };

        let config = SimulationConfig {
            steps: 10,
            years_per_step: 1000.0, // 1000 years per step
            warmup_steps: 0,
            layer_set_params: vec![layer_params],
            surface_temp_k: 500.0, // Hot surface (227°C)
        };

        // Create empty components vector for simulation
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let mut sim = Simulation::new(config, &mut components);

        // Create radiative cooling component
        let mut cooling_component = RadiativeCoolingComponent::new();
        cooling_component.initialize(&mut sim);

        // Get initial surface temperature
        let first_h3_cell = sim.layer_sets[0].layers.keys().next().copied()
            .expect("Should have at least one H3 cell");
        let initial_surface_temp = sim.layer_sets[0].layers[&first_h3_cell].cells[0].temperature_kelvin();

        println!("Initial surface temperature: {:.1}K ({:.1}°C)",
                 initial_surface_temp, initial_surface_temp - 273.15);

        // Track temperature over time
        let mut temperatures = Vec::new();
        temperatures.push(initial_surface_temp);

        // Run simulation with cooling for several steps
        for step in 0..5 {
            // Apply radiative cooling
            cooling_component.step(&mut sim, step, step);

            // Get new surface temperature
            let surface_temp = sim.layer_sets[0].layers[&first_h3_cell].cells[0].temperature_kelvin();
            temperatures.push(surface_temp);

            println!("Step {}: Surface temp = {:.1}K ({:.1}°C), Cooled by {:.1}K",
                     step + 1,
                     surface_temp,
                     surface_temp - 273.15,
                     initial_surface_temp - surface_temp);
        }

        // Verify cooling occurred
        let final_surface_temp = temperatures.last().unwrap();
        let total_cooling = initial_surface_temp - final_surface_temp;

        println!("\n📊 Cooling Results:");
        println!("   Initial: {:.1}K ({:.1}°C)", initial_surface_temp, initial_surface_temp - 273.15);
        println!("   Final:   {:.1}K ({:.1}°C)", final_surface_temp, final_surface_temp - 273.15);
        println!("   Total cooling: {:.1}K over 5000 years", total_cooling);

        // Assertions - more realistic expectations for geological cooling
        assert!(total_cooling > 0.0, "Surface should have cooled");
        assert!(total_cooling < 50.0, "Cooling should be reasonable (< 50K over 5000 years)");
        assert!(*final_surface_temp > 400.0, "Surface should still be quite hot (> 400K)");

        // Check that deeper cells are less affected
        let surface_cell_temp = sim.layer_sets[0].layers[&first_h3_cell].cells[0].temperature_kelvin();
        let deep_cell_temp = sim.layer_sets[0].layers[&first_h3_cell].cells[4].temperature_kelvin();
        let depth_difference = surface_cell_temp - deep_cell_temp;

        println!("   Surface cell: {:.1}K", surface_cell_temp);
        println!("   Deep cell (50m): {:.1}K", deep_cell_temp);
        println!("   Depth difference: {:.1}K", depth_difference.abs());

        // Surface should be cooler than deep cells due to radiative cooling
        assert!(surface_cell_temp < deep_cell_temp,
                "Surface should be cooler than deeper cells due to radiative cooling");

        println!("   ✅ Surface cooling test passed!");
        println!("   ✅ Radiative cooling creates realistic temperature gradient");
    }
}
