use crate::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use crate::material::material::MaterialPhases;
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
            material_phase: MaterialPhases::Solid,
            planet_radius_km: 3390.0, // Mars radius
        };
        EnergyMassCell::new(props)
    }

    #[test]
    fn test_pressure_affects_melting_point() {
        // Test at standard atmospheric pressure (101325 Pa)
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let (melt_threshold_standard, _) = cell_standard.get_phase_thresholds();

        // Test at high pressure (10x atmospheric = 1,013,250 Pa)
        let cell_high_pressure = create_test_cell_at_pressure(1_013_250.0);
        let (melt_threshold_high, _) = cell_high_pressure.get_phase_thresholds();

        // Test at low pressure (0.1x atmospheric = 10,132.5 Pa)
        let cell_low_pressure = create_test_cell_at_pressure(10_132.5);
        let (melt_threshold_low, _) = cell_low_pressure.get_phase_thresholds();

        // Higher pressure should increase melting point (and thus energy threshold)
        assert!(melt_threshold_high > melt_threshold_standard, 
                "High pressure should increase melting threshold: {} vs {}", 
                melt_threshold_high, melt_threshold_standard);

        // Lower pressure should decrease melting point (and thus energy threshold)
        assert!(melt_threshold_low < melt_threshold_standard,
                "Low pressure should decrease melting threshold: {} vs {}",
                melt_threshold_low, melt_threshold_standard);

        println!("Melting thresholds - Low: {:.0}, Standard: {:.0}, High: {:.0}", 
                melt_threshold_low, melt_threshold_standard, melt_threshold_high);
    }

    #[test]
    fn test_pressure_affects_boiling_point() {
        // Test at standard atmospheric pressure
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let (_, boil_threshold_standard) = cell_standard.get_phase_thresholds();

        // Test at high pressure (10x atmospheric)
        let cell_high_pressure = create_test_cell_at_pressure(1_013_250.0);
        let (_, boil_threshold_high) = cell_high_pressure.get_phase_thresholds();

        // Test at low pressure (0.1x atmospheric)
        let cell_low_pressure = create_test_cell_at_pressure(10_132.5);
        let (_, boil_threshold_low) = cell_low_pressure.get_phase_thresholds();

        // Higher pressure should increase boiling point (and thus energy threshold)
        assert!(boil_threshold_high > boil_threshold_standard,
                "High pressure should increase boiling threshold: {} vs {}",
                boil_threshold_high, boil_threshold_standard);

        // Lower pressure should decrease boiling point (and thus energy threshold)
        assert!(boil_threshold_low < boil_threshold_standard,
                "Low pressure should decrease boiling threshold: {} vs {}",
                boil_threshold_low, boil_threshold_standard);

        println!("Boiling thresholds - Low: {:.0}, Standard: {:.0}, High: {:.0}",
                boil_threshold_low, boil_threshold_standard, boil_threshold_high);
    }

    #[test]
    fn test_pressure_slope_calculations() {
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let cell_double_pressure = create_test_cell_at_pressure(202650.0); // 2x atmospheric

        let (melt_std, boil_std) = cell_standard.get_phase_thresholds();
        let (melt_2x, boil_2x) = cell_double_pressure.get_phase_thresholds();

        // Calculate the actual pressure effect
        let pressure_diff = 202650.0 - 101325.0; // 101325 Pa difference
        let melt_energy_diff = melt_2x - melt_std;
        let boil_energy_diff = boil_2x - boil_std;

        // The energy difference should be proportional to pressure difference
        // and should be positive (higher pressure = higher energy threshold)
        assert!(melt_energy_diff > 0.0, "Melting energy should increase with pressure");
        assert!(boil_energy_diff > 0.0, "Boiling energy should increase with pressure");

        println!("Pressure difference: {} Pa", pressure_diff);
        println!("Melting energy difference: {:.0} J", melt_energy_diff);
        println!("Boiling energy difference: {:.0} J", boil_energy_diff);
    }

    #[test]
    fn test_extreme_pressure_conditions() {
        // Test at very high pressure (100x atmospheric = 10,132,500 Pa)
        let cell_extreme_high = create_test_cell_at_pressure(10_132_500.0);
        let (melt_extreme, boil_extreme) = cell_extreme_high.get_phase_thresholds();

        // Test at very low pressure (0.01x atmospheric = 1,013.25 Pa)
        let cell_extreme_low = create_test_cell_at_pressure(1_013.25);
        let (melt_low, boil_low) = cell_extreme_low.get_phase_thresholds();

        // Standard pressure for comparison
        let cell_standard = create_test_cell_at_pressure(101325.0);
        let (melt_std, boil_std) = cell_standard.get_phase_thresholds();

        // Extreme high pressure should significantly increase thresholds
        assert!(melt_extreme > melt_std * 1.1, "Extreme high pressure should significantly increase melting threshold");
        assert!(boil_extreme > boil_std * 1.1, "Extreme high pressure should significantly increase boiling threshold");

        // Extreme low pressure should decrease thresholds (but may not be as dramatic as high pressure)
        assert!(melt_low < melt_std, "Extreme low pressure should decrease melting threshold");
        assert!(boil_low < boil_std, "Extreme low pressure should decrease boiling threshold");

        println!("Extreme conditions:");
        println!("  Very low pressure - Melt: {:.0}, Boil: {:.0}", melt_low, boil_low);
        println!("  Standard pressure - Melt: {:.0}, Boil: {:.0}", melt_std, boil_std);
        println!("  Very high pressure - Melt: {:.0}, Boil: {:.0}", melt_extreme, boil_extreme);
    }

    #[test]
    fn test_pressure_phase_transition_consistency() {
        // Create cells at different pressures but same initial conditions
        let low_pressure_cell = create_test_cell_at_pressure(50_000.0);   // ~0.5 atm
        let high_pressure_cell = create_test_cell_at_pressure(500_000.0); // ~5 atm

        // Both should start as solid at 273.15K
        assert_eq!(low_pressure_cell.material_phase, MaterialPhases::Solid);
        assert_eq!(high_pressure_cell.material_phase, MaterialPhases::Solid);

        // Get their respective melting thresholds
        let (melt_low, _) = low_pressure_cell.get_phase_thresholds();
        let (melt_high, _) = high_pressure_cell.get_phase_thresholds();

        // Verify the pressure effect is consistent with physics
        assert!(melt_high > melt_low, "Higher pressure should require more energy to melt");

        // The difference should be reasonable (not too extreme)
        let ratio = melt_high / melt_low;
        assert!(ratio > 1.0 && ratio < 2.0, "Pressure effect should be reasonable: ratio = {}", ratio);
    }
}
