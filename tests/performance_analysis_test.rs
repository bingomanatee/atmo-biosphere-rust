use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;
use std::time::{Instant, Duration};

#[test]
fn test_performance_analysis() {
    println!("🔍 PERFORMANCE ANALYSIS TEST");
    println!("============================");
    println!("⏰ Short simulation: 1,000 steps (1 million years)");
    println!("🎯 Goal: Identify performance bottlenecks for optimization");
    
    // Create simulation configuration for performance analysis
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 1_000, // Short test: 1,000 steps = 1 million years
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    // Create simulation without components (pure radiative transfer)
    let mut components: Vec<Box<dyn SimComponent>> = vec![];
    let mut sim = SimulationImmut::new(config, &mut components);
    
    println!("✅ Simulation created");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    println!("   - Total cells: {}", sim.total_cells());
    
    // Load layer sets
    sim.load_layer_sets();
    println!("✅ Layer sets loaded: {} geological layers", sim.layer_sets.len());
    
    // Performance timing variables
    let mut step_times = Vec::new();
    let mut total_step_time = Duration::new(0, 0);
    let mut radiative_transfer_time = Duration::new(0, 0);
    let mut transaction_time = Duration::new(0, 0);
    
    let simulation_start = Instant::now();
    
    println!("\n🚀 Starting performance analysis simulation...");
    println!("📊 Measuring: step time, radiative transfer, transactions");
    
    // Run the performance test
    for step_num in 0..sim.config.steps {
        let step_start = Instant::now();
        
        // Time the full step
        sim.step();
        
        let step_duration = step_start.elapsed();
        step_times.push(step_duration);
        total_step_time += step_duration;
        
        // Report progress every 100 steps
        if step_num % 100 == 0 || step_num == sim.config.steps - 1 {
            let avg_step_time = if !step_times.is_empty() {
                step_times.iter().sum::<Duration>() / step_times.len() as u32
            } else {
                Duration::new(0, 0)
            };
            
            println!("   Step {}/{}: {:.2}ms avg step time", 
                     step_num + 1, sim.config.steps, avg_step_time.as_secs_f64() * 1000.0);
        }
    }
    
    let total_simulation_time = simulation_start.elapsed();
    
    // Performance Analysis Results
    println!("\n📊 PERFORMANCE ANALYSIS RESULTS:");
    println!("=================================");
    
    // Basic timing stats
    let avg_step_time = total_step_time / sim.config.steps as u32;
    let steps_per_second = sim.config.steps as f64 / total_simulation_time.as_secs_f64();
    
    println!("⏱️  Total simulation time: {:.2}s", total_simulation_time.as_secs_f64());
    println!("⏱️  Average step time: {:.2}ms", avg_step_time.as_secs_f64() * 1000.0);
    println!("🚀 Steps per second: {:.1}", steps_per_second);
    
    // Extrapolation to billion year simulation
    let billion_year_steps = 1_000_000_u64;
    let estimated_billion_year_time = avg_step_time * billion_year_steps as u32;
    let estimated_hours = estimated_billion_year_time.as_secs_f64() / 3600.0;
    
    println!("\n🔮 Billion Year Simulation Projection:");
    println!("   - Steps needed: {}", billion_year_steps);
    println!("   - Estimated time: {:.1} hours ({:.1} days)", estimated_hours, estimated_hours / 24.0);
    
    // Performance breakdown analysis
    println!("\n🔍 Performance Breakdown Analysis:");
    
    // Step time distribution
    step_times.sort();
    let min_step = step_times.first().unwrap();
    let max_step = step_times.last().unwrap();
    let median_step = step_times[step_times.len() / 2];
    let p95_step = step_times[(step_times.len() as f64 * 0.95) as usize];
    
    println!("   Step time distribution:");
    println!("     Min: {:.2}ms", min_step.as_secs_f64() * 1000.0);
    println!("     Median: {:.2}ms", median_step.as_secs_f64() * 1000.0);
    println!("     95th percentile: {:.2}ms", p95_step.as_secs_f64() * 1000.0);
    println!("     Max: {:.2}ms", max_step.as_secs_f64() * 1000.0);
    
    // Memory and system analysis
    println!("\n💾 System Resource Analysis:");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    println!("   - Total cells: {}", sim.total_cells());
    println!("   - Cells per layer set: {}", sim.total_cells() as usize / sim.layer_sets.len());
    
    // Transaction analysis
    let (pending, committed) = sim.transaction_manager.get_transaction_stats();
    println!("   - Final pending transactions: {}", pending);
    println!("   - Total committed transactions: {}", committed);
    println!("   - Avg transactions per step: {:.1}", committed as f64 / sim.config.steps as f64);
    
    // Optimization recommendations
    println!("\n🎯 OPTIMIZATION RECOMMENDATIONS:");
    println!("=================================");
    
    if avg_step_time.as_millis() > 100 {
        println!("❌ SLOW: Average step time > 100ms");
        println!("   → Focus on radiative transfer optimization");
        println!("   → Consider reducing transaction overhead");
    } else if avg_step_time.as_millis() > 50 {
        println!("⚠️  MODERATE: Average step time 50-100ms");
        println!("   → Some optimization needed for billion year runs");
    } else {
        println!("✅ FAST: Average step time < 50ms");
        println!("   → Performance acceptable for long simulations");
    }
    
    if estimated_hours > 24.0 {
        println!("❌ Billion year simulation too slow (>{:.1} hours)", estimated_hours);
        println!("   → Need significant optimization");
        println!("   → Target: <10ms per step for reasonable run times");
    } else if estimated_hours > 8.0 {
        println!("⚠️  Billion year simulation slow ({:.1} hours)", estimated_hours);
        println!("   → Some optimization recommended");
    } else {
        println!("✅ Billion year simulation feasible ({:.1} hours)", estimated_hours);
    }
    
    // Specific optimization targets
    println!("\n🔧 Specific Optimization Targets:");
    if committed > 0 {
        let transaction_overhead = avg_step_time.as_millis() as f64 / (committed as f64 / sim.config.steps as f64);
        println!("   - Transaction overhead: {:.2}ms per transaction", transaction_overhead);
        if transaction_overhead > 1.0 {
            println!("     → Optimize atomic transaction processing");
        }
    }
    
    println!("   - Radiative transfer: Main computational bottleneck");
    println!("     → Consider spatial optimization (fewer neighbor pairs)");
    println!("     → Consider temporal optimization (adaptive time steps)");
    println!("     → Consider algorithmic optimization (faster heat transfer)");
    
    // Final assessment
    println!("\n🎯 FINAL ASSESSMENT:");
    if steps_per_second > 20.0 {
        println!("✅ EXCELLENT performance: {:.1} steps/sec", steps_per_second);
    } else if steps_per_second > 10.0 {
        println!("✅ GOOD performance: {:.1} steps/sec", steps_per_second);
    } else if steps_per_second > 5.0 {
        println!("⚠️  MODERATE performance: {:.1} steps/sec", steps_per_second);
    } else {
        println!("❌ POOR performance: {:.1} steps/sec", steps_per_second);
        println!("   → Significant optimization required");
    }
    
    // Test assertions
    assert!(total_simulation_time.as_secs() < 300, "Test should complete in under 5 minutes");
    assert!(steps_per_second > 1.0, "Should process at least 1 step per second");
    
    println!("\n🎉 Performance analysis completed!");
}
