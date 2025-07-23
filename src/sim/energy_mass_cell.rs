use std::sync::Arc;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::material::material::MassCalculationParams;
use crate::material::{MaterialPhase, MaterialPhases, MaterialsLoader};
use crate::utils::h3_utils::H3Utils;
use h3o::CellIndex;

///
/// note - energy mass cell is a region in the h3o system with a cell, height and volume.
/// the area is estimated as a cylinder with the radius determined by the cell and the planet radius 
/// 
pub struct EnergyMassCell {
    pub cell_index: CellIndex,
    energy_joules: f64,
    mass_kg: f64,
    material_name: String,
    pub material_phase: MaterialPhases,
    pub pressure_pa: f64,

    // Energy bank for phase transitions
    pub phase_transition_energy_bank: f64,

    pub height_km: f64,
    pub top_km: f64,
    pub bottom_km: f64,
    pub planet_radius_km: f64,

    // Conductivity caching - set to 0.0 when dependent properties change
    // Units: W/(m·K) - thermal conductivity
    pub conductivity_w_m_k: f64,

    // Pending energy changes from all components - accumulated during apply_effects phase
    pub pending_energy_delta: f64,
}

pub struct EnergyMassCellProps {
    pub cell_index: CellIndex,
    pub temperature_kelvin: f64,
    pub pressure_pa: f64,
    pub height_km: f64,
    pub top_km: f64,
    pub material_name: String,
    pub planet_radius_km: f64,
}

impl EnergyMassCell {
    fn get_material_phase(&self) -> Result<Arc<MaterialPhase>, String> {
        MaterialsLoader::get_phase_properties(&self.material_name, self.material_phase)
    }

    /// Calculate effective melting temperature based on current pressure using Clausius-Clapeyron equation
    fn effective_melt_temp(&self) -> f64 {
        let material = self.material();
        let base_melt_temp = material.melt_temp as f64;
        // Clamp to geological pressure range: ~0.1 atm to deep mantle (~1e15 Pa)
        let pressure_diff = (self.pressure_pa - 101325.0).clamp(-90000.0, 1e15);

        // Use Clausius-Clapeyron relation: dT/dP = T*ΔV/L
        // Calculate specific volume change: ΔV = 1/ρ_liquid - 1/ρ_solid
        let rho_solid = material.density_kg_m3 as f64; // Solid density
        let rho_liquid = rho_solid * 0.92; // Water expands ~8% when melting (liquid is less dense)
        let delta_v_specific = (1.0 / rho_liquid) - (1.0 / rho_solid); // m³/kg

        // Latent heat of fusion (J/kg)
        let latent_heat_fusion = material.latent_heat_fusion as f64;

        // Clausius-Clapeyron slope: dT/dP = T*ΔV/L (K/Pa)
        let dt_dp = (base_melt_temp * delta_v_specific) / latent_heat_fusion;

        base_melt_temp + (dt_dp * pressure_diff)
    }

    /// Calculate effective boiling temperature based on current pressure using Clausius-Clapeyron equation
    fn effective_boil_temp(&self) -> f64 {
        let material = self.material();
        let base_boil_temp = material.boil_temp as f64;
        // Clamp to geological pressure range: ~0.1 atm to deep mantle (~1e15 Pa)
        let pressure_diff = (self.pressure_pa - 101325.0).clamp(-90000.0, 1e15);

        // Use Clausius-Clapeyron relation: dT/dP = T*ΔV/L
        // Calculate specific volume change: ΔV = 1/ρ_gas - 1/ρ_liquid
        let rho_liquid = material.density_kg_m3 as f64 * 0.92; // Liquid density (water is ~8% less dense than solid)

        // Gas density at standard conditions (ideal gas approximation)
        // For water vapor at 373K and 1 atm: ρ ≈ 0.598 kg/m³
        let rho_gas = 0.598; // kg/m³ (much less dense than liquid)
        let delta_v_specific = (1.0 / rho_gas) - (1.0 / rho_liquid); // m³/kg

        // Latent heat of vaporization (J/kg)
        let latent_heat_vapor = material.latent_heat_vapor as f64;

        // Clausius-Clapeyron slope: dT/dP = T*ΔV/L (K/Pa)
        let dt_dp = (base_boil_temp * delta_v_specific) / latent_heat_vapor;

        base_boil_temp + (dt_dp * pressure_diff)
    }

