use atmo_biosphere_rust::components::{BinaryPairComponent, LayerCellComponent, RadianceComponent, MetricsReportingComponent};
use atmo_biosphere_rust::simulation::{LayerConfig, PlanetConfig, Simulation, SimulationConfig};
use h3o::Resolution;

/// Billion Year Geological Simulation with Thermal Radiance
/// 
/// This example demonstrates a realistic geological simulation over billion-year timescales
/// using the RadianceComponent for high-temperature heat transfer and LayerCellComponent
/// for proper geological initialization.
/// 
/// The simulation models:
/// - Continental crust (0-20km): Granite composition, high thermal gradient
/// - Upper mantle (20-170km): Basalt composition, moderate thermal gradient  
/// - Lower mantle (170-320km): Peridotite composition, low thermal gradient
/// - Thermal radiance between cells using Stefan-Boltzmann law
/// - Realistic planetary parameters (Earth-like)

fn main() {
    println!("🌍 Starting Billion Year Geological Simulation with Thermal Radiance");
    println!("=====================================================================");
    
    // Create Earth-like planetary configuration
    let planet_config = PlanetConfig {
        radius_km: 6371.0,                    // Earth radius
        surface_gravity_m_s_s: 9.81,          // Earth gravity
        surface_temperature_k: 288.15,        // 15°C surface temperature
    };
    
    // Define geological layers with realistic properties
    let layers = vec![
        // Continental Crust Layer (0-20km depth)
        LayerConfig {
            height_per_step_km: 5.0,              // 5km per depth step
            depth_steps: 4,                       // 4 steps = 20km total thickness
            resolution: Resolution::Four,          // ~1,770 km² per cell
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
    ];
    
    // Create simulation configuration for billion-year timescales
    let config = SimulationConfig {
        planet: planet_config,
        layers,
        years_per_step: 1_000_000u32,            // 1 million years per step
        steps: 1000u32,                         // 1000 steps = 1 billion years total
    };
    
    println!("📊 Simulation Configuration:");
    println!("   • Planet: Earth-like (radius: {:.0} km, gravity: {:.2} m/s²)", 
             config.planet.radius_km, config.planet.surface_gravity_m_s_s);
    println!("   • Surface Temperature: {:.1}°C", config.planet.surface_temperature_k - 273.15);
    println!("   • Time Scale: {} years per step", config.years_per_step);
    println!("   • Total Duration: {} billion years", 
             (config.steps as u64 * config.years_per_step as u64) / 1_000_000_000);
    println!("   • Geological Layers: {}", config.layers.len());
    
    for (i, layer) in config.layers.iter().enumerate() {
        let total_thickness = layer.height_per_step_km * layer.depth_steps as f64;
        println!("     - {}: {:.0}km thick, {:.0}K/km gradient", 
                 layer.name, total_thickness, layer.temperature_gradient_k_per_km);
    }
    
    // Initialize simulation
    println!("\n🔧 Initializing Simulation...");
    let mut sim = Simulation::new(config.clone());
    
    // Initialize geological cells
    sim.initialize_cells();
    let initial_stats = sim.get_stats();
    println!("   • Created {} geological cells", initial_stats.total_cells);
    
    // Add LayerCellComponent for geological initialization
    println!("   • Adding LayerCellComponent for geological properties...");
    sim.add_component(Box::new(LayerCellComponent::new()));

    // Add BinaryPairComponent to create cell relationships (required for RadianceComponent)
    println!("   • Adding BinaryPairComponent for cell relationships...");
    sim.add_component(Box::new(BinaryPairComponent::new()));

    // Add RadianceComponent for thermal radiance (now with efficient BinaryPairListener pattern)
    println!("   • Adding RadianceComponent for thermal radiance...");
    let radiance_component = RadianceComponent::with_emissivity(0.95); // High emissivity for geological materials
    sim.add_component(Box::new(radiance_component));

    // Add MetricsReportingComponent for performance analysis
    println!("   • Adding MetricsReportingComponent for performance tracking...");
    let metrics_component = MetricsReportingComponent::with_settings(
        true,  // detailed_reporting
        true,  // component_analysis
        true,  // trend_analysis
        0.1    // min_duration_threshold_ms
    );
    sim.add_component(Box::new(metrics_component));

    // Initialize all components
    sim.initialize_components();
    
    // Display initial conditions
    println!("\n🌡️  Initial Thermal Conditions:");
    display_thermal_profile(&sim, &config);
    
    // Run billion-year simulation with progress reporting
    println!("\n🚀 Running Billion Year Simulation...");
    println!("    (Progress will be reported every 100 million years)");
    
    let progress_interval = 100; // Report every 100 steps (100 million years)
    
    for step in 1..=config.steps {
        sim.step();
        
        // Report progress every 100 million years
        if step % progress_interval == 0 {
            let years_elapsed = step as u64 * config.years_per_step as u64;
            let billion_years = years_elapsed as f64 / 1_000_000_000.0;
            
            println!("\n📈 Progress: {:.1} billion years elapsed (step {})", billion_years, step);
            display_thermal_profile(&sim, &config);
            
            // Display energy transfer statistics
            let stats = sim.get_stats();
            println!("   • Total simulation time: {:.1} billion years",
                     stats.years_simulated as f64 / 1_000_000_000.0);
        }
    }
    
    // Complete simulation
    sim.complete_components();
    
    // Display final results
    println!("\n🎯 Final Results After 1 Billion Years:");
    println!("========================================");
    display_thermal_profile(&sim, &config);
    
    let final_stats = sim.get_stats();
    println!("\n📊 Simulation Statistics:");
    println!("   • Total cells processed: {}", final_stats.total_cells);
    println!("   • Total steps completed: {}", final_stats.current_step);
    println!("   • Total time simulated: {:.1} billion years",
             final_stats.years_simulated as f64 / 1_000_000_000.0);

    // Complete all components (MetricsReportingComponent will automatically generate performance report)
    sim.complete_components();

    println!("\n✅ Billion Year Geological Simulation Complete!");
    println!("   The RadianceComponent and LayerCellComponent successfully modeled");
    println!("   thermal evolution over geological timescales using realistic physics.");
}

/// Display thermal profile of the geological layers
fn display_thermal_profile(sim: &Simulation, config: &SimulationConfig) {
    println!("   Thermal Profile by Layer:");
    
    // Get sample cells from each layer to show temperature distribution
    let cells = sim.get_geological_cells();
    
    for (layer_idx, layer_config) in config.layers.iter().enumerate() {
        let mut layer_temps = Vec::new();
        let mut layer_pressures = Vec::new();
        
        // Collect temperatures from this layer
        for entry in cells.iter() {
            let (location, cell_data) = (entry.key(), entry.value());
            if location.layer_set_index() == layer_idx {
                layer_temps.push(cell_data.temperature_k);
                layer_pressures.push(cell_data.pressure_pa / 1e9); // Convert to GPa
            }
        }
        
        if !layer_temps.is_empty() {
            let avg_temp = layer_temps.iter().sum::<f64>() / layer_temps.len() as f64;
            let min_temp = layer_temps.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_temp = layer_temps.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            let avg_pressure = layer_pressures.iter().sum::<f64>() / layer_pressures.len() as f64;
            
            println!("     • {}: {:.0}°C avg ({:.0}-{:.0}°C range), {:.1} GPa pressure", 
                     layer_config.name,
                     avg_temp - 273.15,
                     min_temp - 273.15, 
                     max_temp - 273.15,
                     avg_pressure);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_billion_year_config() {
        let planet_config = PlanetConfig {
            radius_km: 6371.0,
            surface_gravity_m_s_s: 9.81,
            surface_temperature_k: 288.15,
        };
        
        let layers = vec![
            LayerConfig {
                height_per_step_km: 5.0,
                depth_steps: 4,
                resolution: Resolution::Four,
                name: "Test Crust".to_string(),
                temperature_gradient_k_per_km: 25.0,
            },
        ];
        
        let config = SimulationConfig {
            planet: planet_config,
            layers,
            years_per_step: 1_000_000,
            steps: 1000,
        };
        
        // Verify billion-year timescale
        let total_years = config.steps as u64 * config.years_per_step as u64;
        assert_eq!(total_years, 1_000_000_000); // 1 billion years
        
        // Verify realistic planetary parameters
        assert_eq!(config.planet.radius_km, 6371.0); // Earth radius
        assert_eq!(config.planet.surface_gravity_m_s_s, 9.81); // Earth gravity
    }
    
    #[test]
    fn test_component_integration() {
        // Test that components can be created and configured
        let layer_component = LayerCellComponent::new();
        let radiance_component = RadianceComponent::new().with_emissivity(0.95);
        
        assert_eq!(radiance_component.default_emissivity, 0.95);
    }
}
