
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPhase {
    pub density_kg_m3: f32,
    pub specific_heat_capacity_j_per_kg_k: f32,
    pub thermal_conductivity_w_m_k: f32,
    pub thermal_transmission_r0_min: f32,
    pub thermal_transmission_r0_max: f32,
    pub melt_temp: f32,
    pub melt_temp_min: Option<f32>,
    pub melt_temp_max: Option<f32>,
    pub latent_heat_fusion: f32,
    pub boil_temp: f32,
    pub latent_heat_vapor: f32,
    pub gas_interference_factor: Option<f32>,
    pub thermal_conduction_modifier: Option<f32>,
    // Additional fields found in JSON
    pub thermal_expansivity: Option<f32>,
    pub dynamic_viscosity: Option<f64>, // This can be very large (e.g., 1e+25)
    pub bulk_modulus_pa: Option<f64>,   // This is very large (e.g., 130000000000)
    pub activation_energy_j_per_mol: Option<f32>,
    pub activation_volume_m3_per_mol: Option<f32>,
    pub cool_temp_min: Option<f32>,
    pub cool_temp_max: Option<f32>,
}

/// Parameters for mass calculation from pressure and volume
#[derive(Debug, Clone)]
pub struct MassCalculationParams {
    pub pressure_pa: f64,
    pub volume_km3: f64,
    pub temperature_k: f64,
}

/// Parameters for pressure calculation from temperature, volume, and mass
#[derive(Debug, Clone)]
pub struct PressureCalculationParams {
    pub mass_kg: f64,
    pub volume_km3: f64,
    pub temperature_k: f64,
}

impl MassCalculationParams {
    /// Create new parameters with standard conditions as defaults
    pub fn new(pressure_pa: f64, volume_km3: f64, temperature_k: f64) -> Self {
        Self {
            pressure_pa,
            volume_km3,
            temperature_k,
        }
    }

    /// Create parameters at standard temperature and pressure (STP)
    pub fn at_stp(volume_km3: f64) -> Self {
        Self {
            pressure_pa: 101325.0, // Standard atmospheric pressure
            volume_km3,
            temperature_k: 273.15, // 0°C
        }
    }

    /// Create parameters at normal temperature and pressure (NTP)
    pub fn at_ntp(volume_km3: f64) -> Self {
        Self {
            pressure_pa: 101325.0, // Standard atmospheric pressure
            volume_km3,
            temperature_k: 293.15, // 20°C
        }
    }
}

impl PressureCalculationParams {
    /// Create new parameters for pressure calculation
    pub fn new(mass_kg: f64, volume_km3: f64, temperature_k: f64) -> Self {
        Self {
            mass_kg,
            volume_km3,
            temperature_k,
        }
    }

    /// Create parameters at standard temperature (STP)
    pub fn at_stp(mass_kg: f64, volume_km3: f64) -> Self {
        Self {
            mass_kg,
            volume_km3,
            temperature_k: 273.15, // 0°C
        }
    }

    /// Create parameters at normal temperature (NTP)
    pub fn at_ntp(mass_kg: f64, volume_km3: f64) -> Self {
        Self {
            mass_kg,
            volume_km3,
            temperature_k: 293.15, // 20°C
        }
    }
}

impl MaterialPhase {
    /// Calculate mass from pressure and volume using named parameters
    ///
    /// For solids and liquids: Uses density with bulk modulus correction if available
    /// For gases: Uses ideal gas approximation or density at standard conditions
    ///
    /// # Arguments
    /// * `params` - MassCalculationParams containing pressure (Pa), volume (km³), and temperature (K)
    ///
    /// # Returns
    /// Mass in kilograms
    ///

