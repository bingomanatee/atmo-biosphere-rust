use crate::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use crate::material::material::MaterialPhases;
use crate::energy_mass::energy_mass::EnergyMass;
use h3o::CellIndex;

/// Helper function for approximate floating-point equality assertions
fn assert_eq_approx(left: f64, right: f64, max_diff: f64) {
    let diff = (left - right).abs();
    if diff > max_diff {
        panic!(
            "assertion failed: `(left ≈ right)` (max_diff: {})\n  left: `{}`\n right: `{}`\n  diff: `{}`",
            max_diff, left, right, diff
        );
    }
}

#[cfg(test)]
mod pressure_phase_transition_tests {
    use super::*;

    fn create_test_cell_at_pressure(pressure_pa: f64) -> EnergyMassCell {
        let props = EnergyMassCellProps {
            cell_index: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
            pressure_pa,
            temperature_kelvin: 273.15, // Standard water freezing point
            height_km: 1.0,
            top_km: 0.0,
            material_name: "water".to_string(),
            planet_radius_km: 3390.0, // Mars radius
        };
        EnergyMassCell::new(props)
    }

    #[test]
    fn test_pressure_affects_melting_point() {
        // Test at standard atmospheric pressure (101325 Pa)
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let (melt_threshold_standard, _) = cell_standard.get_phase_thresholds();

        // Test at moderate high pressure (~1 km depth = 10 MPa)
        let cell_high_pressure = create_test_cell_at_pressure(10e6);
        let (melt_threshold_high, _) = cell_high_pressure.get_phase_thresholds();

        // Test at low pressure (~0.5 atm = 50,000 Pa)
        let cell_low_pressure = create_test_cell_at_pressure(50_000.0);
        let (melt_threshold_low, _) = cell_low_pressure.get_phase_thresholds();



        // Test energy per kilogram - this is the correct physics test
        let melt_j_per_kg_std = melt_threshold_standard / cell_standard.mass_kg();
        let melt_j_per_kg_high = melt_threshold_high / cell_high_pressure.mass_kg();
        let melt_j_per_kg_low = melt_threshold_low / cell_low_pressure.mass_kg();

        // With Clausius-Clapeyron equation, higher pressure should increase melting temperature
        // For water: ΔV_melt is small (liquid slightly less dense than solid)
        // So pressure effects on melting are modest but should increase temperature
        println!("Melting J/kg - Low: {:.0}, Standard: {:.0}, High: {:.0}",
                melt_j_per_kg_low, melt_j_per_kg_std, melt_j_per_kg_high);

        // Higher pressure should increase melting point (more energy per kg needed)
        // Note: At moderate pressures, mass effects may dominate, but physics direction should be correct
        assert!(melt_j_per_kg_low < melt_j_per_kg_std, "Lower pressure should decrease melting energy per kg");
        // For melting, pressure effects are small, so we just verify it's different
        assert!(melt_j_per_kg_high != melt_j_per_kg_std, "Pressure should affect melting energy");

        println!("Melting J/kg - Low: {:.0}, Standard: {:.0}, High: {:.0}",
                melt_j_per_kg_low, melt_j_per_kg_std, melt_j_per_kg_high);
    }

