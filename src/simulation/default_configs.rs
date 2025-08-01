use crate::simulation::{LayerConfig, PlanetConfig, SimulationConfig};
use h3o::Resolution;

/// Default Earth-like planet configuration
pub fn default_planet_config() -> PlanetConfig {
    PlanetConfig {
        radius_km: 6371.0,                    // Earth radius
        surface_gravity_m_s_s: 9.81,          // Earth gravity
        surface_temperature_k: 288.15,        // Earth surface temperature (15°C)
    }
}

/// Default billion-year proven layer configuration
/// Based on the successful billion_year_radiance.rs example
/// 
/// This configuration has been tested and proven for:
/// - Billion-year geological simulations
/// - Realistic temperature gradients
/// - Proper depth scaling
/// - Efficient computation
pub fn default_billion_year_layers() -> Vec<LayerConfig> {
    vec![
        // Continental Crust Layer (0-20km depth)
        LayerConfig {
            height_per_step_km: 5.0,              // 5km per depth step
            depth_steps: 4,                       // 4 steps = 20km total thickness
            resolution: Resolution::Four,          // ~1,770 km² per cell (realistic for testing)
            name: "Continental Crust".to_string(),
            temperature_gradient_k_per_km: 25.0,   // High gradient: 25K/km
        },
        
        // Upper Mantle Layer (20-170km depth)  
        LayerConfig {
            height_per_step_km: 25.0,             // 25km per depth step
            depth_steps: 6,                       // 6 steps = 150km thickness
            resolution: Resolution::Four,          // Same resolution for efficiency
            name: "Upper Mantle".to_string(),
            temperature_gradient_k_per_km: 15.0,   // Moderate gradient: 15K/km
        },
        
        // Lower Mantle Layer (170-320km depth)
        LayerConfig {
            height_per_step_km: 50.0,             // 50km per depth step  
            depth_steps: 3,                       // 3 steps = 150km thickness
            resolution: Resolution::Four,          // Same resolution
            name: "Lower Mantle".to_string(),
            temperature_gradient_k_per_km: 10.0,   // Low gradient: 10K/km
        },
    ]
}

/// Default billion-year proven layer configuration with Resolution::Three
/// For more realistic testing (fewer cells than Resolution::Four)
pub fn default_billion_year_layers_res3() -> Vec<LayerConfig> {
    vec![
        // Continental Crust Layer (0-20km depth)
        LayerConfig {
            height_per_step_km: 5.0,              // 5km per depth step
            depth_steps: 4,                       // 4 steps = 20km total thickness
            resolution: Resolution::Three,         // ~2,082 km² per cell (realistic for testing)
            name: "Continental Crust".to_string(),
            temperature_gradient_k_per_km: 25.0,   // High gradient: 25K/km
        },
        
        // Upper Mantle Layer (20-170km depth)  
        LayerConfig {
            height_per_step_km: 25.0,             // 25km per depth step
            depth_steps: 6,                       // 6 steps = 150km thickness
            resolution: Resolution::Three,         // Same resolution for efficiency
            name: "Upper Mantle".to_string(),
            temperature_gradient_k_per_km: 15.0,   // Moderate gradient: 15K/km
        },
        
        // Lower Mantle Layer (170-320km depth)
        LayerConfig {
            height_per_step_km: 50.0,             // 50km per depth step  
            depth_steps: 3,                       // 3 steps = 150km thickness
            resolution: Resolution::Three,         // Same resolution
            name: "Lower Mantle".to_string(),
            temperature_gradient_k_per_km: 10.0,   // Low gradient: 10K/km
        },
    ]
}

/// Default billion-year proven layer configuration with Resolution::Two
/// For production-scale simulations
pub fn default_billion_year_layers_res2() -> Vec<LayerConfig> {
    vec![
        // Continental Crust Layer (0-20km depth)
        LayerConfig {
            height_per_step_km: 5.0,              // 5km per depth step
            depth_steps: 4,                       // 4 steps = 20km total thickness
            resolution: Resolution::Two,           // ~5,882 km² per cell (production scale)
            name: "Continental Crust".to_string(),
            temperature_gradient_k_per_km: 25.0,   // High gradient: 25K/km
        },
        
        // Upper Mantle Layer (20-170km depth)  
        LayerConfig {
            height_per_step_km: 25.0,             // 25km per depth step
            depth_steps: 6,                       // 6 steps = 150km thickness
            resolution: Resolution::Two,           // Same resolution for efficiency
            name: "Upper Mantle".to_string(),
            temperature_gradient_k_per_km: 15.0,   // Moderate gradient: 15K/km
        },
        
        // Lower Mantle Layer (170-320km depth)
        LayerConfig {
            height_per_step_km: 50.0,             // 50km per depth step  
            depth_steps: 3,                       // 3 steps = 150km thickness
            resolution: Resolution::Two,           // Same resolution
            name: "Lower Mantle".to_string(),
            temperature_gradient_k_per_km: 10.0,   // Low gradient: 10K/km
        },
    ]
}

