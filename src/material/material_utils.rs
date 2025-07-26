use crate::material::material::MaterialPhase;

/// Utility functions for working with MaterialPhase properties
/// These functions help convert scaled integer values back to their original floating-point representations
pub struct MaterialUtils;

impl MaterialUtils {
    /// Convert gas interference factor from scaled f32 back to original f64 (0.0-1.0 range)
    pub fn gas_interference_factor_as_f64(phase: &MaterialPhase) -> f64 {
        phase.gas_interference_factor as f64 / 1000.0
    }

    /// Convert thermal conduction modifier from scaled f32 back to original f64
    pub fn thermal_conduction_modifier_dimensionless_as_f64(phase: &MaterialPhase) -> f64 {
        phase.thermal_conduction_modifier_dimensionless as f64 / 1000.0
    }

    /// Convert thermal expansivity from scaled f32 back to original f64 (very small values like 1e-05)
    pub fn thermal_expansivity_per_k_as_f64(phase: &MaterialPhase) -> f64 {
        phase.thermal_expansivity_per_k as f64 / 1e9
    }

    /// Convert activation volume from scaled f32 back to original f64 (very small values like 1e-05)
    pub fn activation_volume_m3_per_mol_as_f64(phase: &MaterialPhase) -> f64 {
        phase.activation_volume_m3_per_mol as f64 / 1e9
    }

    /// Convert dynamic viscosity from f64 to f64 (can be very large values like 1e+25)
    pub fn dynamic_viscosity_pa_s_as_f64(phase: &MaterialPhase) -> f64 {
        phase.dynamic_viscosity_pa_s
    }

    /// Convert bulk modulus from f64 to f64 (large values like 130000000000)
    pub fn bulk_modulus_pa_as_f64(phase: &MaterialPhase) -> f64 {
        phase.bulk_modulus_pa
    }

    /// Get density as f64 for calculations
    pub fn density_kg_m3_as_f64(phase: &MaterialPhase) -> f64 {
        phase.density_kg_m3 as f64
    }

    /// Get specific heat capacity as f64 for calculations
    pub fn specific_heat_capacity_j_per_kg_k_as_f64(phase: &MaterialPhase) -> f64 {
        phase.specific_heat_capacity_j_per_kg_k as f64
    }

    /// Get thermal conductivity as f64 for calculations
    pub fn thermal_conductivity_w_m_k_as_f64(phase: &MaterialPhase) -> f64 {
        phase.thermal_conductivity_w_m_k as f64
    }

    /// Get thermal transmission range as f64 tuple for calculations
    pub fn thermal_transmission_range_as_f64(phase: &MaterialPhase) -> (f64, f64) {
        (
            phase.thermal_transmission_r0_min as f64,
            phase.thermal_transmission_r0_max as f64,
        )
    }

    /// Get melting temperature as f64 for calculations
    pub fn melt_temp_as_f64(phase: &MaterialPhase) -> f64 {
        phase.melt_temp as f64
    }

    /// Get melting temperature range as f64 tuple for calculations
    pub fn melt_temp_range_as_f64(phase: &MaterialPhase) -> (f64, f64) {
        (phase.melt_temp_min as f64, phase.melt_temp_max as f64)
    }

    /// Get boiling temperature as f64 for calculations
    pub fn boil_temp_as_f64(phase: &MaterialPhase) -> f64 {
        phase.boil_temp as f64
    }

    /// Get latent heat of fusion as f64 for calculations
    pub fn latent_heat_fusion_as_f64(phase: &MaterialPhase) -> f64 {
        phase.latent_heat_fusion as f64
    }

    /// Get latent heat of vaporization as f64 for calculations
    pub fn latent_heat_vapor_as_f64(phase: &MaterialPhase) -> f64 {
        phase.latent_heat_vapor as f64
    }

    /// Get activation energy as f64 for calculations
    pub fn activation_energy_j_per_mol_as_f64(phase: &MaterialPhase) -> f64 {
        phase.activation_energy_j_per_mol as f64
    }

