#[cfg(test)]
mod layer_pressure_tests {
    use crate::sim::simulation::{Simulation, SimulationConfig};
    use crate::sim::layer_set::LayerSetParams;
    use crate::energy_mass::energy_mass::EnergyMass;
    use h3o::Resolution;

    fn create_test_simulation_config() -> SimulationConfig {
        SimulationConfig {
            steps: 10,
            years_per_step: 1.0,
            warmup_steps: 0,
            thermal_config: crate::sim::simulation::ThermalGradientConfig::earth_like(288.15), // 15°C surface
            layer_set_params: vec![
                // Surface layer (atmosphere/crust)
                LayerSetParams {
                    resolution: Resolution::Three,
                    start_height_km: 0.0, // Will be adjusted automatically
                    cell_height_km: 1.0,
                    material_name: "water".to_string(),
                    column_count: 3, // 3 cells = 3 km thick
                    planet_radius_km: 6371.0,
                },
                // Upper mantle layer
                LayerSetParams {
                    resolution: Resolution::Three,
                    start_height_km: 0.0, // Will be adjusted automatically
                    cell_height_km: 2.0,
                    material_name: "water".to_string(),
                    column_count: 2, // 2 cells = 4 km thick
                    planet_radius_km: 6371.0,
                },
                // Lower mantle layer
                LayerSetParams {
                    resolution: Resolution::Three,
                    start_height_km: 0.0, // Will be adjusted automatically
                    cell_height_km: 5.0,
                    material_name: "water".to_string(),
                    column_count: 1, // 1 cell = 5 km thick
                    planet_radius_km: 6371.0,
                },
            ],
        }
    }

    #[test]
    fn test_layer_stacking_and_pressure_calculation() {
        let config = create_test_simulation_config();
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let simulation = Simulation::new(config, &mut components);

        // Verify we have 3 layer sets
        assert_eq!(simulation.layer_sets.len(), 3);

        // Test that layers are stacked properly
        // Layer 0: starts at 0.0 km, 3 cells of 1 km each = 0-3 km
        // Layer 1: starts at 3.0 km, 2 cells of 2 km each = 3-7 km  
        // Layer 2: starts at 7.0 km, 1 cell of 5 km each = 7-12 km

        // Get a sample cell from each layer to check pressures
        let layer_0 = &simulation.layer_sets[0];
        let layer_1 = &simulation.layer_sets[1];
        let layer_2 = &simulation.layer_sets[2];

        // Verify layer start heights
        assert_eq!(layer_0.start_height_km, 0.0);
        assert_eq!(layer_1.start_height_km, 3.0);
        assert_eq!(layer_2.start_height_km, 7.0);

        // Get sample cells from each layer
        let sample_cell_0 = layer_0.layers.values().next().unwrap().cells.first().unwrap();
        let sample_cell_1 = layer_1.layers.values().next().unwrap().cells.first().unwrap();
        let sample_cell_2 = layer_2.layers.values().next().unwrap().cells.first().unwrap();

        // Verify pressures increase with depth
        let pressure_0 = sample_cell_0.pressure_pa();
        let pressure_1 = sample_cell_1.pressure_pa();
        let pressure_2 = sample_cell_2.pressure_pa();

        println!("Layer 0 pressure: {:.0} Pa", pressure_0);
        println!("Layer 1 pressure: {:.0} Pa", pressure_1);
        println!("Layer 2 pressure: {:.0} Pa", pressure_2);

        // Surface layer should be close to atmospheric pressure
        assert!(pressure_0 >= 101325.0, "Surface pressure should be at least atmospheric");
        assert!(pressure_0 < 200000.0, "Surface pressure should not be too high");

        // Deeper layers should have higher pressure
        assert!(pressure_1 > pressure_0, "Layer 1 should have higher pressure than Layer 0");
        assert!(pressure_2 > pressure_1, "Layer 2 should have higher pressure than Layer 1");

        // Pressure should increase significantly with depth
        assert!(pressure_2 > pressure_0 * 2.0, "Deep layer pressure should be significantly higher");

        println!("✅ Layer stacking and pressure calculation test passed");
        println!("   Pressure progression: {:.0} → {:.0} → {:.0} Pa", pressure_0, pressure_1, pressure_2);
    }

