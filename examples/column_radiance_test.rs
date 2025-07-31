use atmo_biosphere_rust::components::{LayerCellComponent, ColumnRadianceComponent, MetricsReportingComponent};
use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, LayerConfig, PlanetConfig};
use h3o::Resolution;

fn main() {
    println!("🏛️ Testing Column-Based Radiance Optimization");
    println!("   • Column processing: Group cells by H3 index for vertical operations");
    println!("   • Cache optimization: Sequential memory access patterns");
    println!("   • Expected: 30%+ faster than individual cell processing");
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
                resolution: Resolution::Two,  // Same as other tests for comparison
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
    
    // Add ColumnRadianceComponent (column-based optimization!)
    println!("   • Adding ColumnRadianceComponent (column-based optimization)...");
    sim.add_component(Box::new(ColumnRadianceComponent::with_emissivity(0.95)));
    
    // Add MetricsReportingComponent for performance tracking
    println!("   • Adding MetricsReportingComponent for performance tracking...");
    sim.add_component(Box::new(MetricsReportingComponent::new()));
    
    println!();

    // Initialize simulation
    println!("🔧 Initializing simulation...");
    sim.initialize_cells();

    // Run simulation
    println!();
    println!("🚀 Running Column-Based Radiance Test...");
    println!("    Expected performance: <2 seconds (vs 2.51s individual, 6.45s full radiance)");
    println!();

    let start_time = std::time::Instant::now();
    sim.run();
    let duration = start_time.elapsed();

    println!();
    println!("⏱️  Simulation completed in {:.2} seconds", duration.as_secs_f64());
    
    // Calculate performance improvements
    let individual_time = 2.51; // From vertical radiance test
    let baseline_time = 6.45;   // From parallel radiance test
    let improvement_vs_individual = individual_time / duration.as_secs_f64();
    let improvement_vs_baseline = baseline_time / duration.as_secs_f64();
    
    println!();
    println!("📈 Performance Analysis:");
    println!("   • Baseline (full radiance): {:.2}s", baseline_time);
    println!("   • Individual vertical processing: {:.2}s", individual_time);
    println!("   • Column-based processing: {:.2}s", duration.as_secs_f64());
    println!();
    println!("📊 Performance Improvements:");
    println!("   • vs Full radiance: {:.1}x faster", improvement_vs_baseline);
    println!("   • vs Individual processing: {:.1}x faster", improvement_vs_individual);
    
    if improvement_vs_individual > 1.3 {
        println!("   🎉 Excellent! Column optimization achieved 30%+ speedup");
    } else if improvement_vs_individual > 1.1 {
        println!("   ✅ Good! Significant column optimization speedup");
    } else {
        println!("   ⚠️  Column optimization less than expected");
    }

    println!();
    println!("🏛️ Column-Based Architecture Benefits:");
    println!("   • Cache Locality: Sequential access to vertical neighbors");
    println!("   • Reduced Lookups: One grouping pass vs individual HashMap queries");
    println!("   • Memory Efficiency: Better utilization of CPU cache lines");
    println!("   • Scalability: Linear scaling with geological depth");
    
    println!();
    println!("🌍 400km Depth Projection:");
    let cells_400km = 235_000; // Estimated for 8 layers
    let current_cells = 29_000; // Current test
    let scaling_factor = cells_400km as f64 / current_cells as f64;
    let projected_time = duration.as_secs_f64() * scaling_factor;
    
    println!("   • Current test: {} cells in {:.2}s", current_cells, duration.as_secs_f64());
    println!("   • 400km projection: {} cells in ~{:.0}s", cells_400km, projected_time);
    println!("   • Performance target: <30s for real-time geological simulation");
    
    if projected_time < 30.0 {
        println!("   🎯 Target achieved! Ready for 400km depth simulation");
    } else {
        println!("   ⚠️  May need additional optimization for 400km target");
    }

    println!();
    println!("✅ Column-Based Radiance Test Complete!");
    println!("   This optimization demonstrates the power of aligning");
    println!("   data structures with geological physics - vertical");
    println!("   columns are the natural unit of geological processing!");
}