    /// Get emissivity as f64 for calculations
    pub fn emissivity_as_f64(phase: &MaterialPhase) -> f64 {
        phase.emissivity as f64 / 1000.0
    }

    /// Get absorptivity as f64 for calculations
    pub fn absorptivity_as_f64(phase: &MaterialPhase) -> f64 {
        phase.absorptivity as f64 / 1000.0
    }

    /// Get reflectivity as f64 for calculations
    pub fn reflectivity_as_f64(phase: &MaterialPhase) -> f64 {
        phase.reflectivity as f64 / 1000.0
    }

    /// Get cooling temperature range as f64 tuple for calculations
    // cool_temp fields removed - use boil_temp as maximum temperature limit

    /// Print all properties of a material phase in a human-readable format
    pub fn print_phase_properties(phase: &MaterialPhase, phase_name: &str) {
        println!("=== {} Phase Properties ===", phase_name);
        println!("Density: {} kg/m³", phase.density_kg_m3);
        println!("Specific heat capacity: {} J/(kg·K)", phase.specific_heat_capacity_j_per_kg_k);
        println!("Thermal conductivity: {} W/(m·K)", phase.thermal_conductivity_w_m_k);
        println!("Thermal transmission range: {} - {} (units)", 
            phase.thermal_transmission_r0_min, phase.thermal_transmission_r0_max);

        println!("Melting temperature: {} K", phase.melt_temp);
        let (min, max) = Self::melt_temp_range_as_f64(phase);
        println!("Melting temperature range: {:.2} - {:.2} K", min, max);
        println!("Boiling temperature: {} K", phase.boil_temp);
        println!("Latent heat of fusion: {} J/kg", phase.latent_heat_fusion);
        println!("Latent heat of vaporization: {} J/kg", phase.latent_heat_vapor);
        let factor = Self::gas_interference_factor_as_f64(phase);
        println!("Gas interference factor: {:.3}", factor);
        let modifier = Self::thermal_conduction_modifier_dimensionless_as_f64(phase);
        println!("Thermal conduction modifier (dimensionless): {:.3}", modifier);
        let expansivity = Self::thermal_expansivity_per_k_as_f64(phase);
        println!("Thermal expansivity: {:.2e} K⁻¹", expansivity);
        let viscosity = Self::dynamic_viscosity_pa_s_as_f64(phase);
        println!("Dynamic viscosity: {:.2e} Pa·s", viscosity);
        let modulus = Self::bulk_modulus_pa_as_f64(phase);
        println!("Bulk modulus: {:.2e} Pa", modulus);
        let energy = Self::activation_energy_j_per_mol_as_f64(phase);
        println!("Activation energy: {:.0} J/mol", energy);
        let volume = Self::activation_volume_m3_per_mol_as_f64(phase);
        println!("Activation volume: {:.2e} m³/mol", volume);
        // cool_temp fields removed - using boil_temp as maximum
        println!("Maximum temperature: {:.2} K (10x boil temp)", phase.boil_temp * 10.0);
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{MaterialsLoader, MaterialPhases};

    #[test]
    fn test_conversion_functions() {
        let phase = MaterialsLoader::get_phase_properties("basalt", MaterialPhases::Solid)
            .expect("Failed to load basalt solid phase");

        // Test that conversion functions work
        let factor = MaterialUtils::gas_interference_factor_as_f64(&phase);
        assert!(factor >= 0.0 && factor <= 1.0, "Gas interference factor should be between 0 and 1");

        let expansivity = MaterialUtils::thermal_expansivity_per_k_as_f64(&phase);
        assert!(expansivity > 0.0, "Thermal expansivity should be positive");

        // Test basic conversions
        let density_f64 = MaterialUtils::density_kg_m3_as_f64(&phase);
        assert_eq!(density_f64, phase.density_kg_m3 as f64);
    }
}
