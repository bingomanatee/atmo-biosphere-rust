use crate::material::material::{Material, MaterialPhase, MaterialPhases};
use crate::utils::json_parser::JsonParser;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Global cache for loaded materials
static MATERIALS_CACHE: OnceLock<Mutex<HashMap<String, Material>>> = OnceLock::new();

/// Materials loader that provides access to material properties from JSON data
pub struct MaterialsLoader;

impl MaterialsLoader {
    /// Load all materials from the materials.json file
    pub fn load_materials() -> Result<HashMap<String, Material>, String> {
        // Check if materials are already cached
        let cache = MATERIALS_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        {
            let cached_materials = cache.lock().unwrap();
            if !cached_materials.is_empty() {
                return Ok(cached_materials.clone());
            }
        }

        // Load JSON data
        let json = JsonParser::load_json("src/material/materials.json")?;
        
        // Parse materials from JSON
        let materials = Self::parse_materials_from_json(&json)?;
        
        // Cache the materials
        {
            let mut cached_materials = cache.lock().unwrap();
            *cached_materials = materials.clone();
        }
        
        Ok(materials)
    }

    /// Parse materials from JSON Value
    fn parse_materials_from_json(json: &Value) -> Result<HashMap<String, Material>, String> {
        let mut materials = HashMap::new();
        
        let materials_obj = json.as_object()
            .ok_or("Materials JSON root must be an object")?;
        
        for (material_name, material_data) in materials_obj {
            let material = Self::parse_material(material_data)?;
            materials.insert(material_name.clone(), material);
        }
        
        Ok(materials)
    }

    /// Parse a single material from JSON
    fn parse_material(material_data: &Value) -> Result<Material, String> {
        let material_obj = material_data.as_object()
            .ok_or("Material data must be an object")?;
        
        let solid = if let Some(solid_data) = material_obj.get("solid") {
            Some(Self::parse_material_phase(solid_data)?)
        } else {
            None
        };
        
        let liquid = if let Some(liquid_data) = material_obj.get("liquid") {
            Some(Self::parse_material_phase(liquid_data)?)
        } else {
            None
        };
        
        let gas = if let Some(gas_data) = material_obj.get("gas") {
            Some(Self::parse_material_phase(gas_data)?)
        } else {
            None
        };
        
        let emission_compounds = if let Some(compounds_data) = material_obj.get("emission_compounds") {
            Some(Self::parse_emission_compounds(compounds_data)?)
        } else {
            None
        };
        
        Ok(Material {
            solid,
            liquid,
            gas,
            emission_compounds,
        })
    }

    /// Parse a material phase from JSON
    fn parse_material_phase(phase_data: &Value) -> Result<MaterialPhase, String> {
        let phase_obj = phase_data.as_object()
            .ok_or("Material phase data must be an object")?;

        // Helper function to get required f64 value and convert to f32
        let get_required_f32 = |key: &str| -> Result<f32, String> {
            let value = phase_obj.get(key)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("Missing or invalid required field: {}", key))?;

            // Ensure it fits in f32
            if value > f32::MAX as f64 {
                return Err(format!("Value too large for f32 in {}: {}", key, value));
            }
            Ok(value as f32)
        };

        // Helper function to get optional f32 value
        let get_optional_f32 = |key: &str| -> Option<f32> {
            phase_obj.get(key)
                .and_then(|v| v.as_f64())
                .map(|value| value as f32)
        };

        // Helper function to get required u32 value (for integer fields)
        let get_required_u32 = |key: &str| -> Result<f32, String> {
            let value = phase_obj.get(key)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("Missing or invalid required field: {}", key))?;

