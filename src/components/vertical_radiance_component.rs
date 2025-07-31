use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::simulation::{Component, GeologicalCellData, Simulation, SimulationConfig};

/// Optimized radiance component that only calculates vertical (up/down) radiance
/// Horizontal energy transfer is handled by simple blending in the simulation
pub struct VerticalRadianceComponent {
    /// Emissivity for radiance calculations
    emissivity: f64,
}

impl VerticalRadianceComponent {
    /// Create new vertical radiance component with default emissivity
    pub fn new() -> Self {
        Self {
            emissivity: 0.95, // High emissivity for geological materials
        }
    }
    
    /// Create with custom emissivity
    pub fn with_emissivity(emissivity: f64) -> Self {
        Self {
            emissivity: emissivity.clamp(0.0, 1.0),
        }
    }
    
    /// Calculate vertical radiance transfer using Stefan-Boltzmann law
    fn calculate_vertical_radiance_transfer(
        &self,
        source_data: &GeologicalCellData,
        target_data: &GeologicalCellData,
        contact_area_m2: f64,
        time_step_years: f64,
    ) -> f64 {
        // Stefan-Boltzmann constant (W/m²/K⁴)
        const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;
        
        let source_temp = source_data.temperature_k;
        let target_temp = target_data.temperature_k;
        
        // Early exit for small temperature differences (optimization)
        let temp_diff = (source_temp - target_temp).abs();
        if temp_diff < 1.0 { // Less than 1K difference
            return 0.0;
        }
        
        // Net radiant heat transfer: Q = ε * σ * A * (T₁⁴ - T₂⁴)
        let source_temp4 = source_temp.powi(4);
        let target_temp4 = target_temp.powi(4);
        
        let net_power = self.emissivity * STEFAN_BOLTZMANN * contact_area_m2 * 
                       (source_temp4 - target_temp4);
        
        // Convert to energy over time step
        const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = time_step_years * SECONDS_PER_YEAR;
        
        net_power * time_step_seconds // Joules
    }
    
    /// Calculate maximum energy available for transfer (prevent overcooling)
    fn calculate_max_energy_for_transfer(
        &self,
        source_data: &GeologicalCellData,
        target_data: &GeologicalCellData,
    ) -> f64 {
        if source_data.temperature_k <= target_data.temperature_k {
            return 0.0; // No energy available for transfer
        }
        
        // Calculate energy to cool source to target temperature
        let temp_difference = source_data.temperature_k - target_data.temperature_k;
        let specific_heat = 1000.0; // J/kg/K (simplified)
        let mass_kg = source_data.energy_mass.mass_kg();
        
        // Only allow transfer of half the temperature difference to prevent oscillation
        (temp_difference * 0.5) * specific_heat * mass_kg
    }
}

impl Component for VerticalRadianceComponent {
    fn name(&self) -> &'static str {
        "VerticalRadianceComponent"
    }
    
    fn initialize(&mut self, _coll_mgr: &mut CollectionsManager, _config: &SimulationConfig) {
        println!("🌟 VerticalRadianceComponent: Initializing vertical-only thermal radiance...");
        println!("   • Emissivity: {:.2}", self.emissivity);
        println!("   • Horizontal energy transfer handled by simulation blending");
    }
    
    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, _year: f64, config: &SimulationConfig) {
        let cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
            .expect("geological_cells collection should exist");

        let time_step_years = config.years_per_step as f64;
        let mut transfers_calculated = 0;

        // Only print occasionally to reduce console noise
        if step % 1000 == 0 {
            println!("🌟 VerticalRadianceComponent: Processing {} cells for vertical radiance", cells.len());
        }

        // OPTIMIZATION: Group cells by H3 index for vertical column processing
        let mut columns: std::collections::HashMap<h3o::CellIndex, Vec<(CellLocation, GeologicalCellData)>> =
            std::collections::HashMap::new();

        // Group cells into vertical columns
        for entry in cells.iter() {
            let cell_location = *entry.key();
            let cell_data = (*entry.value()).clone();
            let h3_index = cell_location.h3_cell_index();

            columns.entry(h3_index)
                .or_insert_with(Vec::new)
                .push((cell_location, cell_data));
        }

        // Process each vertical column
        for (_h3_index, mut column) in columns {
            // Sort by depth (surface to deep)
            column.sort_by_key(|(location, _)| (location.layer_set_index(), location.depth_index()));

            // Process adjacent pairs in the vertical column
            for window in column.windows(2) {
                let (upper_location, upper_data) = &window[0];
                let (lower_location, lower_data) = &window[1];

                let contact_area_m2 = 1000.0; // Simplified vertical contact area

                let energy_transfer = self.calculate_vertical_radiance_transfer(
                    upper_data, lower_data, contact_area_m2, time_step_years
                );

                if energy_transfer.abs() > 1e6 {
                    // Limit transfer to prevent overcooling
                    let max_transfer = self.calculate_max_energy_for_transfer(upper_data, lower_data);
                    let actual_transfer = energy_transfer.abs().min(max_transfer);

                    if energy_transfer > 0.0 {
                        // Upper cell is hotter, transfers energy downward
                        actor.add("geological_cells", *upper_location, "energy_joules", -actual_transfer);
                        actor.add("geological_cells", *lower_location, "energy_joules", actual_transfer);
                    } else {
                        // Lower cell is hotter, transfers energy upward
                        actor.add("geological_cells", *lower_location, "energy_joules", -actual_transfer);
                        actor.add("geological_cells", *upper_location, "energy_joules", actual_transfer);
                    }
                    transfers_calculated += 1;
                }
            }
        }

        if transfers_calculated > 0 && step % 1000 == 0 {
            println!("🌟 VerticalRadianceComponent: Calculated {} vertical energy transfers at step {}", 
                     transfers_calculated, step);
        }
    }
    
    fn complete(&mut self, _sim: &Simulation, _config: &SimulationConfig) {
        println!("🌟 VerticalRadianceComponent: Vertical radiance processing complete");
    }
}

impl Default for VerticalRadianceComponent {
    fn default() -> Self {
        Self::new()
    }
}
