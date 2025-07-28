use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
use h3o::Resolution;

fn main() {
    println!("🌍 Basic Geological Simulation Test");
    
    // Create minimal simulation configuration
    let config = SimulationConfig {
        planet: PlanetConfig {
            radius_km: 6371.0,           // Earth radius
            surface_gravity_m_s_s: 9.81, // Earth gravity
        },
        years_per_step: 1000,            // 1000 years per step
        steps: 10,                       // 10 steps = 10,000 years
        layers: vec![
            LayerConfig {
                height_per_step_km: 10.0,
                depth_steps: 2,  // 2 steps = 20km crust
                resolution: Resolution::Five,
                name: "Crust".to_string(),
            },
            LayerConfig {
                height_per_step_km: 50.0,
                depth_steps: 6,  // 6 steps = 300km upper mantle
                resolution: Resolution::Four,
                name: "Upper Mantle".to_string(),
            },
        ],
    };
    
    println!("📋 Configuration:");
    println!("  Planet radius: {} km", config.planet.radius_km);
    println!("  Surface gravity: {} m/s²", config.planet.surface_gravity_m_s_s);
    println!("  Years per step: {}", config.years_per_step);
    println!("  Total steps: {}", config.steps);
    println!("  Layers: {}", config.layers.len());
    
    for (i, layer) in config.layers.iter().enumerate() {
        println!("    Layer {}: {} ({} steps × {}km = {}km total, res {:?})",
                 i, layer.name, layer.depth_steps, layer.height_per_step_km,
                 layer.depth_steps as f64 * layer.height_per_step_km, layer.resolution);
    }
    
    // Create simulation
    let mut sim = Simulation::new(config);
    println!("\n✅ Simulation created");
    
    // Initialize cells
    println!("🔧 Initializing cells...");
    sim.initialize_cells();
    
    let stats = sim.get_stats();
    println!("✅ Initialized {} cells", stats.total_cells);
    
    // Show some cell data
    let cells = sim.get_geological_cells();
    println!("\n📊 Sample cell data:");
    let mut count = 0;
    for entry in cells.iter() {
        if count >= 3 { break; } // Show first 3 cells
        
        let (location, data) = (entry.key(), entry.value());
        println!("  Cell {}: Layer[{}] H3[{}] Depth[{}]", 
                 count + 1,
                 location.layer_set_index(),
                 location.h3_cell_index(),
                 location.depth_index());
        println!("    Temperature: {:.1}K, Pressure: {:.0}Pa, Density: {:.0}kg/m³",
                 data.temperature_k, data.pressure_pa, data.density_kg_m3);
        println!("    Energy: {:.0}J, Mass: {:.0}kg",
                 data.energy_mass.energy_joules(), data.energy_mass.mass_kg());
        
        count += 1;
    }
    
    // Run a few simulation steps
    println!("\n🚀 Running simulation...");
    for _ in 0..3 {
        sim.step();
        let current_stats = sim.get_stats();
        println!("  Step {}: {} years simulated", 
                 current_stats.current_step, 
                 current_stats.years_simulated);
    }
    
    let final_stats = sim.get_stats();
    println!("\n📈 Final Statistics:");
    println!("  Steps completed: {}/{}", final_stats.current_step, final_stats.total_steps);
    println!("  Years simulated: {}", final_stats.years_simulated);
    println!("  Total cells: {}", final_stats.total_cells);
    
    println!("\n🎉 Basic simulation test completed!");
}
