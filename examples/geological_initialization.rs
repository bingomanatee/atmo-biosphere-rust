use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
use atmo_biosphere_rust::components::{LayerCellComponent, ThermalComponent};
use h3o::Resolution;

fn main() {
    println!("🌍 Geological Cell Initialization Example");
    
    // Create realistic geological simulation
    let config = SimulationConfig {
        planet: PlanetConfig {
            radius_km: 6371.0,           // Earth radius
            surface_gravity_m_s_s: 9.81, // Earth gravity
        },
        years_per_step: 10000,           // 10,000 years per step
        steps: 5,                        // 50,000 years total
        layers: vec![
            LayerConfig {
                height_per_step_km: 5.0,  // 5km per depth step
                depth_steps: 4,           // 4 steps = 20km crust
                resolution: Resolution::Three, // Coarse for testing
                name: "Continental Crust".to_string(),
            },
            LayerConfig {
                height_per_step_km: 25.0, // 25km per depth step
                depth_steps: 8,           // 8 steps = 200km upper mantle
                resolution: Resolution::Two, // Very coarse for testing
                name: "Upper Mantle".to_string(),
            },
            LayerConfig {
                height_per_step_km: 50.0, // 50km per depth step
                depth_steps: 4,           // 4 steps = 200km lower mantle
                resolution: Resolution::One, // Extremely coarse for testing
                name: "Lower Mantle".to_string(),
            },
        ],
    };
    
    let mut sim = Simulation::new(config);
    sim.initialize_cells();
    
    println!("✅ Simulation initialized with {} cells", sim.get_geological_cells().len());
    
    // Show initial state (before geological initialization)
    println!("\n📊 Initial state (before geological initialization):");
    let mut count = 0;
    for entry in sim.get_geological_cells().iter() {
        if count >= 5 { break; }
        let (location, data) = (entry.key(), entry.value());
        println!("  Cell {}: Layer[{}] Depth[{}] Temp[{:.1}K] Pressure[{:.0}Pa] Density[{:.0}kg/m³]",
                 count + 1, 
                 location.layer_set_index(), 
                 location.depth_index(),
                 data.temperature_k, 
                 data.pressure_pa, 
                 data.density_kg_m3);
        count += 1;
    }
    
    // Add geological initialization component
    println!("\n🔧 Adding LayerCellComponent for geological initialization...");
    sim.add_component(Box::new(LayerCellComponent::with_surface_temperature(288.15))); // 15°C surface
    
    // Add thermal component for ongoing thermal processes
    sim.add_component(Box::new(ThermalComponent::new()));
    
    println!("✅ Added {} components", sim.components.len());
    
    // Run simulation - LayerCellComponent will initialize on first step
    println!("\n🚀 Running simulation with geological initialization...");
    sim.run();
    
    // Show final state (after geological initialization and thermal processing)
    println!("\n📈 Final state (after geological initialization):");
    let mut count = 0;
    let mut temp_sum = 0.0;
    let mut pressure_sum = 0.0;
    let mut density_sum = 0.0;
    
    for entry in sim.get_geological_cells().iter() {
        let (location, data) = (entry.key(), entry.value());
        
        if count < 5 {
            println!("  Cell {}: Layer[{}] Depth[{}] Temp[{:.1}K] Pressure[{:.1}MPa] Density[{:.0}kg/m³]",
                     count + 1, 
                     location.layer_set_index(), 
                     location.depth_index(),
                     data.temperature_k, 
                     data.pressure_pa / 1_000_000.0, // Convert to MPa
                     data.density_kg_m3);
        }
        
        temp_sum += data.temperature_k;
        pressure_sum += data.pressure_pa;
        density_sum += data.density_kg_m3;
        count += 1;
        
        if count >= 1000 { break; } // Sample first 1000 cells for averages
    }
    
    // Show geological statistics
    println!("\n🌍 Geological Statistics (sample of {} cells):", count);
    println!("  Average temperature: {:.1}K ({:.1}°C)", 
             temp_sum / count as f64, (temp_sum / count as f64) - 273.15);
    println!("  Average pressure: {:.1} MPa", 
             (pressure_sum / count as f64) / 1_000_000.0);
    println!("  Average density: {:.0} kg/m³", 
             density_sum / count as f64);
    
    // Show depth-dependent properties
    println!("\n📏 Depth-dependent properties:");
    let mut layer_stats = std::collections::HashMap::new();
    
    for entry in sim.get_geological_cells().iter() {
        let (location, data) = (entry.key(), entry.value());
        let layer = location.layer_set_index();
        
        let stats = layer_stats.entry(layer).or_insert((0, 0.0, 0.0, 0.0));
        stats.0 += 1;
        stats.1 += data.temperature_k;
        stats.2 += data.pressure_pa;
        stats.3 += data.density_kg_m3;
    }
    
    for (layer, (count, temp_sum, pressure_sum, density_sum)) in layer_stats {
        let layer_name = match layer {
            0 => "Continental Crust",
            1 => "Upper Mantle",
            2 => "Lower Mantle",
            _ => "Unknown",
        };
        
        println!("  Layer {}: {} ({} cells)", layer, layer_name, count);
        println!("    Avg Temp: {:.1}K ({:.1}°C)", 
                 temp_sum / count as f64, (temp_sum / count as f64) - 273.15);
        println!("    Avg Pressure: {:.1} MPa", 
                 (pressure_sum / count as f64) / 1_000_000.0);
        println!("    Avg Density: {:.0} kg/m³", 
                 density_sum / count as f64);
    }
    
    let final_stats = sim.get_stats();
    println!("\n📊 Final Simulation Statistics:");
    println!("  Steps completed: {}/{}", final_stats.current_step, final_stats.total_steps);
    println!("  Years simulated: {}", final_stats.years_simulated);
    println!("  Total cells: {}", final_stats.total_cells);
    println!("  Components: {}", sim.components.len());
    
    println!("\n🎉 Geological initialization completed!");
    println!("✅ LayerCellComponent initialized realistic geological properties");
    println!("✅ Temperature gradients: 25K/km (crust), 15K/km (upper mantle), 10K/km (lower mantle)");
    println!("✅ Pressure gradients: ~27 MPa/km depth");
    println!("✅ Material-based density calculations");
    println!("✅ Energy calculated from mass, temperature, and specific heat capacity");
}