    pub fn calculate_mass_from_pressure_volume(&self, params: MassCalculationParams) -> f64 {
        // Convert volume from km³ to m³ for density calculations (1 km³ = 1e9 m³)
        let volume_m3 = params.volume_km3 * 1e9;

        let base_density = self.density_kg_m3 as f64;
        let reference_temperature = 273.15; // Standard temperature (0°C)
        let min_temperature = 0.1; // Minimum temperature to prevent division by zero

        // Ensure temperature is above absolute minimum
        let safe_temperature = params.temperature_k.max(min_temperature);

        // Apply temperature correction to density
        // For most materials: ρ(T) = ρ₀ * (T₀/T) for gases, or ρ₀ * (1 - α*(T-T₀)) for solids/liquids
        let temperature_corrected_density = if let Some(thermal_expansivity) = self.thermal_expansivity {
            // For solids/liquids: use thermal expansivity
            let expansivity_f64 = thermal_expansivity as f64 / 1_000_000.0; // Convert from scaled value
            let temp_diff = safe_temperature - reference_temperature;
            let density_factor = 1.0 - expansivity_f64 * temp_diff;

            // Ensure density doesn't become negative or unreasonably high
            let bounded_density_factor = density_factor.clamp(0.1, 10.0);
            base_density * bounded_density_factor
        } else {
            // For gases or when thermal expansivity is not available: use ideal gas temperature scaling
            let density_factor = reference_temperature / safe_temperature;

            // Limit density scaling to reasonable bounds (0.1x to 100x base density)
            let bounded_density_factor = density_factor.clamp(0.1, 100.0);
            base_density * bounded_density_factor
        };

        // If bulk modulus is available, account for pressure compressibility
        if let Some(bulk_modulus) = self.bulk_modulus_pa {
            let bulk_modulus_f64 = bulk_modulus as f64;

            // For compressible materials, use bulk modulus to adjust density
            // Bulk modulus K = -V * (dP/dV) ≈ ρ * (dP/dρ)
            // For small pressure changes: ρ = ρ₀ * (1 + P/K)
            let reference_pressure = 101325.0; // Standard atmospheric pressure
            // Clamp to geological pressure range: ~0.1 atm to deep mantle (~1e15 Pa)
            let pressure_diff = (params.pressure_pa - reference_pressure).clamp(-90000.0, 1e15);

            // Apply both temperature and pressure corrections
            let density_correction = 1.0 + (pressure_diff / bulk_modulus_f64);
            let final_density = temperature_corrected_density * density_correction.max(0.1);

            final_density * volume_m3
        } else {
            // Use temperature-corrected density only
            temperature_corrected_density * volume_m3
        }
    }

    /// Calculate mass using ideal gas law (primarily for gas phases)
    ///
    /// Uses PV = nRT, where n = m/M (mass/molar_mass)
    /// Rearranged: m = (P * V * M) / (R * T)
    ///
    /// # Arguments
    /// * `params` - MassCalculationParams containing pressure (Pa), volume (km³), and temperature (K)
    /// * `molar_mass_kg_per_mol` - Molar mass in kg/mol
    ///
    /// # Returns
    /// Mass in kilograms
    ///

    pub fn calculate_mass_ideal_gas(
        params: MassCalculationParams,
        molar_mass_kg_per_mol: f64,
    ) -> f64 {
        const R: f64 = 8.314; // Universal gas constant J/(mol·K)

        // Convert volume from km³ to m³ for ideal gas law (1 km³ = 1e9 m³)
        let volume_m3 = params.volume_km3 * 1e9;

        (params.pressure_pa * volume_m3 * molar_mass_kg_per_mol) / (R * params.temperature_k)
    }

    /// Get density as f64 for calculations
    pub fn density_as_f64(&self) -> f64 {
        self.density_kg_m3 as f64
    }

    /// Get bulk modulus as f64 for calculations (if available)
    pub fn bulk_modulus_as_f64(&self) -> Option<f64> {
        self.bulk_modulus_pa.map(|b| b as f64)
    }

    /// Calculate pressure from mass, volume, and temperature using named parameters
    ///
    /// This is the inverse of calculate_mass_from_pressure_volume.
    /// For solids and liquids: Uses density with bulk modulus correction if available
    /// For gases: Uses ideal gas approximation
    ///
    /// # Arguments
    /// * `params` - PressureCalculationParams containing mass (kg), volume (km³), and temperature (K)
    ///
    /// # Returns
    /// Pressure in Pascals
  
