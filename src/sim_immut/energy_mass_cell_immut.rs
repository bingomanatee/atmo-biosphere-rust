use std::sync::Arc;
use crate::material::materials_loader::MaterialsLoader;
use crate::material::material::{MaterialPhase, MassCalculationParams};
use crate::material::MaterialPhases;
use crate::energy_mass::energy_mass::EnergyMass;
use h3o::CellIndex;

/// Immutable Energy/Mass Cell - constructor-based approach for better performance
/// Instead of mutating cells in-place, create new cells with modified properties
#[derive(Debug, Clone)]
pub struct EnergyMassCellImmut {
    pub cell_index: CellIndex,
    pub energy_joules: f64,
    pub mass_kg: f64,
    pub material_name: String,
    pub material_phase: MaterialPhases,
    pub height_km: f64,
    pub top_km: f64,
    pub bottom_km: f64,
    pub pressure_pa: f64,
    pub phase_transition_energy_bank: f64,
    pub planet_radius_km: f64,
    pub conductivity_w_m_k: f64, // Cached value, 0.0 means needs recalculation
}

/// Properties for creating a new ImmutableEnergyMassCell
#[derive(Debug, Clone)]
pub struct EnergyMassCellImmutProps {
    pub cell_index: CellIndex,
    pub height_km: f64,
    pub top_km: f64,
    pub material_name: String,
    pub temperature_kelvin: f64,
    pub pressure_pa: f64,
    pub planet_radius_km: f64,
}