    /// Get phase thresholds for energy-based phase determination
    pub fn get_phase_thresholds(&self) -> (f64, f64) {
        let material = self.material();
        let effective_melt = self.effective_melt_temp();
        let effective_boil = self.effective_boil_temp();



        let melt_energy_threshold = self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * effective_melt;
        let boil_energy_threshold = self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * effective_boil;

        (melt_energy_threshold, boil_energy_threshold)
    }

    /// Simple energy-to-phase mapping
    fn energy_to_phase(&self, energy: f64) -> MaterialPhases {
        let (melt_threshold, boil_threshold) = self.get_phase_thresholds();

        match energy {
            e if e < melt_threshold => MaterialPhases::Solid,
            e if e < boil_threshold => MaterialPhases::Liquid,
            _ => MaterialPhases::Gas,
        }
    }

    /// Check energy distribution for phase transitions (handles both positive and negative deltas)
    /// Returns (border_energy, bank_energy_delta, main_energy_delta) or None if no transition
    fn check_energy_distribution(&self, energy_delta: f64) -> Option<(f64, f64, f64)> {
        let material = self.material();
        let current_energy = self.energy_joules;
        let final_energy = current_energy + energy_delta;

        let current_phase = self.energy_to_phase(current_energy);
        let final_phase = self.energy_to_phase(final_energy);

        // No phase transition
        if current_phase == final_phase {
            return None;
        }

        let (melt_threshold, boil_threshold) = self.get_phase_thresholds();

        // Determine which threshold we're crossing
        let (threshold, latent_heat) = match (current_phase, final_phase) {
            (MaterialPhases::Solid, MaterialPhases::Liquid) => (melt_threshold, material.latent_heat_fusion as f64),
            (MaterialPhases::Liquid, MaterialPhases::Gas) => (boil_threshold, material.latent_heat_vapor as f64),
            (MaterialPhases::Liquid, MaterialPhases::Solid) => (melt_threshold, material.latent_heat_fusion as f64),
            (MaterialPhases::Gas, MaterialPhases::Liquid) => (boil_threshold, material.latent_heat_vapor as f64),
            _ => return None, // Skip multi-phase transitions for now
        };

        // Calculate energy distribution based on direction
        let total_latent_heat = self.mass_kg * latent_heat;

        if energy_delta > 0.0 {
            // Heating: moving toward higher energy phase
            let energy_to_threshold = threshold - current_energy;
            let energy_beyond_threshold = final_energy - threshold;

            let bank_energy_delta = energy_beyond_threshold.min(total_latent_heat);
            let main_energy_delta = energy_to_threshold + if energy_beyond_threshold > total_latent_heat {
                energy_beyond_threshold - total_latent_heat
            } else {
                0.0
            };

            Some((threshold, bank_energy_delta, main_energy_delta))
        } else {
            // Cooling: moving toward lower energy phase
            let energy_to_threshold = current_energy - threshold;
            let energy_beyond_threshold = threshold - final_energy;

            let bank_energy_delta = energy_beyond_threshold.min(total_latent_heat);
            let main_energy_delta = -energy_to_threshold - if energy_beyond_threshold > total_latent_heat {
                energy_beyond_threshold - total_latent_heat
            } else {
                0.0
            };

            Some((threshold, bank_energy_delta, main_energy_delta))
        }
    }