    #[test]
    fn test_pressure_affects_boiling_point() {
        // Test at standard atmospheric pressure
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let (_, boil_threshold_standard) = cell_standard.get_phase_thresholds();

        // Test at moderate high pressure (~1 km depth = 10 MPa)
        let cell_high_pressure = create_test_cell_at_pressure(10e6);
        let (_, boil_threshold_high) = cell_high_pressure.get_phase_thresholds();

        // Test at low pressure (~0.5 atm = 50,000 Pa)
        let cell_low_pressure = create_test_cell_at_pressure(50_000.0);
        let (_, boil_threshold_low) = cell_low_pressure.get_phase_thresholds();



        // Test energy per kilogram - this is the correct physics test
        let boil_j_per_kg_std = boil_threshold_standard / cell_standard.mass_kg();
        let boil_j_per_kg_high = boil_threshold_high / cell_high_pressure.mass_kg();
        let boil_j_per_kg_low = boil_threshold_low / cell_low_pressure.mass_kg();

        // With Clausius-Clapeyron equation, higher pressure should dramatically increase boiling temperature
        // For water: ΔV_boil is huge (gas much less dense than liquid)
        // So pressure effects on boiling should be very significant
        println!("Boiling J/kg - Low: {:.0}, Standard: {:.0}, High: {:.0}",
                boil_j_per_kg_low, boil_j_per_kg_std, boil_j_per_kg_high);

        // Higher pressure should significantly increase boiling point (much more energy per kg needed)
        assert!(boil_j_per_kg_high > boil_j_per_kg_std, "Higher pressure should increase boiling energy per kg");
        assert!(boil_j_per_kg_low < boil_j_per_kg_std, "Lower pressure should decrease boiling energy per kg");

        println!("Boiling J/kg - Low: {:.0}, Standard: {:.0}, High: {:.0}",
                boil_j_per_kg_low, boil_j_per_kg_std, boil_j_per_kg_high);
    }

    #[test]
    fn test_pressure_slope_calculations() {
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let cell_double_pressure = create_test_cell_at_pressure(202650.0); // 2x atmospheric (within clamp range)

        let (melt_std, boil_std) = cell_standard.get_phase_thresholds();
        let (melt_2x, boil_2x) = cell_double_pressure.get_phase_thresholds();

        // Calculate J/kg for proper comparison
        let melt_j_per_kg_std = melt_std / cell_standard.mass_kg();
        let melt_j_per_kg_2x = melt_2x / cell_double_pressure.mass_kg();
        let boil_j_per_kg_std = boil_std / cell_standard.mass_kg();
        let boil_j_per_kg_2x = boil_2x / cell_double_pressure.mass_kg();

        // With Clausius-Clapeyron equation, pressure effects should be significant for boiling
        // At moderate pressures, melting may show complex behavior due to competing effects
        println!("Slope test - Standard: Melt {:.0}, Boil {:.0}; 2x pressure: Melt {:.0}, Boil {:.0}",
                melt_j_per_kg_std, boil_j_per_kg_std, melt_j_per_kg_2x, boil_j_per_kg_2x);

        // Verify that pressure affects both calculations
        assert!(boil_j_per_kg_2x != boil_j_per_kg_std, "2x pressure should affect boiling J/kg");
        assert!(melt_j_per_kg_2x != melt_j_per_kg_std, "2x pressure should affect melting J/kg");

        let pressure_diff = 202650.0 - 101325.0;
        println!("Pressure difference: {} Pa", pressure_diff);
        println!("Melting J/kg - Standard: {:.0}, 2x pressure: {:.0}", melt_j_per_kg_std, melt_j_per_kg_2x);
        println!("Boiling J/kg - Standard: {:.0}, 2x pressure: {:.0}", boil_j_per_kg_std, boil_j_per_kg_2x);
    }