            // Handle fractional values by rounding and ensure it fits in f32
            if value < 0.0 {
                return Err(format!("Negative value not allowed for {}: {}", key, value));
            }
            if value > f32::MAX as f64 {
                return Err(format!("Value too large for f32 in {}: {}", key, value));
            }
            Ok(value.round() as f32)
        };

        // Helper function to get optional f64 value and convert to f32
        let get_optional_u32 = |key: &str| -> Option<f32> {
            phase_obj.get(key)
                .and_then(|v| v.as_f64())
                .and_then(|value| {
                    if value >= 0.0 && value <= f32::MAX as f64 {
                        Some(value.round() as f32)
                    } else {
                        None
                    }
                })
        };

        // Helper function to get required f64 value (for very large values)
        let get_required_u64 = |key: &str| -> Result<f64, String> {
            phase_obj.get(key)
                .and_then(|v| v.as_f64())
                .and_then(|value| {
                    if value >= 0.0 && value <= f64::MAX as f64 {
                        Some(value.round() as f64)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| format!("Missing or invalid required field: {}", key))
        };

        // Special handling for fractional values that should be scaled
        let get_fractional_as_u32 = |key: &str, scale: f64| -> Option<f32> {
            phase_obj.get(key)
                .and_then(|v| v.as_f64())
                .and_then(|value| {
                    let scaled = value * scale;
                    if scaled >= 0.0 && scaled <= f32::MAX as f64 {
                        Some(scaled.round() as f32)
                    } else {
                        None
                    }
                })
        };

        Ok(MaterialPhase {
            density_kg_m3: get_required_u32("density_kg_m3")?,
            specific_heat_capacity_j_per_kg_k: get_required_u32("specific_heat_capacity_j_per_kg_k")?,
            thermal_conductivity_w_m_k: get_required_u32("thermal_conductivity_w_m_k")?,
            thermal_transmission_r0_min: get_required_u32("thermal_transmission_r0_min")?,
            thermal_transmission_r0_max: get_required_u32("thermal_transmission_r0_max")?,
            melt_temp: get_optional_f32("melt_temp").unwrap_or_else(|| {
                // If melt_temp is not provided, calculate from min/max if available
                if let (Some(min), Some(max)) = (get_optional_u32("melt_temp_min"), get_optional_u32("melt_temp_max")) {
                    (min + max) / 2.0
                } else {
                    273.15 // Default to water freezing point
                }
            }),
            melt_temp_min: get_optional_u32("melt_temp_min"),
            melt_temp_max: get_optional_u32("melt_temp_max"),
            latent_heat_fusion: get_required_f32("latent_heat_fusion")?,
            boil_temp: get_required_f32("boil_temp")?,
            latent_heat_vapor: get_required_f32("latent_heat_vapor")?,
            // Gas interference factor is fractional (0.0-1.0), scale by 1000 to preserve precision
            gas_interference_factor: get_fractional_as_u32("gas_interference_factor", 1000.0),
            // Thermal conduction modifier is fractional, scale by 1000
            thermal_conduction_modifier_dimensionless: get_fractional_as_u32("thermal_conduction_modifier_dimensionless", 1000.0).expect("thermal_conduction_modifier_dimensionless is required"),
            // Thermal expansivity is very small (e.g., 1e-05), scale by 1e9 to preserve precision
            thermal_expansivity_per_k: get_fractional_as_u32("thermal_expansivity_per_k", 1e9).expect("thermal_expansivity_per_k is required"),
            dynamic_viscosity_pa_s: get_required_u64("dynamic_viscosity_pa_s")?,
            bulk_modulus_pa: get_required_u64("bulk_modulus_pa")?,
            activation_energy_j_per_mol: get_optional_u32("activation_energy_j_per_mol"),
            activation_volume_m3_per_mol: get_fractional_as_u32("activation_volume_m3_per_mol", 1e9),
            cool_temp_min: get_optional_u32("cool_temp_min"),
            cool_temp_max: get_optional_u32("cool_temp_max"),
        })
    }

    /// Parse emission compounds from JSON
    fn parse_emission_compounds(compounds_data: &Value) -> Result<HashMap<String, f64>, String> {
        let compounds_obj = compounds_data.as_object()
            .ok_or("Emission compounds data must be an object")?;
        
        let mut compounds = HashMap::new();
        for (compound_name, value) in compounds_obj {
            let concentration = value.as_f64()
                .ok_or_else(|| format!("Invalid concentration value for compound: {}", compound_name))?;
            compounds.insert(compound_name.clone(), concentration);
        }
        
        Ok(compounds)
    }

    /// Get phase properties for a specific material and phase
    /// Returns the MaterialPhase if found, or an error if the material or phase doesn't exist
    pub fn get_phase_properties(material_name: &str, phase: MaterialPhases) -> Result<MaterialPhase, String> {
        let materials = Self::load_materials()?;
        
        let material = materials.get(material_name)
            .ok_or_else(|| format!("Material '{}' not found", material_name))?;
        
        let phase_properties = match phase {
            MaterialPhases::Solid => material.solid.as_ref(),
            MaterialPhases::Liquid => material.liquid.as_ref(),
            MaterialPhases::Gas => material.gas.as_ref(),
        };
        
        phase_properties
            .ok_or_else(|| format!("Phase '{}' not found for material '{}'", phase.as_str(), material_name))
            .map(|p| p.clone())
    }

    /// Get all available material names
    pub fn get_material_names() -> Result<Vec<String>, String> {
        let materials = Self::load_materials()?;
        Ok(materials.keys().cloned().collect())
    }

    /// Get all available phases for a specific material as enum variants
    pub fn get_available_phases(material_name: &str) -> Result<Vec<MaterialPhases>, String> {
        let materials = Self::load_materials()?;

        let material = materials.get(material_name)
            .ok_or_else(|| format!("Material '{}' not found", material_name))?;

        let mut phases = Vec::new();
        if material.solid.is_some() {
            phases.push(MaterialPhases::Solid);
        }
        if material.liquid.is_some() {
            phases.push(MaterialPhases::Liquid);
        }
        if material.gas.is_some() {
            phases.push(MaterialPhases::Gas);
        }

        Ok(phases)
    }

    /// Get all available phases for a specific material as strings
    pub fn get_available_phase_names(material_name: &str) -> Result<Vec<String>, String> {
        let phases = Self::get_available_phases(material_name)?;
        Ok(phases.iter().map(|p| p.as_str().to_string()).collect())
    }

    /// Get emission compounds for a material
    pub fn get_emission_compounds(material_name: &str) -> Result<Option<HashMap<String, f64>>, String> {
        let materials = Self::load_materials()?;
        
        let material = materials.get(material_name)
            .ok_or_else(|| format!("Material '{}' not found", material_name))?;
        
        Ok(material.emission_compounds.clone())
    }

    /// Clear the materials cache (useful for testing or reloading)
    pub fn clear_cache() {
        if let Some(cache) = MATERIALS_CACHE.get() {
            let mut cached_materials = cache.lock().unwrap();
            cached_materials.clear();
        }
    }
}