impl EnergyMassCellImmut {
    /// Create a new immutable energy/mass cell
    pub fn new(props: EnergyMassCellImmutProps) -> Self {
        let bottom_km = props.top_km + props.height_km;
        let volume_km3 = Self::calculate_volume_km3(props.cell_index, props.height_km, props.planet_radius_km);
        
        // Determine phase at given conditions
        let material_phase = Self::determine_phase_at_conditions_static(
            &props.material_name, props.temperature_kelvin, props.pressure_pa
        );

        // Get phase-specific material properties
        let phase_material = MaterialsLoader::get_phase_properties(&props.material_name, material_phase)
            .expect("Failed to load phase-specific material properties");

        // Calculate mass using reference temperature for consistency
        let reference_temperature_k = 300.0;
        let mass_kg = phase_material.calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: props.pressure_pa,
            volume_km3,
            temperature_k: reference_temperature_k,
        });

        // Calculate energy based on actual temperature and mass
        let energy_joules = mass_kg * phase_material.specific_heat_capacity_j_per_kg_k as f64 * props.temperature_kelvin;
        
        EnergyMassCellImmut {
            cell_index: props.cell_index,
            energy_joules,
            mass_kg,
            material_name: props.material_name,
            material_phase,
            height_km: props.height_km,
            top_km: props.top_km,
            bottom_km,
            pressure_pa: props.pressure_pa,
            phase_transition_energy_bank: 0.0,
            planet_radius_km: props.planet_radius_km,
            conductivity_w_m_k: 0.0,
        }
    }
    
    /// Create a new cell with modified temperature (immutable constructor pattern)
    pub fn with_temperature(&self, new_temperature_kelvin: f64) -> Self {
        // Temperature change applied via immutable pattern

        // Determine phase at new temperature and current pressure
        let new_phase = Self::determine_phase_at_conditions_static(
            &self.material_name, new_temperature_kelvin, self.pressure_pa
        );
        
        // Get material properties for the new phase
        let new_material = MaterialsLoader::get_phase_properties(&self.material_name, new_phase)
            .expect("Failed to load material properties for new phase");
        
        // Calculate new mass if phase changed
        let new_mass_kg = if new_phase != self.material_phase {
            new_material.calculate_mass_from_pressure_volume(MassCalculationParams {
                pressure_pa: self.pressure_pa,
                volume_km3: self.volume_km3(),
                temperature_k: new_temperature_kelvin,
            })
        } else {
            self.mass_kg
        };

        // Calculate new energy based on new temperature and mass: E = m * c * T
        let new_energy_joules = new_mass_kg * new_material.specific_heat_capacity_j_per_kg_k as f64 * new_temperature_kelvin;

        let new_cell = Self {
            cell_index: self.cell_index,
            energy_joules: new_energy_joules,
            mass_kg: new_mass_kg,
            material_name: self.material_name.clone(),
            material_phase: new_phase,
            height_km: self.height_km,
            top_km: self.top_km,
            bottom_km: self.bottom_km,
            pressure_pa: self.pressure_pa,
            phase_transition_energy_bank: 0.0,
            planet_radius_km: self.planet_radius_km,
            conductivity_w_m_k: 0.0,
        };

        // Return new cell with updated temperature

        new_cell
    }

    /// Create a new cell with modified mass (immutable constructor pattern)
    pub fn with_mass(&self, new_mass_kg: f64) -> Self {
        let safe_mass = new_mass_kg.max(1.0); // Prevent zero/negative mass
        
        Self {
            cell_index: self.cell_index,
            energy_joules: self.energy_joules,
            mass_kg: safe_mass,
            material_name: self.material_name.clone(),
            material_phase: self.material_phase,
            height_km: self.height_km,
            top_km: self.top_km,
            bottom_km: self.bottom_km,
            pressure_pa: self.pressure_pa,
            phase_transition_energy_bank: self.phase_transition_energy_bank,
            planet_radius_km: self.planet_radius_km,
            conductivity_w_m_k: 0.0, // Invalidate conductivity cache
        }
    }

    /// Create a new cell with modified pressure (immutable constructor pattern)
    pub fn with_pressure(&self, new_pressure_pa: f64) -> Self {
        // Preserve current temperature when pressure changes
        let current_temperature = self.temperature_kelvin();

        // Recalculate mass based on new pressure
        let new_mass_kg = self.material_properties().calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: new_pressure_pa,
            volume_km3: self.volume_km3(),
            temperature_k: current_temperature,
        });

        // Recalculate energy to maintain the same temperature: E = m * c * T
        let material = self.material_properties();
        let new_energy_joules = new_mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * current_temperature;

        // Energy recalculated to preserve temperature during pressure change

        let new_cell = Self {
            cell_index: self.cell_index,
            energy_joules: new_energy_joules,
            mass_kg: new_mass_kg,
            material_name: self.material_name.clone(),
            material_phase: self.material_phase,
            height_km: self.height_km,
            top_km: self.top_km,
            bottom_km: self.bottom_km,
            pressure_pa: new_pressure_pa,
            phase_transition_energy_bank: self.phase_transition_energy_bank,
            planet_radius_km: self.planet_radius_km,
            conductivity_w_m_k: 0.0, // Invalidate conductivity cache
        };

        // Return new cell with preserved temperature

        new_cell
    }

    /// Recalculate mass to fill area based on current pressure and temperature (initialization only)
    pub fn recalculate_mass_to_fill_area(&self) -> Self {
        let current_temperature = self.temperature_kelvin();
        let current_pressure = self.pressure_pa;

        // Recalculate mass to properly fill the cell's volume based on density at current P&T
        let material = self.material_properties();
        let final_mass_kg = material.calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa: current_pressure,
            volume_km3: self.volume_km3(),
            temperature_k: current_temperature,
        });

        // Recalculate energy to maintain current temperature with final mass: E = m * c * T
        let final_energy_joules = final_mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * current_temperature;

        Self {
            cell_index: self.cell_index,
            energy_joules: final_energy_joules,
            mass_kg: final_mass_kg,
            material_name: self.material_name.clone(),
            material_phase: self.material_phase,
            height_km: self.height_km,
            top_km: self.top_km,
            bottom_km: self.bottom_km,
            pressure_pa: self.pressure_pa,
            phase_transition_energy_bank: self.phase_transition_energy_bank,
            planet_radius_km: self.planet_radius_km,
            conductivity_w_m_k: 0.0, // Invalidate conductivity cache
        }
    }

    /// Create a new cell with modified energy (immutable constructor pattern)
    pub fn with_energy(&self, new_energy_joules: f64) -> Self {
        // Ensure minimum energy to prevent zero/negative energy
        let material = self.material_properties();
        let min_energy = self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * 1.0; // Minimum 1K
        let safe_energy = new_energy_joules.max(min_energy);
        
        Self {
            cell_index: self.cell_index,
            energy_joules: safe_energy,
            mass_kg: self.mass_kg,
            material_name: self.material_name.clone(),
            material_phase: self.material_phase,
            height_km: self.height_km,
            top_km: self.top_km,
            bottom_km: self.bottom_km,
            pressure_pa: self.pressure_pa,
            phase_transition_energy_bank: self.phase_transition_energy_bank,
            planet_radius_km: self.planet_radius_km,
            conductivity_w_m_k: self.conductivity_w_m_k,
        }
    }

    /// Helper method to calculate volume
    fn calculate_volume_km3(cell_index: CellIndex, height_km: f64, _planet_radius_km: f64) -> f64 {
        let area_km2 = cell_index.area_km2();
        area_km2 * height_km
    }

    /// Helper method to determine phase at conditions (pressure-dependent)
    fn determine_phase_at_conditions_static(material_name: &str, temperature_k: f64, pressure_pa: f64) -> MaterialPhases {
        // Get solid phase properties to determine base melting point
        let solid_phase = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid)
            .unwrap_or_else(|_| panic!("Material {} solid phase not found", material_name));

        // Calculate pressure-dependent melting point using Clausius-Clapeyron relation
        let pressure_dependent_melting_point = Self::calculate_pressure_dependent_melting_point(
            solid_phase.melt_temp as f64,
            pressure_pa,
            material_name
        );

        // Calculate pressure-dependent boiling point (simplified)
        let pressure_dependent_boiling_point = solid_phase.boil_temp as f64 +
            (pressure_pa - 1e5) * 1e-7; // Rough approximation for boiling point pressure dependence

        // Determine phase based on pressure-adjusted transition points
        if temperature_k > pressure_dependent_boiling_point {
            MaterialPhases::Gas
        } else if temperature_k > pressure_dependent_melting_point {
            MaterialPhases::Liquid
        } else {
            MaterialPhases::Solid
        }
    }

    /// Calculate pressure-dependent melting point using Clausius-Clapeyron relation
    fn calculate_pressure_dependent_melting_point(base_melt_temp_k: f64, pressure_pa: f64, material_name: &str) -> f64 {
        // Reference pressure (1 atmosphere = 1e5 Pa)
        const REFERENCE_PRESSURE_PA: f64 = 1e5;

        // Material-specific Clausius-Clapeyron slope (dT/dP) in K/Pa
        // These values are approximate for geological materials
        let dt_dp_k_per_pa = match material_name {
            "granite" => 3.0e-8,   // Granite: ~30 K/GPa
            "basalt" => 2.5e-8,    // Basalt: ~25 K/GPa
            "olivine" => 2.0e-8,   // Olivine: ~20 K/GPa
            _ => 2.5e-8,           // Default: ~25 K/GPa
        };

        // Calculate pressure difference from reference
        let pressure_diff_pa = pressure_pa - REFERENCE_PRESSURE_PA;

        // Apply Clausius-Clapeyron relation: T_melt(P) = T_melt(P0) + (dT/dP) * (P - P0)
        let pressure_adjusted_melt_temp = base_melt_temp_k + dt_dp_k_per_pa * pressure_diff_pa;

        // Ensure melting point doesn't go below absolute minimum
        pressure_adjusted_melt_temp.max(base_melt_temp_k * 0.8)
    }

    /// Get volume in km³
    pub fn volume_km3(&self) -> f64 {
        Self::calculate_volume_km3(self.cell_index, self.height_km, self.planet_radius_km)
    }

    /// Get area in km²
    pub fn area(&self) -> f64 {
        self.cell_index.area_km2()
    }

    /// Create new cell with energy delta applied (immutable pattern)
    pub fn with_energy_delta(self, energy_delta_joules: f64) -> Self {
        if energy_delta_joules == 0.0 {
            return self;
        }

        let new_energy = (self.energy_joules + energy_delta_joules).max(0.0);

        // Calculate new temperature from energy: T = E / (m * c)
        let material = self.material();
        let _new_temperature = new_energy / (self.mass_kg * material.specific_heat_capacity_j_per_kg_k as f64);

        // Create new cell with updated energy and temperature
        Self {
            energy_joules: new_energy,
            ..self
        }
    }

    /// Create new cell with mass delta applied (immutable pattern)
    pub fn with_mass_delta(self, mass_delta_kg: f64) -> Self {
        if mass_delta_kg == 0.0 {
            return self;
        }

        let new_mass = (self.mass_kg + mass_delta_kg).max(1.0); // Minimum 1kg to avoid division by zero

        // Recalculate energy to maintain temperature when mass changes
        let current_temperature = self.temperature_kelvin();
        let material = self.material();
        let new_energy = new_mass * material.specific_heat_capacity_j_per_kg_k as f64 * current_temperature;

        Self {
            mass_kg: new_mass,
            energy_joules: new_energy,
            ..self
        }
    }

    /// Get material properties (renamed to avoid conflict with trait method)
    pub fn material_properties(&self) -> Arc<crate::material::material::MaterialPhase> {
        MaterialsLoader::get_phase_properties(&self.material_name, self.material_phase)
            .expect("Failed to load material properties")
    }
}

