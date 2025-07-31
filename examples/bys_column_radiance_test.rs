use atmo_biosphere_rust::components::{LayerCellComponent, ColumnRadianceComponent, MetricsReportingComponent};
use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, LayerConfig, PlanetConfig};
use h3o::Resolution;

fn main() {
    println!("🌍 BILLION YEAR SIMULATION (BYS) - Column-Based Radiance Test");
    println!("   • Resolution: 2 (coarse for performance testing)");
    println!("   • Time scale: 1 billion years");
    println!("   • Optimization: Column-based vertical radiance + lateral averaging");
    println!("   • Target: <30 seconds total simulation time");
    println!();

    // Create Earth-like planetary configuration for BYS
    let planet_config = PlanetConfig {
        radius_km: 6371.0,                    // Earth radius
        surface_gravity_m_s_s: 9.81,          // Earth gravity
        surface_temperature_k: 288.15,        // 15°C surface temperature
    };
    
    // BYS Configuration: 1 billion years with geological time steps
    let config = SimulationConfig {
        steps: 1000,                          // 1000 steps for billion years
        years_per_step: 1_000_000,           // 1 million years per step
        planet: planet_config,
        layers: vec![
            LayerConfig {
                name: "Continental Crust".to_string(),
                resolution: Resolution::Two,  // Coarse resolution for performance
                depth_steps: 4,               // 4 steps = 40km crust
                height_per_step_km: 10.0,
                temperature_gradient_k_per_km: 25.0,
            },
            LayerConfig {
                name: "Upper Mantle".to_string(),
                resolution: Resolution::Two,  // Same resolution for consistency
                depth_steps: 4,               // 6 steps = 150km upper mantle
                height_per_step_km: 25.0,
                temperature_gradient_k_per_km: 15.0,
            },
            LayerConfig {
                name: "Lower Mantle".to_string(),
                resolution: Resolution::Two,  // Coarse for deep layers
                depth_steps: 4,               // 8 steps = 200km lower mantle
                height_per_step_km: 33.0,
                temperature_gradient_k_per_km: 10.0,
            },
            LayerConfig {
                name: "Asthenosphere".to_string(),
                resolution: Resolution::Two,  // Very coarse for deepest layer
                depth_steps: 3,               // 4 steps = 100km asthenosphere
                height_per_step_km: 50.0,
                temperature_gradient_k_per_km: 5.0,
            },
        ],
    };

    println!("📊 BYS Configuration:");
    println!("   • Total time: {} million years", config.steps * config.years_per_step / 1_000_000);
    println!("   • Time step: {} million years", config.years_per_step / 1_000_000);
    println!("   • Total steps: {}", config.steps);
    println!("   • Total depth: {}km", 
             config.layers.iter().map(|l| l.depth_steps as f64 * l.height_per_step_km).sum::<f64>());
    
    for (i, layer) in config.layers.iter().enumerate() {
        let layer_depth = layer.depth_steps as f64 * layer.height_per_step_km;
        println!("   Layer {}: {} ({}km, {} cells)", 
                 i, layer.name, layer_depth, layer.depth_steps);
    }
    println!();

    // Create simulation
    let mut sim = Simulation::new(config.clone());

    // Add optimized components for BYS
    println!("🔧 Adding BYS-Optimized Components:");
    
    // Add LayerCellComponent for geological cell creation
    println!("   • Adding LayerCellComponent (geological cell initialization)...");
    sim.add_component(Box::new(LayerCellComponent::new()));
    
    // Add ColumnRadianceComponent for optimized radiance
    println!("   • Adding ColumnRadianceComponent (column-based optimization)...");
    sim.add_component(Box::new(ColumnRadianceComponent::with_emissivity(0.95)));
    
    // Add MetricsReportingComponent for performance analysis
    println!("   • Adding MetricsReportingComponent (BYS performance tracking)...");
    sim.add_component(Box::new(MetricsReportingComponent::new()));
    
    println!();

    // Initialize simulation
    println!("🔧 Initializing BYS simulation...");
    sim.initialize_cells();

    // Get cell count for performance projections
    let cell_count = sim.get_geological_cells().len();
    println!("   • Total cells created: {}", cell_count);
    println!("   • Average cells per layer: {:.0}", cell_count as f64 / config.layers.len() as f64);
    println!();

    // Run BYS simulation
    println!("🚀 Running Billion Year Simulation...");
    println!("    Target: <30 seconds for 1000 steps (1 billion years)");
    println!("    Expected: ~{}s based on column optimization", 
             (cell_count as f64 / 29000.0) * 1.98 * (config.steps as f64 / 10.0));
    println!();

    let start_time = std::time::Instant::now();
    
    // Run first 10 steps to estimate performance
    println!("🔬 Running performance sample (first 10 steps)...");
    let sample_start = std::time::Instant::now();
    
    for step in 0..10 {
        sim.step();
        if step % 2 == 0 {
            println!("   Step {}/10 completed ({} million years)", 
                     step + 1, (step + 1) * config.years_per_step / 1_000_000);
        }
    }
    
    let sample_duration = sample_start.elapsed();
    let avg_step_time = sample_duration.as_secs_f64() / 10.0;
    let projected_total_time = avg_step_time * config.steps as f64;
    
    println!();
    println!("📊 Performance Sample Results:");
    println!("   • 10 steps completed in {:.2}s", sample_duration.as_secs_f64());
    println!("   • Average step time: {:.3}s", avg_step_time);
    println!("   • Projected total time: {:.1}s ({:.1} minutes)", 
             projected_total_time, projected_total_time / 60.0);
    
    println!("   🚀 RUNNING FULL BILLION YEAR SIMULATION!");
    println!("      Testing if performance improves over longer cycles...");

    // Run remaining steps with detailed timing analysis
    println!();
    println!("🚀 Running remaining {} steps...", config.steps - 10);

    let mut step_times = Vec::new();
    let checkpoint_interval = 100usize;

    for step in 10..config.steps {
        let step_start = std::time::Instant::now();
        sim.step();
        let step_duration = step_start.elapsed().as_secs_f64();
        step_times.push(step_duration);

        if step % checkpoint_interval as u32 == 0 {
            let progress = (step as f64 / config.steps as f64) * 100.0;
            let recent_avg = step_times.iter().rev().take(checkpoint_interval).sum::<f64>() / checkpoint_interval as f64;
            let overall_avg = step_times.iter().sum::<f64>() / step_times.len() as f64;

            println!("   Progress: {:.1}% ({} million years) | Recent avg: {:.3}s | Overall avg: {:.3}s",
                     progress, step * config.years_per_step / 1_000_000, recent_avg, overall_avg);

            // Check if performance is improving
            if step_times.len() >= checkpoint_interval * 2 {
                let early_avg = step_times.iter().take(checkpoint_interval).sum::<f64>() / checkpoint_interval as f64;
                let improvement = (early_avg - recent_avg) / early_avg * 100.0;
                if improvement > 5.0 {
                    println!("      📈 Performance improving! {:.1}% faster than early steps", improvement);
                } else if improvement < -5.0 {
                    println!("      📉 Performance degrading: {:.1}% slower than early steps", improvement.abs());
                }
            }
        }
    }

    let total_duration = start_time.elapsed();

    println!();
    println!("⏱️  BYS Simulation Results:");
    println!("   • Total time: {:.2}s ({:.1} minutes)",
             total_duration.as_secs_f64(), total_duration.as_secs_f64() / 60.0);
    println!("   • Steps completed: {}", sim.current_step);
    println!("   • Years simulated: {} million",
             sim.current_step * config.years_per_step / 1_000_000);

    if sim.current_step >= config.steps {
        println!("   🎉 FULL BILLION YEAR SIMULATION COMPLETE!");

        // Analyze performance trends over the simulation
        if step_times.len() > 200 {
            let early_steps = &step_times[0..100];
            let middle_steps = &step_times[step_times.len()/2-50..step_times.len()/2+50];
            let late_steps = &step_times[step_times.len()-100..];

            let early_avg = early_steps.iter().sum::<f64>() / early_steps.len() as f64;
            let middle_avg = middle_steps.iter().sum::<f64>() / middle_steps.len() as f64;
            let late_avg = late_steps.iter().sum::<f64>() / late_steps.len() as f64;

            println!();
            println!("📊 Performance Trend Analysis:");
            println!("   • Early steps (1-100): {:.3}s avg", early_avg);
            println!("   • Middle steps: {:.3}s avg", middle_avg);
            println!("   • Late steps (last 100): {:.3}s avg", late_avg);

            let early_to_late_change = (late_avg - early_avg) / early_avg * 100.0;
            if early_to_late_change < -5.0 {
                println!("   🚀 PERFORMANCE IMPROVED: {:.1}% faster over time!", early_to_late_change.abs());
            } else if early_to_late_change > 5.0 {
                println!("   ⚠️  Performance degraded: {:.1}% slower over time", early_to_late_change);
            } else {
                println!("   ✅ Performance stable: {:.1}% change over time", early_to_late_change);
            }
        }
    }

    println!();
    println!("🎯 BYS Performance Analysis:");
    println!("   • Cell count: {}", cell_count);
    println!("   • Resolution: {} (coarse)", config.layers[0].resolution as u8);
    println!("   • Column-based optimization: ENABLED");
    
    if total_duration.as_secs_f64() < 30.0 {
        println!("   🏆 TARGET ACHIEVED: Under 30 seconds!");
    } else if total_duration.as_secs_f64() < 120.0 {
        println!("   ✅ GOOD: Under 2 minutes");
    } else {
        println!("   ⚠️  NEEDS OPTIMIZATION: Over 2 minutes");
    }

    println!();
    println!("🌍 Billion Year Simulation Complete!");
    println!("   Column-based radiance optimization enables geological");
    println!("   simulations at billion-year time scales with realistic");
    println!("   performance for interactive geological modeling!");
}