    /// Determine what phase material should be at given temperature and pressure
    fn determine_phase_at_conditions(&self, temp: f64, pressure: f64) -> MaterialPhases {
        let material = self.material();

        // Calculate effective transition temperatures for this pressure
        let base_melt_temp = material.melt_temp as f64;
        let base_boil_temp = material.boil_temp as f64;
        // Clamp to geological pressure range: ~0.1 atm to deep mantle (~1e15 Pa)
        let pressure_diff = (pressure - 101325.0).clamp(-90000.0, 1e15);

        // Use Clausius-Clapeyron relation for material-specific pressure effects
        // Calculate melting point pressure effect
        let rho_solid = material.density_kg_m3 as f64;
        let rho_liquid = rho_solid * 0.92; // Water expands ~8% when melting
        let delta_v_melt = (1.0 / rho_liquid) - (1.0 / rho_solid);
        let dt_dp_melt = (base_melt_temp * delta_v_melt) / (material.latent_heat_fusion as f64);
        let effective_melt = base_melt_temp + (dt_dp_melt * pressure_diff);

        // Calculate boiling point pressure effect
        let rho_gas = 0.598; // kg/m³ for water vapor at standard conditions
        let delta_v_boil = (1.0 / rho_gas) - (1.0 / rho_liquid);
        let dt_dp_boil = (base_boil_temp * delta_v_boil) / (material.latent_heat_vapor as f64);
        let effective_boil = base_boil_temp + (dt_dp_boil * pressure_diff);

        if temp < effective_melt {
            MaterialPhases::Solid
        } else if temp > effective_boil {
            MaterialPhases::Gas
        } else {
            MaterialPhases::Liquid
        }
    }

    /// Get energy distribution as (active_energy, banked_energy) tuple
    pub fn energy_distribution(&self) -> (f64, f64) {
        (self.energy_joules, self.phase_transition_energy_bank)
    }
    
    pub fn area(&self) -> f64 {
        H3Utils::cell_area(self.cell_index.resolution(), self.planet_radius_km)
    }

