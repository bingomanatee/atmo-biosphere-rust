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

    // Pending energy state for heat transfer calculations
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
    fn get_material_phase(&self) -> Result<MaterialPhase, String> {
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
        let initial_mass_kg = initial_material.calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: props.pressure_pa,
            volume_km3,
            temperature_k: props.temperature_kelvin,
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
        let final_mass_kg = if correct_phase != initial_phase {
            final_material.calculate_mass_from_pressure_volume(MassCalculationParams {
                pressure_pa: props.pressure_pa,
                volume_km3,
                temperature_k: props.temperature_kelvin,
            })
        } else {
            initial_mass_kg
        };

        // Calculate energy based on temperature and mass with correct phase
        let energy_joules = final_mass_kg * final_material.specific_heat_capacity_j_per_kg_k as f64 * props.temperature_kelvin;

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
        self.energy_joules = energy_joules;
    }

    fn set_temperature_kelvin(&mut self, temperature_kelvin: f64) {
        // Fiat operation - bypasses energy bank and instantly sets phase and mass

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

// Additional methods for conductivity invalidation
impl EnergyMassCell {
    /// Invalidate cached conductivity when dependent properties change
    pub fn invalidate_conductivity(&mut self) {
        self.conductivity_w_m_k = 0.0;
    }
}
