use crate::energy_mass::energy_mass::EnergyMass;
use crate::material::material::MassCalculationParams;
use crate::material::{MaterialPhase, MaterialPhases, MaterialsLoader};
use crate::sim::simulation::SimulationState::Paused;
use crate::utils::h3_utils::H3Utils;
use h3o::CellIndex;

///
/// note - energy mass cell is a region in the h3o system with a cell, height and volume.
/// the area is estimated as a cylinder with the radius determined by the cell and the planet radius 
/// 
pub struct EnergyMassCell {
    cell_index: CellIndex,
    energy_joules: f64,
    mass_kg: f64,
    material_name: String,
    pub material_phase: MaterialPhases,
    pressure_pa: f64,

    // Energy bank for phase transitions
    pub phase_transition_energy_bank: f64,

    pub height_km: f64,
    pub top_km: f64,
    pub bottom_km: f64,
    pub planet_radius_km: f64,
}

pub struct EnergyMassCellProps {
    pub cell_index: CellIndex,
    pub temperature_kelvin: f64,
    pub pressure_pa: f64,
    pub height_km: f64,
    pub top_km: f64,
    pub material_name: String,
    pub material_phase: MaterialPhases,
    pub planet_radius_km: f64,
}

impl EnergyMassCell {
    fn get_material_phase(&self) -> Result<MaterialPhase, String> {
        MaterialsLoader::get_phase_properties(&self.material_name, self.material_phase)
    }

    /// Calculate effective melting temperature based on current pressure
    fn effective_melt_temp(&self) -> f64 {
        let material = self.material();
        let base_melt_temp = material.melt_temp as f64;
        let pressure_diff = self.pressure_pa - 101325.0; // Standard atmospheric pressure

        // Use Clausius-Clapeyron relation: dT/dP = T*ΔV/ΔH
        // For most materials, higher pressure raises melting point
        let pressure_slope = 0.0001; // Default slope in K/Pa - should come from material properties
        base_melt_temp + (pressure_slope * pressure_diff)
    }

    /// Calculate effective boiling temperature based on current pressure
    fn effective_boil_temp(&self) -> f64 {
        let material = self.material();
        let base_boil_temp = material.boil_temp as f64;
        let pressure_diff = self.pressure_pa - 101325.0;

        // Higher pressure always raises boiling point
        let pressure_slope = 0.0003; // Default slope in K/Pa - should come from material properties
        base_boil_temp + (pressure_slope * pressure_diff)
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
        let pressure_diff = pressure - 101325.0;

        let effective_melt = base_melt_temp + (0.0001 * pressure_diff);
        let effective_boil = base_boil_temp + (0.0003 * pressure_diff);

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
        let material =
            MaterialsLoader::get_phase_properties(&props.material_name, props.material_phase)
                .unwrap();

        let area = H3Utils::cell_area(props.cell_index.resolution(), 3390.0);
        let volume_km3 = area * props.height_km;

        // Calculate mass using the MaterialPhase method
        let mass_kg = material.calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: props.pressure_pa,
            volume_km3,
            temperature_k: props.temperature_kelvin,
        });

        let energy_joules = mass_kg * material.specific_heat_capacity_j_per_kg_k as f64;

        EnergyMassCell {
            cell_index: props.cell_index,
            energy_joules,
            mass_kg,
            material_name: props.material_name,
            material_phase: props.material_phase,
            height_km: props.height_km,
            top_km: props.top_km,
            bottom_km: props.height_km + props.top_km,
            pressure_pa: props.pressure_pa,
            phase_transition_energy_bank: 0.0,
            planet_radius_km: props.planet_radius_km,
        }
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

    fn material(&self) -> MaterialPhase {
        MaterialsLoader::get_phase_properties(&self.material_name, self.material_phase)
            .unwrap_or_else(|e| panic!("Failed to get material phase: {}", e))
    }
    fn temperature_kelvin(&self) -> f64 {
        // Use active energy + half the banked energy for gradual temperature shift during phase transitions
        let effective_energy = self.energy_joules + (self.phase_transition_energy_bank * 0.5);
        effective_energy
            / self.mass_kg()
            / self.material().specific_heat_capacity_j_per_kg_k as f64
    }

    fn pressure_pa(&self) -> f64 {
        self.pressure_pa
    }

    fn set_pressure_pa(&mut self, pressure_pa: f64) {
        self.pressure_pa = pressure_pa;
        self.mass_kg = self
            .material()
            .calculate_mass_from_pressure_volume(MassCalculationParams {
                pressure_pa,
                volume_km3: self.volume_km3(),
                temperature_k: self.temperature_kelvin(),
            });
    }

    fn set_energy_joules(&mut self, energy_joules: f64) {
        self.energy_joules = energy_joules;
    }

    fn set_temperature_kelvin(&mut self, temperature_kelvin: f64) {
        // Fiat operation - bypasses energy bank and instantly sets phase and mass

        // Determine what phase the material should be at this temperature and current pressure
        let new_phase = self.determine_phase_at_conditions(temperature_kelvin, self.pressure_pa);

        // Update phase if it changed
        if new_phase != self.material_phase {
            self.material_phase = new_phase;

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
    }

    fn add_energy_joules(&mut self, energy_joules: f64) {
        if energy_joules.is_nan() {
            panic!("energy_joules is NaN");
        }
        if energy_joules < 0.0 {
            panic!("cannot add negative energy to energy_mass_cell");
        }

        // Check if we're crossing a phase transition boundary
        if let Some((_border_energy, bank_energy_delta, main_energy_delta)) = self.check_energy_distribution(energy_joules) {
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
            // Normal temperature change - no phase transition
            self.energy_joules += energy_joules;
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
                self.energy_joules = (self.energy_joules - excess_energy).max(0.0);
            }
        } else {
            // Normal temperature change (cooling) - no phase transition
            self.energy_joules = (self.energy_joules - energy_joules).max(0.0);
        }
    }
}