    /// Create a new EnergyMassCell
    pub fn new(props: EnergyMassCellProps) -> Self {
        let area = H3Utils::cell_area(props.cell_index.resolution(), props.planet_radius_km);
        let volume_km3 = area * props.height_km;

        // Start with a default phase (Solid) to get initial material properties
        let initial_phase = MaterialPhases::Solid;
        let initial_material = MaterialsLoader::get_phase_properties(&props.material_name, initial_phase)
            .expect("Failed to load material properties");

        // Calculate initial mass using the default phase
        // Use reference temperature (300K) for mass calculation since mass should primarily
        // depend on material density and volume, not the actual cell temperature
        let reference_temperature_k = 300.0; // Room temperature reference
        let initial_mass_kg = initial_material.calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: props.pressure_pa,
            volume_km3,
            temperature_k: reference_temperature_k,
        });

        // Create temporary cell to use phase determination methods
        let temp_cell = EnergyMassCell {
            cell_index: props.cell_index,
            energy_joules: 0.0, // Will be calculated after phase determination
            mass_kg: initial_mass_kg,
            material_name: props.material_name.clone(),
            material_phase: initial_phase, // Initial phase
            height_km: props.height_km,
            top_km: props.top_km,
            bottom_km: props.height_km + props.top_km,
            pressure_pa: props.pressure_pa,
            phase_transition_energy_bank: 0.0,
            planet_radius_km: props.planet_radius_km,
            conductivity_w_m_k: 0.0,
            pending_energy_delta: 0.0,
        };

        // Determine the correct phase based on temperature and pressure
        let correct_phase = temp_cell.determine_phase_at_conditions(props.temperature_kelvin, props.pressure_pa);

        // Get material properties for the correct phase
        let final_material = MaterialsLoader::get_phase_properties(&props.material_name, correct_phase)
            .expect("Failed to load material properties for correct phase");

        // Recalculate mass with the correct phase if needed
        // Use reference temperature for mass calculation, not actual cell temperature
        let final_mass_kg = if correct_phase != initial_phase {
            final_material.calculate_mass_from_pressure_volume(MassCalculationParams {
                pressure_pa: props.pressure_pa,
                volume_km3,
                temperature_k: reference_temperature_k,
            })
        } else {
            initial_mass_kg
        };

        // Calculate energy based on temperature and mass with correct phase
        let energy_joules = final_mass_kg * final_material.specific_heat_capacity_j_per_kg_k as f64 * props.temperature_kelvin;

        // Energy calculation complete

        EnergyMassCell {
            cell_index: props.cell_index,
            energy_joules,
            mass_kg: final_mass_kg,
            material_name: props.material_name,
            material_phase: correct_phase, // Use the determined phase
            height_km: props.height_km,
            top_km: props.top_km,
            bottom_km: props.height_km + props.top_km,
            pressure_pa: props.pressure_pa,
            phase_transition_energy_bank: 0.0,
            planet_radius_km: props.planet_radius_km,
            conductivity_w_m_k: 0.0, // Will be computed on first access
            pending_energy_delta: 0.0,
        }
    }

    /// Create a new EnergyMassCell with modified temperature (immutable constructor pattern)
    pub fn with_temperature(source: &EnergyMassCell, new_temperature_kelvin: f64) -> EnergyMassCell {
        // Calculate new energy based on new temperature: E = m * c * T
        let material = source.material();
        let new_energy_joules = source.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * new_temperature_kelvin;

        // Determine phase at new temperature and current pressure
        let new_phase = source.determine_phase_at_conditions(new_temperature_kelvin, source.pressure_pa);

        // Calculate new mass if phase changed
        let new_mass_kg = if new_phase != source.material_phase {
            let new_material = MaterialsLoader::get_phase_properties(&source.material_name, new_phase)
                .expect("Failed to load material properties for new phase");
            new_material.calculate_mass_from_pressure_volume(MassCalculationParams {
                pressure_pa: source.pressure_pa,
                volume_km3: source.volume_km3(),
                temperature_k: new_temperature_kelvin,
            })
        } else {
            source.mass_kg
        };

        EnergyMassCell {
            cell_index: source.cell_index,
            energy_joules: new_energy_joules,
            mass_kg: new_mass_kg,
            material_name: source.material_name.clone(),
            material_phase: new_phase,
            height_km: source.height_km,
            top_km: source.top_km,
            bottom_km: source.bottom_km,
            pressure_pa: source.pressure_pa,
            phase_transition_energy_bank: 0.0, // Reset energy bank for new cell
            planet_radius_km: source.planet_radius_km,
            conductivity_w_m_k: 0.0, // Will be computed on first access
            pending_energy_delta: 0.0,
        }
    }

    /// Create a new EnergyMassCell with modified mass (immutable constructor pattern)
    pub fn with_mass(source: &EnergyMassCell, new_mass_kg: f64) -> EnergyMassCell {
        let safe_mass = new_mass_kg.max(1.0); // Prevent zero/negative mass

        EnergyMassCell {
            cell_index: source.cell_index,
            energy_joules: source.energy_joules,
            mass_kg: safe_mass,
            material_name: source.material_name.clone(),
            material_phase: source.material_phase,
            height_km: source.height_km,
            top_km: source.top_km,
            bottom_km: source.bottom_km,
            pressure_pa: source.pressure_pa,
            phase_transition_energy_bank: source.phase_transition_energy_bank,
            planet_radius_km: source.planet_radius_km,
            conductivity_w_m_k: 0.0, // Invalidate conductivity cache
            pending_energy_delta: source.pending_energy_delta,
        }
    }

    /// Create a new EnergyMassCell with modified pressure (immutable constructor pattern)
    pub fn with_pressure(source: &EnergyMassCell, new_pressure_pa: f64) -> EnergyMassCell {
        // Recalculate mass based on new pressure
        let new_mass_kg = source.material().calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: new_pressure_pa,
            volume_km3: source.volume_km3(),
            temperature_k: source.temperature_kelvin(),
        });

        EnergyMassCell {
            cell_index: source.cell_index,
            energy_joules: source.energy_joules,
            mass_kg: new_mass_kg,
            material_name: source.material_name.clone(),
            material_phase: source.material_phase,
            height_km: source.height_km,
            top_km: source.top_km,
            bottom_km: source.bottom_km,
            pressure_pa: new_pressure_pa,
            phase_transition_energy_bank: source.phase_transition_energy_bank,
            planet_radius_km: source.planet_radius_km,
            conductivity_w_m_k: 0.0, // Invalidate conductivity cache
            pending_energy_delta: source.pending_energy_delta,
        }
    }

    /// Create a new EnergyMassCell with modified energy (immutable constructor pattern)
    pub fn with_energy(source: &EnergyMassCell, new_energy_joules: f64) -> EnergyMassCell {
        // Ensure minimum energy to prevent zero/negative energy
        let material = source.material();
        let min_energy = source.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * 1.0; // Minimum 1K
        let safe_energy = new_energy_joules.max(min_energy);

        EnergyMassCell {
            cell_index: source.cell_index,
            energy_joules: safe_energy,
            mass_kg: source.mass_kg,
            material_name: source.material_name.clone(),
            material_phase: source.material_phase,
            height_km: source.height_km,
            top_km: source.top_km,
            bottom_km: source.bottom_km,
            pressure_pa: source.pressure_pa,
            phase_transition_energy_bank: source.phase_transition_energy_bank,
            planet_radius_km: source.planet_radius_km,
            conductivity_w_m_k: source.conductivity_w_m_k,
            pending_energy_delta: source.pending_energy_delta,
        }
    }

    /// Add energy change to pending delta (components should use this instead of direct modification)
    pub fn add_pending_energy_change(&mut self, energy_delta_joules: f64) {
        self.pending_energy_delta += energy_delta_joules;
    }

    /// Get current pending energy change
    pub fn pending_energy_change(&self) -> f64 {
        self.pending_energy_delta
    }

    /// Apply all pending energy changes and reset pending delta to zero
    /// This should only be called by the simulation after all components have applied their effects
    pub fn apply_pending_energy_changes(&mut self) {
        if self.pending_energy_delta != 0.0 {
            self.energy_joules += self.pending_energy_delta;
            self.pending_energy_delta = 0.0;
            self.conductivity_w_m_k = 0.0; // Invalidate conductivity cache
        }
    }

    /// Add or remove mass directly (for plume transport)
    /// This bypasses normal pressure-volume-temperature calculations
    pub fn add_mass_kg(&mut self, mass_delta_kg: f64) {
        self.mass_kg = (self.mass_kg + mass_delta_kg).max(1.0); // Prevent zero/negative mass
        self.conductivity_w_m_k = 0.0; // Invalidate conductivity cache
    }

    /// Get the material name for this cell
    pub fn material_name(&self) -> &str {
        &self.material_name
    }

    /// Reset pending energy changes without applying them (for cleanup)
    pub fn reset_pending_energy_changes(&mut self) {
        self.pending_energy_delta = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_zero_mass_problem_is_fixed() {
        println!("\n🧪 Testing Zero Mass Problem Fix");
        println!("================================");

        // The critical test: 1K temperature should NOT produce zero mass
        let props = EnergyMassCellProps {
            cell_index: h3o::CellIndex::try_from(0x85283473fffffff_u64).unwrap(),
            height_km: 10.0,
            top_km: 0.0,
            material_name: "basalt".to_string(),
            temperature_kelvin: 1.0, // This was causing zero mass
            pressure_pa: 1e5,
            planet_radius_km: 6371.0,
        };

        let cell = EnergyMassCell::new(props);
        let mass = cell.mass_kg();

        println!("Cell with 1K temperature:");
        println!("  Mass: {:.2e} kg", mass);
        println!("  Temperature: {:.1}K", cell.temperature_kelvin());

        // The fix: mass should be > 0 even with 1K temperature
        assert!(mass > 0.0, "CRITICAL: Mass should be > 0 even with 1K temperature");
        assert!(mass > 1e10, "Mass should be substantial for 10km³ of basalt");

        println!("✅ ZERO MASS PROBLEM FIXED!");
        println!("   - 1K temperature produces non-zero mass: {:.2e} kg", mass);
        println!("   - Mass calculation now independent of input temperature");
    }
}

