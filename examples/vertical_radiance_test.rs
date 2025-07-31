use atmo_biosphere_rust::components::{LayerCellComponent, VerticalRadianceComponent, MetricsReportingComponent};
use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, LayerConfig, PlanetConfig};
use h3o::Resolution;

fn main() {
    println!("🚀 Testing Vertical Radiance + Horizontal Blending Optimization");
    println!("   • Vertical radiance: Full Stefan-Boltzmann physics");
    println!("   • Horizontal blending: Simple energy smoothing in simulation");
    println!("   • Expected: 3-5x faster than full radiance approach");
    println!();

    // Create Earth-like planetary configuration
    let planet_config = PlanetConfig {
        radius_km: 6371.0,                    // Earth radius
        surface_gravity_m_s_s: 9.81,          // Earth gravity
        surface_temperature_k: 288.15,        // 15°C surface temperature
    };
    
    // Create test simulation configuration
    let config = SimulationConfig {
        steps: 10,
        years_per_step: 1000,
        planet: planet_config,
        layers: vec![
            LayerConfig {
                name: "Test Crust".to_string(),
                resolution: Resolution::Two,  // Same as parallel test for comparison
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
    
    // Add LayerCellComponent to create geological cells with up/down IDs
    println!("   • Adding LayerCellComponent (with vertical neighbor IDs)...");
    sim.add_component(Box::new(LayerCellComponent::new()));
    
    // Add VerticalRadianceComponent (NO binary pairs needed!)
    println!("   • Adding VerticalRadianceComponent (vertical-only radiance)...");
    sim.add_component(Box::new(VerticalRadianceComponent::with_emissivity(0.95)));
    
    // Add MetricsReportingComponent for performance tracking
    println!("   • Adding MetricsReportingComponent for performance tracking...");
    sim.add_component(Box::new(MetricsReportingComponent::new()));
    
    println!();

    // Initialize simulation
    println!("🔧 Initializing simulation...");
    sim.initialize_cells();

    // Run simulation
    println!();
    println!("🚀 Running Vertical Radiance + Horizontal Blending Test...");
    println!("    Expected performance: 2-3 seconds (vs 6.45s for full radiance)");
    println!();

    let start_time = std::time::Instant::now();
    sim.run();
    let duration = start_time.elapsed();

    println!();
    println!("⏱️  Simulation completed in {:.2} seconds", duration.as_secs_f64());
    
    // Calculate performance improvement
    let baseline_time = 6.45; // From parallel radiance test
    let improvement = baseline_time / duration.as_secs_f64();
    
    println!();
    println!("📈 Performance Analysis:");
    println!("   • Baseline (full radiance): {:.2}s", baseline_time);
    println!("   • Optimized (vertical + blending): {:.2}s", duration.as_secs_f64());
    println!("   • Performance improvement: {:.1}x faster", improvement);
    
    if improvement > 2.0 {
        println!("   🎉 Excellent! Achieved target 2x+ speedup");
    } else if improvement > 1.5 {
        println!("   ✅ Good! Significant speedup achieved");
    } else {
        println!("   ⚠️  Speedup less than expected, may need further optimization");
    }

    println!();
    println!("✅ Vertical Radiance + Horizontal Blending Test Complete!");
    println!("   This approach demonstrates how we can achieve significant");
    println!("   performance improvements by focusing physics calculations");
    println!("   on the most important energy transfers (vertical) while");
    println!("   using simple approximations for less critical transfers (horizontal).");
}