    #[test]
    fn test_mass_accumulation_affects_pressure() {
        let config = create_test_simulation_config();
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let simulation = Simulation::new(config, &mut components);

        // Get cells from the same column in different layers
        let first_cell_id = simulation.layer_sets[0].layers.keys().next().unwrap();
        
        let cell_layer_0 = &simulation.layer_sets[0].layers[first_cell_id].cells[0];
        let cell_layer_1 = &simulation.layer_sets[1].layers[first_cell_id].cells[0];
        let cell_layer_2 = &simulation.layer_sets[2].layers[first_cell_id].cells[0];

        // Calculate expected pressure differences based on mass
        let mass_0 = cell_layer_0.mass_kg();
        let mass_1 = cell_layer_1.mass_kg();
        let area = cell_layer_0.area();

        // Expected additional pressure from layer 0 mass
        let expected_pressure_increase_1 = (mass_0 / area / 1e6) * 9.81; // Convert km² to m²
        let expected_pressure_increase_2 = ((mass_0 + mass_1) / area / 1e6) * 9.81;

        let pressure_0 = cell_layer_0.pressure_pa();
        let pressure_1 = cell_layer_1.pressure_pa();
        let pressure_2 = cell_layer_2.pressure_pa();

        println!("Mass layer 0: {:.0} kg, Area: {:.2} km²", mass_0, area);
        println!("Expected pressure increase for layer 1: {:.0} Pa", expected_pressure_increase_1);
        println!("Actual pressure difference: {:.0} Pa", pressure_1 - pressure_0);

        // Verify that pressure increases are reasonable
        // Note: With thermal gradients, higher temperatures reduce density and thus mass
        assert!(pressure_1 - pressure_0 > expected_pressure_increase_1 * 0.1,
                "Pressure increase should be at least 10% of the expected value");
        assert!(pressure_2 > pressure_1,
                "Pressure should continue increasing with depth");

        println!("✅ Mass accumulation pressure test passed");
    }

    #[test]
    fn test_layer_continuity_top_bottom_alignment() {
        let config = create_test_simulation_config();
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let simulation = Simulation::new(config, &mut components);

        // Test configuration:
        // Layer 0: starts at 0.0 km, 3 cells of 1 km each = 0-3 km
        // Layer 1: starts at 3.0 km, 2 cells of 2 km each = 3-7 km
        // Layer 2: starts at 7.0 km, 1 cell of 5 km each = 7-12 km

        // Verify layer start heights are contiguous
        assert_eq!(simulation.layer_sets[0].start_height_km, 0.0, "Layer 0 should start at 0.0 km");
        assert_eq!(simulation.layer_sets[1].start_height_km, 3.0, "Layer 1 should start at 3.0 km");
        assert_eq!(simulation.layer_sets[2].start_height_km, 7.0, "Layer 2 should start at 7.0 km");

        // Get a sample column from each layer to check cell continuity
        let first_cell_id = simulation.layer_sets[0].layers.keys().next().unwrap();

        let column_0 = &simulation.layer_sets[0].layers[first_cell_id];
        let column_1 = &simulation.layer_sets[1].layers[first_cell_id];
        let column_2 = &simulation.layer_sets[2].layers[first_cell_id];

        println!("=== Layer 0 Cells ===");
        for (i, cell) in column_0.cells.iter().enumerate() {
            let top = cell.top_km;
            let bottom = cell.bottom_km;
            println!("  Cell {}: top={:.1} km, bottom={:.1} km, height={:.1} km",
                    i, top, bottom, cell.height_km);
        }

        println!("=== Layer 1 Cells ===");
        for (i, cell) in column_1.cells.iter().enumerate() {
            let top = cell.top_km;
            let bottom = cell.bottom_km;
            println!("  Cell {}: top={:.1} km, bottom={:.1} km, height={:.1} km",
                    i, top, bottom, cell.height_km);
        }

        println!("=== Layer 2 Cells ===");
        for (i, cell) in column_2.cells.iter().enumerate() {
            let top = cell.top_km;
            let bottom = cell.bottom_km;
            println!("  Cell {}: top={:.1} km, bottom={:.1} km, height={:.1} km",
                    i, top, bottom, cell.height_km);
        }

        // Verify continuity within Layer 0
        for i in 0..column_0.cells.len() - 1 {
            let current_bottom = column_0.cells[i].bottom_km;
            let next_top = column_0.cells[i + 1].top_km;
            assert_eq!(current_bottom, next_top,
                      "Layer 0: Cell {} bottom ({:.1}) should equal Cell {} top ({:.1})",
                      i, current_bottom, i + 1, next_top);
        }

        // Verify continuity within Layer 1
        for i in 0..column_1.cells.len() - 1 {
            let current_bottom = column_1.cells[i].bottom_km;
            let next_top = column_1.cells[i + 1].top_km;
            assert_eq!(current_bottom, next_top,
                      "Layer 1: Cell {} bottom ({:.1}) should equal Cell {} top ({:.1})",
                      i, current_bottom, i + 1, next_top);
        }

        // Verify continuity between Layer 0 and Layer 1
        let layer_0_last_bottom = column_0.cells.last().unwrap().bottom_km;
        let layer_1_first_top = column_1.cells.first().unwrap().top_km;
        assert_eq!(layer_0_last_bottom, layer_1_first_top,
                  "Layer 0 bottom ({:.1}) should equal Layer 1 top ({:.1})",
                  layer_0_last_bottom, layer_1_first_top);

        // Verify continuity between Layer 1 and Layer 2
        let layer_1_last_bottom = column_1.cells.last().unwrap().bottom_km;
        let layer_2_first_top = column_2.cells.first().unwrap().top_km;
        assert_eq!(layer_1_last_bottom, layer_2_first_top,
                  "Layer 1 bottom ({:.1}) should equal Layer 2 top ({:.1})",
                  layer_1_last_bottom, layer_2_first_top);

        // Verify total depth coverage
        let surface_top = column_0.cells.first().unwrap().top_km;
        let deepest_bottom = column_2.cells.last().unwrap().bottom_km;
        let expected_total_depth = 3.0 * 1.0 + 2.0 * 2.0 + 1.0 * 5.0; // 3 + 4 + 5 = 12 km

        assert_eq!(surface_top, 0.0, "Surface should start at 0.0 km");
        assert_eq!(deepest_bottom, expected_total_depth,
                  "Total depth should be {:.1} km, got {:.1} km",
                  expected_total_depth, deepest_bottom);

        println!("✅ Layer continuity test passed");
        println!("   Total depth coverage: {:.1} km (surface to {:.1} km)",
                expected_total_depth, deepest_bottom);
        println!("   All layers and cells are perfectly contiguous!");
    }