impl EnergyMass for EnergyMassCell {
    fn energy_joules(&self) -> f64 {
        self.energy_joules + self.phase_transition_energy_bank
    }

    fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    fn volume_km3(&self) -> f64 {
        self.area() * self.height_km
    }

    fn material(&self) -> Arc<MaterialPhase> {
        MaterialsLoader::get_phase_properties(&self.material_name, self.material_phase)
            .unwrap_or_else(|e| panic!("Failed to get material phase: {}", e))
    }
    fn temperature_kelvin(&self) -> f64 {
        // Use active energy + half the banked energy for gradual temperature shift during phase transitions
        let effective_energy = self.energy_joules + (self.phase_transition_energy_bank * 0.5);
        let mass = self.mass_kg();
        let specific_heat = self.material().specific_heat_capacity_j_per_kg_k as f64;

        // Prevent division by zero and ensure minimum realistic temperature
        if mass <= 0.0 || specific_heat <= 0.0 || effective_energy <= 0.0 {
            return 1.0; // Minimum 1K to prevent absolute zero issues
        }

        let calculated_temp = effective_energy / (mass * specific_heat);

        // Ensure temperature is realistic (minimum 1K, maximum based on material)
        calculated_temp.max(1.0).min(10000.0) // Cap at 10,000K for safety
    }