    pub fn calculate_pressure_from_mass_volume(&self, params: PressureCalculationParams) -> f64 {
        // Convert volume from km³ to m³ for density calculations (1 km³ = 1e9 m³)
        let volume_m3 = params.volume_km3 * 1e9;

        // Calculate actual density from mass and volume
        let actual_density = params.mass_kg / volume_m3;

        let base_density = self.density_kg_m3 as f64;
        let reference_temperature = 273.15; // Standard temperature (0°C)
        let reference_pressure = 101325.0; // Standard atmospheric pressure

        // Apply temperature correction to base density to get expected density at this temperature
        let temperature_corrected_density = if let Some(thermal_expansivity) = self.thermal_expansivity {
            // For solids/liquids: use thermal expansivity
            let expansivity_f64 = thermal_expansivity as f64 / 1_000_000.0; // Convert from scaled value
            let temp_diff = params.temperature_k - reference_temperature;
            base_density * (1.0 - expansivity_f64 * temp_diff)
        } else {
            // For gases or when thermal expansivity is not available: use ideal gas temperature scaling
            base_density * (reference_temperature / params.temperature_k)
        };

        // If bulk modulus is available, solve for pressure using compressibility
        if let Some(bulk_modulus) = self.bulk_modulus_pa {
            let bulk_modulus_f64 = bulk_modulus as f64;

            // From the forward equation: ρ = ρ₀(T) * (1 + (P - P₀)/K)
            // Solving for P: P = P₀ + K * (ρ/ρ₀(T) - 1)
            let density_ratio = actual_density / temperature_corrected_density;
            let pressure_correction = bulk_modulus_f64 * (density_ratio - 1.0);

            reference_pressure + pressure_correction
        } else {
            // For incompressible materials, pressure doesn't significantly affect density
            // Use ideal gas law approximation: P = (ρ * R * T) / M
            // But since we don't have molar mass, we'll use a simplified approach
            // based on the density difference from expected

            // If actual density is higher than expected, pressure is likely higher
            let density_ratio = actual_density / temperature_corrected_density;

            // Simple approximation: assume linear relationship for small changes
            reference_pressure * density_ratio
        }
    }

    /// Calculate pressure using ideal gas law (primarily for gas phases)
    ///
    /// Uses PV = nRT, where n = m/M (mass/molar_mass)
    /// Rearranged: P = (m * R * T) / (V * M)
    ///
    /// # Arguments
    /// * `params` - PressureCalculationParams containing mass (kg), volume (km³), and temperature (K)
    /// * `molar_mass_kg_per_mol` - Molar mass in kg/mol
    ///
    /// # Returns
    /// Pressure in Pascals
    ///

    pub fn calculate_pressure_ideal_gas(
        params: PressureCalculationParams,
        molar_mass_kg_per_mol: f64,
    ) -> f64 {
        const R: f64 = 8.314; // Universal gas constant J/(mol·K)

        // Convert volume from km³ to m³ for ideal gas law (1 km³ = 1e9 m³)
        let volume_m3 = params.volume_km3 * 1e9;

        (params.mass_kg * R * params.temperature_k) / (volume_m3 * molar_mass_kg_per_mol)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub solid: Option<MaterialPhase>,
    pub liquid: Option<MaterialPhase>,
    pub gas: Option<MaterialPhase>,
    pub emission_compounds: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialPhases {
    Solid,
    Liquid,
    Gas,
}

impl MaterialPhases {
    /// Convert the enum to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MaterialPhases::Solid => "solid",
            MaterialPhases::Liquid => "liquid",
            MaterialPhases::Gas => "gas",
        }
    }

    /// Convert a string to the MaterialPhases enum
    /// Accepts case-insensitive input
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "solid" => Some(MaterialPhases::Solid),
            "liquid" => Some(MaterialPhases::Liquid),
            "gas" => Some(MaterialPhases::Gas),
            _ => None,
        }
    }

    /// Get all valid phase names as strings
    pub fn all_phase_names() -> Vec<&'static str> {
        vec!["solid", "liquid", "gas"]
    }

    /// Get all MaterialPhases enum variants
    pub fn all_phases() -> Vec<Self> {
        vec![MaterialPhases::Solid, MaterialPhases::Liquid, MaterialPhases::Gas]
    }
}