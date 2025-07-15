use crate::material::material::{MaterialPhase, MassCalculationParams, PressureCalculationParams};

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
    fn test_pressure_calculation_params_new() {
        let params = PressureCalculationParams::new(1000.0, 1.0, 273.15);
        assert_eq!(params.mass_kg, 1000.0);
        assert_eq!(params.volume_km3, 1.0);
        assert_eq!(params.temperature_k, 273.15);
    }

    #[test]
    fn test_pressure_calculation_params_at_stp() {
        let params = PressureCalculationParams::at_stp(1000.0, 2.0);
        assert_eq!(params.mass_kg, 1000.0);
        assert_eq!(params.volume_km3, 2.0);
        assert_eq!(params.temperature_k, 273.15);
    }

    #[test]
    fn test_pressure_calculation_params_at_ntp() {
        let params = PressureCalculationParams::at_ntp(1000.0, 1.5);
        assert_eq!(params.mass_kg, 1000.0);
        assert_eq!(params.volume_km3, 1.5);
        assert_eq!(params.temperature_k, 293.15);
    }

    #[test]
    fn test_calculate_pressure_from_mass_volume() {
        let phase = create_test_material_phase();
        
        // Test with standard conditions
        let params = PressureCalculationParams::at_stp(1000000000.0, 1.0); // 1 billion kg in 1 km³
        let pressure = phase.calculate_pressure_from_mass_volume(params);
        
        // Should be close to standard atmospheric pressure for this density
        assert!(pressure > 50000.0 && pressure < 200000.0, "Pressure calculation failed: got {}", pressure);
    }

    #[test]
    fn test_calculate_pressure_ideal_gas() {
        let params = PressureCalculationParams::at_stp(1.29, 1.0); // Air mass at STP
        let molar_mass_air = 0.029; // kg/mol for air
        
        let pressure = MaterialPhase::calculate_pressure_ideal_gas(params, molar_mass_air);
        
        // Should be approximately standard atmospheric pressure (101325 Pa)
        assert!((pressure - 101325.0).abs() < 5000.0, "Ideal gas pressure calculation failed: got {}", pressure);
    }

    #[test]
    fn test_inverse_relationship() {
        let phase = create_test_material_phase();
        
        // Start with known pressure, volume, temperature
        let original_pressure = 101325.0;
        let volume_km3 = 1.0;
        let temperature_k = 273.15;
        
        // Calculate mass from pressure
        let mass_params = MassCalculationParams::new(original_pressure, volume_km3, temperature_k);
        let calculated_mass = phase.calculate_mass_from_pressure_volume(mass_params);
        
        // Now calculate pressure back from mass
        let pressure_params = PressureCalculationParams::new(calculated_mass, volume_km3, temperature_k);
        let calculated_pressure = phase.calculate_pressure_from_mass_volume(pressure_params);
        
        // The calculated pressure should be close to the original pressure
        let pressure_diff = (calculated_pressure - original_pressure).abs();
        let relative_error = pressure_diff / original_pressure;
        
        assert!(relative_error < 0.1, 
            "Inverse relationship failed: original={}, calculated={}, relative_error={}", 
            original_pressure, calculated_pressure, relative_error);
    }

    #[test]
    fn test_ideal_gas_inverse_relationship() {
        let original_pressure = 101325.0;
        let volume_km3 = 1.0;
        let temperature_k = 273.15;
        let molar_mass_air = 0.029;
        
        // Calculate mass from pressure using ideal gas law
        let mass_params = MassCalculationParams::new(original_pressure, volume_km3, temperature_k);
        let calculated_mass = MaterialPhase::calculate_mass_ideal_gas(mass_params, molar_mass_air);
        
        // Now calculate pressure back from mass
        let pressure_params = PressureCalculationParams::new(calculated_mass, volume_km3, temperature_k);
        let calculated_pressure = MaterialPhase::calculate_pressure_ideal_gas(pressure_params, molar_mass_air);
        
        // Should be very close to original pressure for ideal gas
        let pressure_diff = (calculated_pressure - original_pressure).abs();
        assert!(pressure_diff < 1.0, 
            "Ideal gas inverse relationship failed: original={}, calculated={}, diff={}", 
            original_pressure, calculated_pressure, pressure_diff);
    }
}