    fn pressure_pa(&self) -> f64 {
        self.pressure_pa
    }

    fn set_pressure_pa(&mut self, pressure_pa: f64) {
        self.pressure_pa = pressure_pa;
        self.invalidate_conductivity(); // Invalidate cached conductivity
        self.mass_kg = self
            .material()
            .calculate_mass_from_pressure_volume(MassCalculationParams {
                pressure_pa,
                volume_km3: self.volume_km3(),
                temperature_k: self.temperature_kelvin(),
            });
    }

    fn set_energy_joules(&mut self, energy_joules: f64) {
        // Ensure minimum energy to prevent zero/negative energy
        let min_energy = self.mass_kg * self.material().specific_heat_capacity_j_per_kg_k as f64 * 1.0; // Minimum 1K
        self.energy_joules = energy_joules.max(min_energy);
    }

    fn set_temperature_kelvin(&mut self, temperature_kelvin: f64) {
        // Fiat operation - bypasses energy bank and instantly sets phase and mass
        // Debug: set_temperature_kelvin called

        // Determine what phase the material should be at this temperature and current pressure
        let new_phase = self.determine_phase_at_conditions(temperature_kelvin, self.pressure_pa);

        // Update phase if it changed
        if new_phase != self.material_phase {
            self.material_phase = new_phase;
            self.invalidate_conductivity(); // Invalidate cached conductivity due to phase change

            // Recalculate mass for new phase at current pressure/volume/temperature
            let mass_params = MassCalculationParams {
                pressure_pa: self.pressure_pa,
                volume_km3: self.volume_km3(),
                temperature_k: temperature_kelvin,
            };
            self.mass_kg = self.material().calculate_mass_from_pressure_volume(mass_params);
        }

        // Calculate energy from temperature using E = m * c * T
        let material = self.material();
        self.energy_joules = self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * temperature_kelvin;

        // Clear energy bank since this is a fiat operation
        self.phase_transition_energy_bank = 0.0;

        // Debug: set_temperature_kelvin completed
    }