    #[test]
    fn test_thermal_gradient_initialization() {
        let config = create_test_simulation_config();
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let simulation = Simulation::new(config, &mut components);

        // Test quadratic thermal configuration:
        // Surface temperature: 288.15 K (15°C)
        // Surface gradient: 25 K/km decreasing to 10 K/km at 200 km depth
        // Quadratic model provides more realistic deep temperatures

        let first_cell_id = simulation.layer_sets[0].layers.keys().next().unwrap();

        let column_0 = &simulation.layer_sets[0].layers[first_cell_id];
        let column_1 = &simulation.layer_sets[1].layers[first_cell_id];
        let column_2 = &simulation.layer_sets[2].layers[first_cell_id];

        println!("=== Quadratic Thermal Gradient Test ===");
        println!("Surface temperature: {:.1} K ({:.1}°C)", 288.15, 288.15 - 273.15);
        println!("Surface gradient: 25.0 K/km → Deep gradient: 10.0 K/km at 200 km");
        println!();

        // Test Layer 0 temperatures
        for (i, cell) in column_0.cells.iter().enumerate() {
            let depth_center = cell.top_km + cell.height_km / 2.0;
            let expected_temp = simulation.calculate_temperature_at_depth(depth_center);
            let actual_temp = cell.temperature_kelvin();
            let gradient_at_depth = simulation.thermal_config().gradient_at_depth(depth_center);

            println!("Layer 0, Cell {}: depth={:.1}km, temp={:.1}K ({:.1}°C), gradient={:.1}K/km",
                    i, depth_center, actual_temp, actual_temp - 273.15, gradient_at_depth);

            assert!((actual_temp - expected_temp).abs() < 1.0,
                   "Temperature should be close to expected: got {:.1}K, expected {:.1}K",
                   actual_temp, expected_temp);
        }

        // Test Layer 1 temperatures
        for (i, cell) in column_1.cells.iter().enumerate() {
            let depth_center = cell.top_km + cell.height_km / 2.0;
            let expected_temp = simulation.calculate_temperature_at_depth(depth_center);
            let actual_temp = cell.temperature_kelvin();
            let gradient_at_depth = simulation.thermal_config().gradient_at_depth(depth_center);

            println!("Layer 1, Cell {}: depth={:.1}km, temp={:.1}K ({:.1}°C), gradient={:.1}K/km",
                    i, depth_center, actual_temp, actual_temp - 273.15, gradient_at_depth);

            assert!((actual_temp - expected_temp).abs() < 1.0,
                   "Temperature should be close to expected: got {:.1}K, expected {:.1}K",
                   actual_temp, expected_temp);
        }

        // Test Layer 2 temperatures
        for (i, cell) in column_2.cells.iter().enumerate() {
            let depth_center = cell.top_km + cell.height_km / 2.0;
            let expected_temp = simulation.calculate_temperature_at_depth(depth_center);
            let actual_temp = cell.temperature_kelvin();
            let gradient_at_depth = simulation.thermal_config().gradient_at_depth(depth_center);

            println!("Layer 2, Cell {}: depth={:.1}km, temp={:.1}K ({:.1}°C), gradient={:.1}K/km",
                    i, depth_center, actual_temp, actual_temp - 273.15, gradient_at_depth);

            assert!((actual_temp - expected_temp).abs() < 1.0,
                   "Temperature should be close to expected: got {:.1}K, expected {:.1}K",
                   actual_temp, expected_temp);
        }

        // Verify temperature increases with depth
        let surface_temp = column_0.cells[0].temperature_kelvin();
        let mid_temp = column_1.cells[0].temperature_kelvin();
        let deep_temp = column_2.cells[0].temperature_kelvin();

        assert!(mid_temp > surface_temp, "Mid-depth temperature should be higher than surface");
        assert!(deep_temp > mid_temp, "Deep temperature should be higher than mid-depth");

        // Verify quadratic gradient behavior
        let total_depth = column_2.cells[0].top_km + column_2.cells[0].height_km / 2.0;
        let surface_gradient = simulation.thermal_config().gradient_at_depth(0.0);
        let deep_gradient = simulation.thermal_config().gradient_at_depth(total_depth);

        println!();
        println!("Temperature progression: {:.1}K → {:.1}K → {:.1}K", surface_temp, mid_temp, deep_temp);
        println!("Gradient progression: {:.1} → {:.1} K/km", surface_gradient, deep_gradient);

        // Verify gradient decreases with depth
        assert!(deep_gradient < surface_gradient,
               "Deep gradient should be less than surface gradient: {:.1} vs {:.1}",
               deep_gradient, surface_gradient);

        // Verify gradients are in expected range
        assert!(surface_gradient > 20.0 && surface_gradient < 30.0,
               "Surface gradient should be ~25 K/km: got {:.1}", surface_gradient);

        println!("✅ Quadratic thermal gradient initialization test passed");
        println!("   Surface: {:.1}°C, Deep: {:.1}°C over {:.1} km",
                surface_temp - 273.15, deep_temp - 273.15, total_depth);
        println!("   Gradient: {:.1} K/km → {:.1} K/km", surface_gradient, deep_gradient);
    }