/// Convenience function to get phase properties by name and phase string
/// Converts string phase names ("solid", "liquid", "gas") to MaterialPhases enum
pub fn get_phase_properties_by_name(material_name: &str, phase_name: &str) -> Result<MaterialPhase, String> {
    // Convert string to MaterialPhases enum
    let phase = match phase_name.to_lowercase().as_str() {
        "solid" => MaterialPhases::Solid,
        "liquid" => MaterialPhases::Liquid,
        "gas" => MaterialPhases::Gas,
        _ => return Err(format!("Invalid phase name: '{}'. Valid phases are: solid, liquid, gas", phase_name)),
    };

    MaterialsLoader::get_phase_properties(material_name, phase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_materials() {
        let result = MaterialsLoader::load_materials();
        assert!(result.is_ok(), "Failed to load materials: {:?}", result.err());
        
        let materials = result.unwrap();
        assert!(!materials.is_empty(), "No materials loaded");
        
        // Check that some expected materials exist
        assert!(materials.contains_key("basalt"), "Basalt material not found");
        assert!(materials.contains_key("granite"), "Granite material not found");
        assert!(materials.contains_key("water"), "Water material not found");
    }

    #[test]
    fn test_get_phase_properties() {
        let result = MaterialsLoader::get_phase_properties("basalt", MaterialPhases::Solid);
        assert!(result.is_ok(), "Failed to get basalt solid properties: {:?}", result.err());

        let phase = result.unwrap();
        assert!(phase.density_kg_m3 > 0.0, "Invalid density value");
    }

    #[test]
    fn test_get_phase_properties_by_name() {
        let result = get_phase_properties_by_name("water", "liquid");
        assert!(result.is_ok(), "Failed to get water liquid properties: {:?}", result.err());

        let phase = result.unwrap();
        assert_eq!(phase.density_kg_m3, 1000.0, "Water density should be 1000 kg/m³");
    }

    #[test]
    fn test_invalid_material() {
        let result = MaterialsLoader::get_phase_properties("nonexistent", MaterialPhases::Solid);
        assert!(result.is_err(), "Should fail for nonexistent material");
    }

    #[test]
    fn test_invalid_phase() {
        let result = get_phase_properties_by_name("basalt", "plasma");
        assert!(result.is_err(), "Should fail for invalid phase");
    }
}
