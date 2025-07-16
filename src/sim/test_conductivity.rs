#[cfg(test)]
mod tests {
    use crate::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
    use crate::energy_mass::energy_mass::EnergyMass;
    use h3o::{CellIndex, Resolution};

    fn create_test_cell(temperature_k: f64, pressure_pa: f64, height_km: f64) -> EnergyMassCell {
        let cell_index = CellIndex::base_cells().next().unwrap()
            .children(Resolution::Three).next().unwrap();
        
        let props = EnergyMassCellProps {
            cell_index,
            temperature_kelvin: temperature_k,
            pressure_pa,
            height_km,
            top_km: 0.0,
            material_name: "basalt".to_string(),
            planet_radius_km: 6371.0, // Earth radius
        };
        
        EnergyMassCell::new(props)
    }

    #[test]
    fn test_conductivity_calculation() {
        let mut cell = create_test_cell(1500.0, 1e5, 10.0);

        // Get conductivity - should compute and cache it
        let conductivity1 = cell.get_conductivity_w_m_k();
        assert!(conductivity1 > 0.0, "Conductivity should be positive");

        // Get conductivity again - should return cached value
        let conductivity2 = cell.get_conductivity_w_m_k();
        assert_eq!(conductivity1, conductivity2, "Cached conductivity should be the same");

        // Force recomputation
        cell.recompute_conductivity();
        let conductivity3 = cell.get_conductivity_w_m_k();
        assert_eq!(conductivity1, conductivity3, "Recomputed conductivity should be the same");
    }

    #[test]
    fn test_conductivity_invalidation() {
        let mut cell = create_test_cell(1500.0, 1e5, 10.0);

        // Get initial conductivity
        let initial_conductivity = cell.get_conductivity_w_m_k();

        // Change pressure - should invalidate conductivity
        cell.set_pressure_pa(2e5);
        let new_conductivity = cell.get_conductivity_w_m_k();

        // Conductivity should be different due to pressure change
        assert_ne!(initial_conductivity, new_conductivity,
                   "Conductivity should change when pressure changes");
    }

    #[test]
    fn test_energy_transmission() {
        let mut hot_cell = create_test_cell(2000.0, 1e5, 10.0);
        let mut cold_cell = create_test_cell(1000.0, 1e5, 10.0);

        let initial_hot_temp = hot_cell.temperature_kelvin();
        let initial_cold_temp = cold_cell.temperature_kelvin();

        // Transmit energy from hot to cold cell
        hot_cell.transmit_energy(&mut cold_cell, true);

        // Check pending energy deltas
        let hot_pending = hot_cell.pending_energy_delta();
        let cold_pending = cold_cell.pending_energy_delta();

        // Hot cell should lose energy (negative delta), cold cell should gain energy (positive delta)
        assert!(hot_pending < 0.0, "Hot cell should have negative pending energy");
        assert!(cold_pending > 0.0, "Cold cell should have positive pending energy");

        // Energy should be conserved
        assert!((hot_pending + cold_pending).abs() < 1e-10,
                "Energy should be conserved: {} + {} = {}",
                hot_pending, cold_pending, hot_pending + cold_pending);
    }

    #[test]
    fn test_commit_energy_changes() {
        let mut hot_cell = create_test_cell(2000.0, 1e5, 10.0);
        let mut cold_cell = create_test_cell(1000.0, 1e5, 10.0);

        let initial_hot_temp = hot_cell.temperature_kelvin();
        let initial_cold_temp = cold_cell.temperature_kelvin();

        // Transmit energy
        hot_cell.transmit_energy(&mut cold_cell, true);

        // Commit changes
        hot_cell.commit();
        cold_cell.commit();

        // Check that pending deltas are cleared
        assert_eq!(hot_cell.pending_energy_delta(), 0.0, "Hot cell pending should be cleared");
        assert_eq!(cold_cell.pending_energy_delta(), 0.0, "Cold cell pending should be cleared");

        // Check that temperatures have changed appropriately
        let final_hot_temp = hot_cell.temperature_kelvin();
        let final_cold_temp = cold_cell.temperature_kelvin();

        assert!(final_hot_temp < initial_hot_temp, "Hot cell should have cooled down");
        assert!(final_cold_temp > initial_cold_temp, "Cold cell should have warmed up");
    }

    #[test]
    fn test_conductance_coefficient_calculation() {
        let mut cell1 = create_test_cell(1500.0, 1e5, 10.0);
        let mut cell2 = create_test_cell(1600.0, 1e5, 10.0);

        // Test vertical neighbor (above/below)
        let vertical_conductance = cell1.calculate_conductance_coefficient(&mut cell2, true);
        assert!(vertical_conductance > 0.0, "Vertical conductance should be positive");

        // Test horizontal neighbor (side by side)
        let horizontal_conductance = cell1.calculate_conductance_coefficient(&mut cell2, false);
        assert!(horizontal_conductance > 0.0, "Horizontal conductance should be positive");

        // Vertical and horizontal conductance should be different due to different contact areas
        assert_ne!(vertical_conductance, horizontal_conductance,
                   "Vertical and horizontal conductance should be different");
    }
}