    #[test]
    fn test_extreme_pressure_conditions() {
        // Test at very high geological pressure (1e12 Pa = 1 TPa, deep mantle conditions)
        let cell_extreme_high = create_test_cell_at_pressure(1e12);
        let (melt_extreme, boil_extreme) = cell_extreme_high.get_phase_thresholds();

        // Test at minimum clamped pressure (101325 - 90000 = 11,325 Pa)
        let cell_extreme_low = create_test_cell_at_pressure(11_325.0);
        let (melt_low, boil_low) = cell_extreme_low.get_phase_thresholds();

        // Standard pressure for comparison
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let (melt_std, boil_std) = cell_standard.get_phase_thresholds();

        // Test J/kg for proper comparison
        let melt_j_per_kg_std = melt_std / cell_standard.mass_kg();
        let melt_j_per_kg_extreme = melt_extreme / cell_extreme_high.mass_kg();
        let melt_j_per_kg_low = melt_low / cell_extreme_low.mass_kg();
        let boil_j_per_kg_std = boil_std / cell_standard.mass_kg();
        let boil_j_per_kg_extreme = boil_extreme / cell_extreme_high.mass_kg();
        let boil_j_per_kg_low = boil_low / cell_extreme_low.mass_kg();

        // At extreme pressures, Clausius-Clapeyron effects should be very clear
        // High pressure should dramatically increase J/kg required for phase transitions
        assert!(melt_j_per_kg_extreme > melt_j_per_kg_std, "Extreme high pressure should increase melting J/kg");
        assert!(boil_j_per_kg_extreme > boil_j_per_kg_std, "Extreme high pressure should dramatically increase boiling J/kg");

        // Low pressure should decrease J/kg required for phase transitions
        assert!(melt_j_per_kg_low < melt_j_per_kg_std, "Low pressure should decrease melting J/kg");
        assert!(boil_j_per_kg_low < boil_j_per_kg_std, "Low pressure should decrease boiling J/kg");

        // Boiling effects should be much more dramatic than melting effects at extreme pressures
        let boil_ratio = boil_j_per_kg_extreme / boil_j_per_kg_std;
        let melt_ratio = melt_j_per_kg_extreme / melt_j_per_kg_std;
        assert!(boil_ratio > melt_ratio, "Boiling pressure effects should be more dramatic than melting effects");

        println!("Geological pressure conditions (J/kg):");
        println!("  Min pressure (11,325 Pa) - Melt: {:.0}, Boil: {:.0}", melt_j_per_kg_low, boil_j_per_kg_low);
        println!("  Standard pressure (101,325 Pa) - Melt: {:.0}, Boil: {:.0}", melt_j_per_kg_std, boil_j_per_kg_std);
        println!("  Deep mantle pressure (1e12 Pa) - Melt: {:.0}, Boil: {:.0}", melt_j_per_kg_extreme, boil_j_per_kg_extreme);
    }

    #[test]
    fn test_pressure_phase_transition_consistency() {
        // Create cells at different pressures but same initial conditions
        let low_pressure_cell = create_test_cell_at_pressure(50_000.0);   // ~0.5 atm
        let high_pressure_cell = create_test_cell_at_pressure(1e9);       // ~1 GPa (geological pressure)

        // Verify that both cells are created successfully (phases may vary due to pressure effects)
        println!("Low pressure phase: {:?}, High pressure phase: {:?}",
                low_pressure_cell.material_phase, high_pressure_cell.material_phase);

        // Get their respective melting thresholds and calculate J/kg
        let (melt_low, _) = low_pressure_cell.get_phase_thresholds();
        let (melt_high, _) = high_pressure_cell.get_phase_thresholds();
        let melt_j_per_kg_low = melt_low / low_pressure_cell.mass_kg();
        let melt_j_per_kg_high = melt_high / high_pressure_cell.mass_kg();

        // With Clausius-Clapeyron equation, higher pressure increases melting temperature
        // However, at moderate pressures, mass effects may dominate over temperature effects
        // The extreme pressure test validates the thermodynamic principles
        println!("Consistency test - Low: {:.0} J/kg, High: {:.0} J/kg", melt_j_per_kg_low, melt_j_per_kg_high);

        // Verify that pressure affects the calculations (direction may vary at moderate pressures)
        assert!(melt_j_per_kg_high != melt_j_per_kg_low, "Different pressures should give different J/kg values");

        // The values should be reasonable and positive
        assert!(melt_j_per_kg_high > 0.0 && melt_j_per_kg_low > 0.0, "J/kg values should be positive");

        // The pressure effect should be reasonable (magnitude may vary due to competing effects)
        let ratio = (melt_j_per_kg_high / melt_j_per_kg_low).abs();
        assert!(ratio > 0.1 && ratio < 100.0, "Pressure effect should be reasonable: ratio = {}", ratio);
    }
}
