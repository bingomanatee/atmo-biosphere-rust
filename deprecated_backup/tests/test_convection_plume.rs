#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::MaterialPhase;
    use crate::material::MaterialPhases;

    #[test]
    fn test_buoyancy_velocity_calculation() {
        let component = ConvectionPlumeComponent::new();
        
        // Test case: hot plume (lower density) rising through cooler ambient material
        let plume_density = 3200.0; // kg/m³ - hot, less dense material
        let ambient_density = 3300.0; // kg/m³ - cooler, denser material
        let plume_radius_km = 5.0;
        
        let velocity = component.calculate_buoyancy_velocity(
            plume_density, 
            ambient_density, 
            plume_radius_km
        );
        
        // Should have positive velocity (upward movement)
        assert!(velocity > 0.0, "Hot plume should rise with positive velocity");
        assert!(velocity < 100.0, "Velocity should be capped at reasonable geological rates");
        
        println!("Buoyancy velocity for Δρ = {} kg/m³: {:.6} km/year", 
            ambient_density - plume_density, velocity);
    }

    #[test]
    fn test_no_buoyancy_when_denser() {
        let component = ConvectionPlumeComponent::new();
        
        // Test case: dense plume (higher density) - should not rise
        let plume_density = 3400.0; // kg/m³ - denser material
        let ambient_density = 3300.0; // kg/m³ - less dense ambient
        let plume_radius_km = 5.0;
        
        let velocity = component.calculate_buoyancy_velocity(
            plume_density, 
            ambient_density, 
            plume_radius_km
        );
        
        // Should have zero velocity (no buoyancy)
        assert_eq!(velocity, 0.0, "Dense plume should not rise");
    }

    #[test]
    fn test_area_scaled_probability() {
        let component = ConvectionPlumeComponent::new();
        
        // Test that probability scales with area
        let temp_excess = 500.0; // K above threshold
        let years_per_step = 100.0;
        
        let small_area = 100.0; // km²
        let large_area = 400.0; // km² (4x larger)
        
        let prob_small = component.calculate_plume_probability(small_area, years_per_step, temp_excess);
        let prob_large = component.calculate_plume_probability(large_area, years_per_step, temp_excess);
        
        // Larger area should have proportionally higher probability
        assert!(prob_large > prob_small, "Larger area should have higher plume probability");
        assert!((prob_large / prob_small - 4.0).abs() < 0.1, 
            "Probability should scale linearly with area");
        
        println!("Small area ({} km²) probability: {:.2e}", small_area, prob_small);
        println!("Large area ({} km²) probability: {:.2e}", large_area, prob_large);
        println!("Ratio: {:.2}", prob_large / prob_small);
    }

    #[test]
    fn test_temperature_enhancement() {
        let component = ConvectionPlumeComponent::new();
        
        let area = 100.0; // km²
        let years_per_step = 100.0;
        
        let low_temp_excess = 100.0; // K
        let high_temp_excess = 500.0; // K
        
        let prob_low = component.calculate_plume_probability(area, years_per_step, low_temp_excess);
        let prob_high = component.calculate_plume_probability(area, years_per_step, high_temp_excess);
        
        // Higher temperature should dramatically increase probability
        assert!(prob_high > prob_low, "Higher temperature should increase plume probability");
        assert!(prob_high / prob_low > 10.0, "Temperature should have exponential effect");
        
        println!("Low temp excess ({} K) probability: {:.2e}", low_temp_excess, prob_low);
        println!("High temp excess ({} K) probability: {:.2e}", high_temp_excess, prob_high);
        println!("Enhancement factor: {:.1}", prob_high / prob_low);
    }

    #[test]
    fn test_density_calculation_with_pressure_temperature() {
        let component = ConvectionPlumeComponent::new();
        
        // Create a mock material phase for testing
        let material = MaterialPhase {
            density_kg_m3: 3300.0,
            thermal_conductivity_w_m_k: 3.0,
            specific_heat_capacity_j_kg_k: 1000.0,
            latent_heat_fusion: 400000.0,
            latent_heat_vapor: 6000000.0,
            melt_temp: 1473.0,
            melt_temp_min: Some(1400.0),
            melt_temp_max: Some(1500.0),
            boil_temp: 3000.0,
            gas_interference_factor: Some(0.1),
            thermal_conduction_modifier_dimensionless: 1000.0,
            thermal_expansivity_per_k: 200.0,
            dynamic_viscosity_pa_s: 1e21,
            bulk_modulus_pa: 130e9,
            activation_energy_j_per_mol: Some(500000.0),
            activation_volume_m3_per_mol: Some(5e-6),
            cool_temp_min: Some(200.0),
            cool_temp_max: Some(300.0),
        };
        
        let pressure_pa = 1e9; // 1 GPa
        let temperature_k = 1800.0; // Hot temperature
        
        let density = component.calculate_density(&material, pressure_pa, temperature_k);
        
        // Should return a reasonable density value
        assert!(density > 2000.0, "Density should be reasonable for rock material");
        assert!(density < 5000.0, "Density should not be unrealistically high");
        
        println!("Calculated density at {} Pa, {} K: {:.1} kg/m³", 
            pressure_pa, temperature_k, density);
    }

    #[test]
    fn test_plume_physics_integration() {
        let component = ConvectionPlumeComponent::new();
        
        println!("\n🌋 Convection Plume Physics Test");
        println!("================================");
        
        // Test realistic geological scenario
        let hot_density = 3200.0; // kg/m³ - hot mantle material
        let cool_density = 3300.0; // kg/m³ - cooler mantle material
        let plume_radius = 10.0; // km
        
        let velocity = component.calculate_buoyancy_velocity(hot_density, cool_density, plume_radius);
        
        println!("Hot plume density: {} kg/m³", hot_density);
        println!("Cool ambient density: {} kg/m³", cool_density);
        println!("Density difference: {} kg/m³", cool_density - hot_density);
        println!("Plume radius: {} km", plume_radius);
        println!("Calculated buoyancy velocity: {:.3} km/year", velocity);
        
        // Verify reasonable geological velocity
        assert!(velocity > 0.001, "Velocity should be measurable on geological timescales");
        assert!(velocity < 50.0, "Velocity should not exceed realistic mantle convection rates");
        
        // Test area scaling
        let cell_area_1km2 = 1.0;
        let cell_area_100km2 = 100.0;
        let temp_excess = 300.0;
        let years_per_step = 1000.0;
        
        let prob_1km2 = component.calculate_plume_probability(cell_area_1km2, years_per_step, temp_excess);
        let prob_100km2 = component.calculate_plume_probability(cell_area_100km2, years_per_step, temp_excess);
        
        println!("\nArea scaling test:");
        println!("1 km² cell probability: {:.2e}", prob_1km2);
        println!("100 km² cell probability: {:.2e}", prob_100km2);
        println!("Scaling factor: {:.1}", prob_100km2 / prob_1km2);
        
        assert!((prob_100km2 / prob_1km2 - 100.0).abs() < 1.0, 
            "Probability should scale linearly with area");
        
        println!("\n✅ All buoyancy physics tests passed!");
    }
}