    fn add_energy_joules(&mut self, energy_joules: f64) {
        if energy_joules.is_nan() {
            panic!("energy_joules is NaN");
        }
        if energy_joules < 0.0 {
            panic!("cannot add negative energy to energy_mass_cell");
        }

        // Apply energy capacity limits based on material properties
        let material = self.material();

        // Use 10x boil_temp as the energy capacity limit (scientific maximum)
        let max_temp = material.boil_temp as f64 * 10.0;

        // Calculate current total energy and potential energy after addition
        let current_energy = self.energy_joules + self.phase_transition_energy_bank;
        let potential_energy = current_energy + energy_joules;
        let potential_temp = potential_energy / (self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64);

        // If adding this energy would exceed the material's temperature capacity, cap it
        let max_energy = max_temp * self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64;
        let capped_energy = if potential_energy > max_energy {
            (max_energy - current_energy).max(0.0) // Only add energy up to the material's capacity
        } else {
            energy_joules // Normal case - no capping needed
        };

        // Only proceed if there's energy to add after capping
        if capped_energy <= 0.0 {
            return; // Cell is already at maximum temperature capacity
        }

        // Check if we're crossing a phase transition boundary (using capped energy)
        if let Some((_border_energy, bank_energy_delta, main_energy_delta)) = self.check_energy_distribution(capped_energy) {
            // We're crossing a phase boundary - distribute energy appropriately
            self.energy_joules += main_energy_delta;
            self.phase_transition_energy_bank += bank_energy_delta;

            // Check if we've accumulated enough energy for phase transition
            let material = self.material();
            let latent_heat_needed = if self.material_phase == MaterialPhases::Solid {
                self.mass_kg * material.latent_heat_fusion as f64
            } else if self.material_phase == MaterialPhases::Liquid {
                self.mass_kg * material.latent_heat_vapor as f64
            } else {
                f64::INFINITY // Gas phase, no further heating transitions
            };

            if self.phase_transition_energy_bank >= latent_heat_needed {
                // Perform phase transition
                self.material_phase = match self.material_phase {
                    MaterialPhases::Solid => MaterialPhases::Liquid,
                    MaterialPhases::Liquid => MaterialPhases::Gas,
                    MaterialPhases::Gas => MaterialPhases::Gas, // No change
                };

                // Move excess banked energy to main energy
                let excess_energy = self.phase_transition_energy_bank - latent_heat_needed;
                self.phase_transition_energy_bank = 0.0;
                self.energy_joules += excess_energy;
            }
        } else {
            // Normal temperature change - no phase transition (using capped energy)
            self.energy_joules += capped_energy;
        }
    }

    fn remove_energy_joules(&mut self, energy_joules: f64) {
        if energy_joules.is_nan() {
            panic!("energy_joules is NaN");
        }
        if energy_joules < 0.0 {
            panic!("cannot subtract negative energy to energy_mass_cell");
        }

        // Check if we're crossing a phase transition boundary (cooling)
        if let Some((_border_energy, bank_energy_delta, main_energy_delta)) = self.check_energy_distribution(-energy_joules) {
            // We're crossing a phase boundary - distribute energy appropriately (consistent addition)
            self.energy_joules += main_energy_delta; // main_energy_delta is negative for cooling
            self.phase_transition_energy_bank += bank_energy_delta; // bank_energy_delta is positive

            // Check if we've accumulated enough energy removal for phase transition
            let material = self.material();
            let latent_heat_needed = if self.material_phase == MaterialPhases::Gas {
                self.mass_kg * material.latent_heat_vapor as f64
            } else if self.material_phase == MaterialPhases::Liquid {
                self.mass_kg * material.latent_heat_fusion as f64
            } else {
                f64::INFINITY // Solid phase, no further cooling transitions
            };

            if self.phase_transition_energy_bank >= latent_heat_needed {
                // Perform phase transition
                self.material_phase = match self.material_phase {
                    MaterialPhases::Gas => MaterialPhases::Liquid,
                    MaterialPhases::Liquid => MaterialPhases::Solid,
                    MaterialPhases::Solid => MaterialPhases::Solid, // No change
                };

                // Continue cooling with excess banked energy
                let excess_energy = self.phase_transition_energy_bank - latent_heat_needed;
                self.phase_transition_energy_bank = 0.0;

                // Ensure minimum energy (1K equivalent)
                let min_energy = self.mass_kg * self.material().specific_heat_capacity_j_per_kg_k as f64 * 1.0;
                self.energy_joules = (self.energy_joules - excess_energy).max(min_energy);
            }
        } else {
            // Normal temperature change (cooling) - no phase transition
            // Ensure minimum energy (1K equivalent)
            let min_energy = self.mass_kg * self.material().specific_heat_capacity_j_per_kg_k as f64 * 1.0;
            self.energy_joules = (self.energy_joules - energy_joules).max(min_energy);
        }
    }
}

// Additional methods for conductivity invalidation
impl EnergyMassCell {
    /// Invalidate cached conductivity when dependent properties change
    pub fn invalidate_conductivity(&mut self) {
        self.conductivity_w_m_k = 0.0;
    }
}
