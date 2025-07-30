use atmo_biosphere_rust::components::{LayerCellComponent, ParallelRadianceComponent, MetricsReportingComponent};
use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, LayerConfig, PlanetConfig};
use h3o::Resolution;

fn main() {
    println!("🚀 Testing Parallel Radiance Component (No Binary Pairs!)");
    println!("   This approach eliminates pre-computed binary pairs entirely");
    println!("   and processes cells in parallel chunks with cached neighbors.");
    println!();

    // Create Earth-like planetary configuration
    let planet_config = PlanetConfig {
        radius_km: 6371.0,                    // Earth radius
        surface_gravity_m_s_s: 9.81,          // Earth gravity
        surface_temperature_k: 288.15,        // 15°C surface temperature
    };

    // Create a smaller simulation for testing
    let config = SimulationConfig {
        steps: 10,
        years_per_step: 1000,
        planet: planet_config,
        layers: vec![
            LayerConfig {
                name: "Test Crust".to_string(),
                resolution: Resolution::Two,  // Much smaller for testing
                depth_steps: 3,
                height_per_step_km: 10.0,
                temperature_gradient_k_per_km: 25.0,
            },
            LayerConfig {
                name: "Test Mantle".to_string(),
                resolution: Resolution::Two,
                depth_steps: 2,
                height_per_step_km: 20.0,
                temperature_gradient_k_per_km: 15.0,
            },
        ],
    };

    println!("📊 Test Configuration:");
    for (i, layer) in config.layers.iter().enumerate() {
        println!("   Layer {}: {} (Resolution {}, {} depth steps)", 
                 i, layer.name, layer.resolution as u8, layer.depth_steps);
    }
    println!();

    // Create simulation
    let mut sim = Simulation::new(config.clone());

    // Add components
    println!("🔧 Adding Components:");
    
    // Add LayerCellComponent to create geological cells
    println!("   • Adding LayerCellComponent...");
    sim.add_component(Box::new(LayerCellComponent::new()));
    
    // Add ParallelRadianceComponent (NO BinaryPairComponent needed!)
    println!("   • Adding ParallelRadianceComponent (no binary pairs!)...");
    sim.add_component(Box::new(ParallelRadianceComponent::new()));
    
    // Add MetricsReportingComponent for performance tracking
    println!("   • Adding MetricsReportingComponent for performance tracking...");
    sim.add_component(Box::new(MetricsReportingComponent::new()));
    
    println!();

    // Initialize simulation
    println!("🔧 Initializing simulation...");
    sim.initialize_cells();

    // Run simulation
    println!();
    println!("🚀 Running Parallel Radiance Test...");
    println!("    (This should be much faster than the binary pairs approach)");
    println!();

    let start_time = std::time::Instant::now();
    sim.run();
    let duration = start_time.elapsed();

    println!();
    println!("⏱️  Simulation completed in {:.2} seconds", duration.as_secs_f64());

    println!();
    println!("✅ Parallel Radiance Test Complete!");
    println!("   This approach demonstrates how we can eliminate the need for");
    println!("   pre-computing millions of binary pairs by using parallel cell");
    println!("   processing with cached neighbor relationships.");
}