/// Complete default simulation configuration for billion-year geological simulations
pub fn default_billion_year_config() -> SimulationConfig {
    SimulationConfig {
        planet: default_planet_config(),
        layers: default_billion_year_layers(),
        years_per_step: 1_000_000,            // 1 million years per step
        steps: 1000,                          // 1000 steps = 1 billion years total
    }
}

/// Complete default simulation configuration for testing (shorter duration)
pub fn default_test_config() -> SimulationConfig {
    SimulationConfig {
        planet: default_planet_config(),
        layers: default_billion_year_layers_res3(), // Use Resolution::Three for faster testing
        years_per_step: 100_000,              // 100k years per step
        steps: 10,                            // 10 steps = 1 million years total
    }
}

/// Complete default simulation configuration for plume testing
pub fn default_plume_test_config() -> SimulationConfig {
    SimulationConfig {
        planet: default_planet_config(),
        layers: default_billion_year_layers_res3(), // Use Resolution::Three for realistic testing
        years_per_step: 100_000,              // 100k years per step
        steps: 5,                             // 5 steps = 500k years total
    }
}

/// Summary of the default billion-year layer configuration:
/// 
/// **Total Structure:**
/// - **Total cells per column**: 13 (4 crust + 6 upper mantle + 3 lower mantle)
/// - **Total depth**: 320km (20km + 150km + 150km)
/// - **Temperature range**: 288K to ~4,488K (surface to deep)
/// 
/// **Layer Details:**
/// 1. **Continental Crust** (0-20km):
///    - 4 cells × 5km = 20km depth
///    - 25K/km gradient = 500K temperature increase
///    - Range: 288K to 788K
/// 
/// 2. **Upper Mantle** (20-170km):
///    - 6 cells × 25km = 150km depth  
///    - 15K/km gradient = 2,250K temperature increase
///    - Range: 788K to 3,038K
/// 
/// 3. **Lower Mantle** (170-320km):
///    - 3 cells × 50km = 150km depth
///    - 10K/km gradient = 1,500K temperature increase
///    - Range: 3,038K to 4,538K
/// 
/// **Why This Configuration Works:**
/// - ✅ Proven in billion-year simulations
/// - ✅ Realistic Earth-like temperature gradients
/// - ✅ Proper depth scaling (fine → coarse with depth)
/// - ✅ Efficient computation (same resolution across layers)
/// - ✅ Good for plume formation (strong temperature contrasts)
/// - ✅ Suitable for atmospheric modeling (realistic surface conditions)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configs() {
        let planet = default_planet_config();
        assert_eq!(planet.radius_km, 6371.0);
        assert_eq!(planet.surface_gravity_m_s_s, 9.81);
        assert_eq!(planet.surface_temperature_k, 288.15);

        let layers = default_billion_year_layers();
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].name, "Continental Crust");
        assert_eq!(layers[1].name, "Upper Mantle");
        assert_eq!(layers[2].name, "Lower Mantle");

        // Test total depth calculation
        let total_depth: f64 = layers.iter()
            .map(|layer| layer.height_per_step_km * layer.depth_steps as f64)
            .sum();
        assert_eq!(total_depth, 320.0); // 20 + 150 + 150 = 320km

        // Test total cells per column
        let total_cells: u32 = layers.iter()
            .map(|layer| layer.depth_steps)
            .sum();
        assert_eq!(total_cells, 13); // 4 + 6 + 3 = 13 cells per column
    }

    #[test]
    fn test_billion_year_config() {
        let config = default_billion_year_config();
        let total_years = config.steps as u64 * config.years_per_step as u64;
        assert_eq!(total_years, 1_000_000_000); // 1 billion years
    }

    #[test]
    fn test_different_resolutions() {
        let res2 = default_billion_year_layers_res2();
        let res3 = default_billion_year_layers_res3();
        let res4 = default_billion_year_layers();

        // All should have same structure, different resolutions
        assert_eq!(res2.len(), res3.len());
        assert_eq!(res3.len(), res4.len());
        
        assert_eq!(res2[0].resolution, Resolution::Two);
        assert_eq!(res3[0].resolution, Resolution::Three);
        assert_eq!(res4[0].resolution, Resolution::Four);
    }
}
