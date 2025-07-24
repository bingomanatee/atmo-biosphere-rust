use crate::deprecated::sim::energy_mass_cell::EnergyMassCell;
use crate::utils::h3_utils::H3Utils;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::material::MaterialPhase;
use crate::constants::{REFERENCE_PRESSURE_PA, MIN_PRESSURE_DIFFERENCE_PA, MAX_PRESSURE_DIFFERENCE_PA, THERMAL_CONDUCTION_MODIFIER_SCALE, THERMAL_EXPANSIVITY_SCALE};

/// Conductivity implementation for EnergyMassCell
impl EnergyMassCell {
    /// Get the cached conductivity, recomputing if necessary
    /// Returns: thermal conductivity in W/(m·K)
    pub fn get_conductivity_w_m_k(&mut self) -> f64 {
        if self.conductivity_w_m_k == 0.0 {
            self.conductivity_w_m_k = self.calculate_pressure_adjusted_conductivity_w_m_k();
        }
        self.conductivity_w_m_k
    }

    /// Manually force recomputation of conductivity
    pub fn recompute_conductivity(&mut self) {
        self.conductivity_w_m_k = self.calculate_pressure_adjusted_conductivity_w_m_k();
    }

    /// Calculate pressure-adjusted thermal conductivity for this cell
    /// Returns: thermal conductivity in W/(m·K)
    pub fn calculate_pressure_adjusted_conductivity_w_m_k(&self) -> f64 {
        let material = self.material();

        // Destructure material properties directly and convert in place
        let MaterialPhase {
            thermal_conductivity_w_m_k,
            thermal_conduction_modifier_dimensionless,
            thermal_expansivity_per_k,
            bulk_modulus_pa,
            ..
        } = *material;

        // Calculate pressure difference and clamp it for stability
        let pressure_difference = self.pressure_pa - REFERENCE_PRESSURE_PA;
        let clamped_pressure_diff = pressure_difference.clamp(MIN_PRESSURE_DIFFERENCE_PA, MAX_PRESSURE_DIFFERENCE_PA);

        // Calculate beta coefficient (convert thermal_expansivity_per_k from scaled storage)
        let beta_coefficient = (thermal_expansivity_per_k as f64 / THERMAL_EXPANSIVITY_SCALE) * bulk_modulus_pa;

        // Calculate pressure-adjusted conductivity
        let pressure_adjustment = 1.0 + beta_coefficient * (clamped_pressure_diff / bulk_modulus_pa);

        // Convert and apply all factors in one expression
        (thermal_conductivity_w_m_k as f64) * (thermal_conduction_modifier_dimensionless as f64 / THERMAL_CONDUCTION_MODIFIER_SCALE) * pressure_adjustment
    }

    /// Calculate shared contact area between this cell and a neighbor
    fn calculate_shared_contact_area(&self, neighbor_is_vertical: bool) -> f64 {
        if neighbor_is_vertical {
            // For vertical neighbors (above/below), use horizontal footprint area
            // Keep in km² and convert to m² only at the end
            let area_km2 = self.area();
            area_km2 * 1e6 // Convert km² to m²
        } else {
            // For horizontal neighbors, use shared edge length * vertical thickness
            // Keep everything in km until final conversion
            let resolution = self.cell_index.resolution();
            let shared_edge_length_km = H3Utils::estimate_cell_edge_length_km(resolution, self.planet_radius_km);
            let vertical_thickness_km = self.height_km;
            let area_km2 = shared_edge_length_km * vertical_thickness_km;
            area_km2 * 1e6 // Convert km² to m²
        }
    }

    /// Calculate center-to-center distance between this cell and a neighbor
    fn calculate_center_to_center_distance(&self, neighbor: &EnergyMassCell, neighbor_is_vertical: bool) -> f64 {
        if neighbor_is_vertical {
            // For vertical neighbors, use vertical cell thickness
            // Keep in km and convert to meters only at the end
            let distance_km = self.height_km;
            distance_km * 1000.0 // Convert km to m
        } else {
            // For horizontal neighbors, use horizontal center-to-center distance
            // H3Utils returns meters, so this is unavoidable
            H3Utils::cell_distance_m(self.cell_index, neighbor.cell_index, self.planet_radius_km)
        }
    }

    /// Calculate conductance coefficient between this cell and a neighbor
    /// Uses a default timestep of 1 hour (3600 seconds)
    pub fn calculate_conductance_coefficient(
        &mut self,
        neighbor: &mut EnergyMassCell,
        neighbor_is_vertical: bool,
    ) -> f64 {
        self.calculate_conductance_coefficient_with_timestep(neighbor, neighbor_is_vertical, 3600.0)
    }

    /// Calculate conductance coefficient between this cell and a neighbor with custom timestep
    pub fn calculate_conductance_coefficient_with_timestep(
        &mut self,
        neighbor: &mut EnergyMassCell,
        neighbor_is_vertical: bool,
        timestep_seconds: f64,
    ) -> f64 {
        // Get pressure-adjusted conductivities for both cells
        let conductivity_self = self.get_conductivity_w_m_k();
        let conductivity_neighbor = neighbor.get_conductivity_w_m_k();

        // Calculate interface conductivity (average)
        let interface_conductivity = (conductivity_self + conductivity_neighbor) / 2.0;

        // Calculate shared contact area
        let shared_contact_area = self.calculate_shared_contact_area(neighbor_is_vertical);

        // Calculate center-to-center distance
        let center_to_center_distance = self.calculate_center_to_center_distance(neighbor, neighbor_is_vertical);

        // Calculate conductance coefficient
        let conductance_coefficient = interface_conductivity
            * shared_contact_area
            / center_to_center_distance
            * timestep_seconds;

        conductance_coefficient
    }

    /// Transmit energy to another cell and update pending state
    /// Uses default timestep of 1 hour
    pub fn transmit_energy(&mut self, other: &mut EnergyMassCell, neighbor_is_vertical: bool) {
        self.transmit_energy_with_timestep(other, neighbor_is_vertical, 3600.0);
    }

    /// Transmit energy to another cell and update pending state with custom timestep
    pub fn transmit_energy_with_timestep(&mut self, other: &mut EnergyMassCell, neighbor_is_vertical: bool, timestep_seconds: f64) {
        // Calculate conductance coefficient
        let conductance_coefficient = self.calculate_conductance_coefficient_with_timestep(other, neighbor_is_vertical, timestep_seconds);

        // Calculate temperature difference
        let temp_diff = other.temperature_kelvin() - self.temperature_kelvin();

        // Calculate energy transfer
        let energy_transfer = conductance_coefficient * temp_diff;

        // Update pending energy delta
        self.pending_energy_delta += energy_transfer;
        other.pending_energy_delta -= energy_transfer;
    }

    /// Commit pending energy changes
    pub fn commit(&mut self) {
        if self.pending_energy_delta != 0.0 {
            if self.pending_energy_delta > 0.0 {
                self.add_energy_joules(self.pending_energy_delta);
            } else {
                self.remove_energy_joules(-self.pending_energy_delta);
            }
            self.pending_energy_delta = 0.0;
        }
    }

    /// Get pending energy delta
    pub fn pending_energy_delta(&self) -> f64 {
        self.pending_energy_delta
    }
}