impl EnergyMass for EnergyMassCellImmut {
    fn energy_joules(&self) -> f64 {
        self.energy_joules
    }

    fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    fn volume_km3(&self) -> f64 {
        self.volume_km3()
    }

    fn material(&self) -> Arc<MaterialPhase> {
        MaterialsLoader::get_phase_properties(&self.material_name, self.material_phase)
            .expect("Failed to load material properties")
    }

    fn temperature_kelvin(&self) -> f64 {
        let mass = self.mass_kg();
        let material_props = self.material_properties();
        let specific_heat = material_props.specific_heat_capacity_j_per_kg_k as f64;
        let effective_energy = self.energy_joules + self.phase_transition_energy_bank;

        // Prevent division by zero and ensure minimum realistic temperature
        if mass <= 0.0 || specific_heat <= 0.0 || effective_energy <= 0.0 {
            return 1.0; // Minimum 1K to prevent absolute zero issues
        }

        let calculated_temp = effective_energy / (mass * specific_heat);
        calculated_temp.max(1.0).min(10000.0) // Cap at 10,000K for safety
    }

    fn pressure_pa(&self) -> f64 {
        self.pressure_pa
    }

    fn set_pressure_pa(&mut self, _pressure_pa: f64) {
        panic!("set_pressure_pa called on immutable cell - use with_pressure constructor instead");
    }

    fn set_energy_joules(&mut self, _energy_joules: f64) {
        panic!("set_energy_joules called on immutable cell - use with_energy constructor instead");
    }

    fn set_temperature_kelvin(&mut self, _temperature_kelvin: f64) {
        panic!("set_temperature_kelvin called on immutable cell - use with_temperature constructor instead");
    }

    fn add_energy_joules(&mut self, _energy_joules: f64) {
        panic!("add_energy_joules called on immutable cell - use with_energy constructor instead");
    }

    fn remove_energy_joules(&mut self, _energy_joules: f64) {
        panic!("remove_energy_joules called on immutable cell - use with_energy constructor instead");
    }
}