    #[test]
    fn test_quadratic_gradient_at_deep_depths() {
        let config = create_test_simulation_config();
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let simulation = Simulation::new(config, &mut components);

        println!("=== Deep Quadratic Gradient Test ===");

        // Test gradient at various depths to show quadratic behavior
        let test_depths = vec![0.0, 10.0, 50.0, 100.0, 150.0, 200.0, 250.0];

        for depth in test_depths {
            let gradient = simulation.thermal_config().gradient_at_depth(depth);
            let temperature = simulation.calculate_temperature_at_depth(depth);

            println!("Depth: {:3.0} km, Gradient: {:4.1} K/km, Temp: {:5.1} K ({:5.1}°C)",
                    depth, gradient, temperature, temperature - 273.15);
        }

        // Verify gradient decreases with depth
        let surface_gradient = simulation.thermal_config().gradient_at_depth(0.0);
        let mid_gradient = simulation.thermal_config().gradient_at_depth(100.0);
        let deep_gradient = simulation.thermal_config().gradient_at_depth(200.0);

        assert!(surface_gradient > mid_gradient,
               "Mid-depth gradient should be less than surface: {:.1} vs {:.1}",
               mid_gradient, surface_gradient);
        assert!(mid_gradient > deep_gradient,
               "Deep gradient should be less than mid-depth: {:.1} vs {:.1}",
               deep_gradient, mid_gradient);

        // Verify specific gradient values
        assert!((surface_gradient - 25.0).abs() < 0.1,
               "Surface gradient should be ~25 K/km: got {:.1}", surface_gradient);
        assert!((deep_gradient - 10.0).abs() < 0.1,
               "Deep gradient should be ~10 K/km: got {:.1}", deep_gradient);

        // Verify temperature at 200 km is much cooler than linear gradient would give
        let temp_200km = simulation.calculate_temperature_at_depth(200.0);
        let linear_temp_200km = 288.15 + (200.0 * 25.0); // What linear 25 K/km would give

        assert!(temp_200km < linear_temp_200km,
               "Quadratic gradient should give cooler temps than linear: {:.1}°C vs {:.1}°C",
               temp_200km - 273.15, linear_temp_200km - 273.15);

        println!("   Linear gradient would give: {:.1}°C at 200 km", linear_temp_200km - 273.15);
        println!("   Quadratic gradient gives: {:.1}°C at 200 km", temp_200km - 273.15);
        println!("   Savings: {:.1}°C cooler!", (linear_temp_200km - temp_200km));

        println!();
        println!("✅ Deep quadratic gradient test passed");
        println!("   Gradient: {:.1} K/km → {:.1} K/km → {:.1} K/km",
                surface_gradient, mid_gradient, deep_gradient);
        println!("   Temperature at 200 km: {:.1}°C", temp_200km - 273.15);
    }
}
