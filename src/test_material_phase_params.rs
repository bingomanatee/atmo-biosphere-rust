use crate::material::material::{MaterialPhase, MassCalculationParams};

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_material_phase() -> MaterialPhase {
        MaterialPhase {
            density_kg_m3: 1000.0,
            specific_heat_capacity_j_per_kg_k: 4186.0,
            thermal_conductivity_w_m_k: 1.0,
            thermal_transmission_r0_min: 1.0,
            thermal_transmission_r0_max: 5.0,
            melt_temp: 273.0,
            melt_temp_min: Some(270.0),
            melt_temp_max: Some(276.0),
            latent_heat_fusion: 334000.0,
            boil_temp: 373.0,
            latent_heat_vapor: 2260000.0,
            gas_interference_factor: Some(500.0),
            thermal_conduction_modifier: Some(1000.0),
            thermal_expansivity: Some(200.0),
            dynamic_viscosity: Some(1000.0),
            bulk_modulus_pa: Some(2200000000.0),
            activation_energy_j_per_mol: Some(50000.0),
            activation_volume_m3_per_mol: Some(18.0),
            cool_temp_min: Some(250.0),
            cool_temp_max: Some(300.0),
        }
    }

    #[test]
    fn test_mass_calculation_params_new() {
        let params = MassCalculationParams::new(101325.0, 1.0, 273.15);
        assert_eq!(params.pressure_pa, 101325.0);
        assert_eq!(params.volume_km3, 1.0);
        assert_eq!(params.temperature_k, 273.15);
    }

    #[test]
    fn test_mass_calculation_params_at_stp() {
        let params = MassCalculationParams::at_stp(2.0);
        assert_eq!(params.pressure_pa, 101325.0);
        assert_eq!(params.volume_km3, 2.0);
        assert_eq!(params.temperature_k, 273.15);
    }

    #[test]
    fn test_mass_calculation_params_at_ntp() {
        let params = MassCalculationParams::at_ntp(1.5);
        assert_eq!(params.pressure_pa, 101325.0);
        assert_eq!(params.volume_km3, 1.5);
        assert_eq!(params.temperature_k, 293.15);
    }

    #[test]
    fn test_calculate_mass_from_pressure_volume_with_params() {
        let phase = create_test_material_phase();
        let params = MassCalculationParams::at_stp(1.0);
        
        let mass = phase.calculate_mass_from_pressure_volume(params);
        
        // Should be close to density * volume for standard conditions
        assert!(mass > 900.0 && mass < 1100.0, "Mass calculation failed: got {}", mass);
    }

    #[test]
    fn test_calculate_mass_ideal_gas_with_params() {
        let params = MassCalculationParams::at_stp(1.0);
        let molar_mass_air = 0.029; // kg/mol for air
        
        let mass = MaterialPhase::calculate_mass_ideal_gas(params, molar_mass_air);
        
        // Should be approximately 1.29 kg for air at STP
        assert!((mass - 1.29).abs() < 0.1, "Ideal gas calculation failed: got {}", mass);
    }

    #[test]
    fn test_temperature_effects() {
        let phase = create_test_material_phase();
        
        // Test at different temperatures
        let cold_params = MassCalculationParams::new(101325.0, 1.0, 250.0); // Cold
        let hot_params = MassCalculationParams::new(101325.0, 1.0, 350.0);  // Hot
        
        let cold_mass = phase.calculate_mass_from_pressure_volume(cold_params);
        let hot_mass = phase.calculate_mass_from_pressure_volume(hot_params);
        
        // For materials with thermal expansivity, density should decrease with temperature
        // So mass should be higher at lower temperature
        assert!(cold_mass > hot_mass, "Temperature effect failed: cold={}, hot={}", cold_mass, hot_mass);
    }
}
