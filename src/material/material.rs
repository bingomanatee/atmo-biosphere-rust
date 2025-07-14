
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPhase {
    pub density_kg_m3: u32,
    pub specific_heat_capacity_j_per_kg_k: u32,
    pub thermal_conductivity_w_m_k: u32,
    pub thermal_transmission_r0_min: u32,
    pub thermal_transmission_r0_max: u32,
    pub melt_temp: Option<u32>,
    pub melt_temp_min: Option<u32>,
    pub melt_temp_max: Option<u32>,
    pub latent_heat_fusion: Option<u32>,
    pub boil_temp: Option<u32>,
    pub latent_heat_vapor: Option<u32>,
    pub gas_interference_factor: Option<u32>,
    pub thermal_conduction_modifier: Option<u32>,
    // Additional fields found in JSON
    pub thermal_expansivity: Option<u32>,
    pub dynamic_viscosity: Option<u64>, // This can be very large (e.g., 1e+25)
    pub bulk_modulus_pa: Option<u64>,   // This is very large (e.g., 130000000000)
    pub activation_energy_j_per_mol: Option<u32>,
    pub activation_volume_m3_per_mol: Option<u32>,
    pub cool_temp_min: Option<u32>,
    pub cool_temp_max: Option<u32>,
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