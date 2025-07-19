// Demonstration of reusable performance reporting during simulation

use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::thermal_gradient_config::ThermalGradientConfig;
use atmo_biosphere_rust::component::conduction_component::ConductionComponent;
use atmo_biosphere_rust::component::core_radiance_component::CoreRadianceComponent;
use atmo_biosphere_rust::component::convection_plume_component::ConvectionPlumeComponent;

fn main() {
    println!("📊 Reusable Performance Reporting Demo");
    println!("======================================");
    println!("Shows how to get performance data during simulation without ending it\n");

    // Create minimal simulation
    let thermal_config = ThermalGradientConfig {
        surface_temperature_kelvin: 288.0,
        thermal_gradient_per_km: 25.0,
        second_order_gradient_per_km2: -0.075,
    };

    let layer_params = vec![
        LayerSetParams {
            name: "Crust".to_string(),
            start_height_km: 0.0,
            end_height_km: -50.0,
            cells_per_column: 3,
            material_name: "granite".to_string(),
            surface_temperature_kelvin: 288.0,
            thermal_gradient_per_km: 25.0,
        },
        LayerSetParams {
            name: "Upper Mantle".to_string(),
            start_height_km: -50.0,
            end_height_km: -150.0,
            cells_per_column: 4,
            material_name: "peridotite".to_string(),
            surface_temperature_kelvin: 1500.0,
            thermal_gradient_per_km: 15.0,
        },
    ];

    let config = SimulationConfig {
        steps: 10,
        years_per_step: 5000.0,
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Create components
    let mut components: Vec<Box<dyn atmo_biosphere_rust::component::SimComponent>> = vec![
        Box::new(ConductionComponent::new()),
        Box::new(CoreRadianceComponent::new(140.0, 3.5e6)),
        Box::new(ConvectionPlumeComponent::with_seed(12345)),
    ];

    // Create simulation
    let mut sim = Simulation::new(config.clone(), &mut components);
    sim.initialize();

    println!("🚀 Running {} steps with reusable performance reporting...", config.steps);

    // Run simulation with periodic performance reporting
    for step in 0..config.steps {
        println!("\n🔄 Step {} (Year {})", step + 1, sim.current_year());
        
        let step_start = std::time::Instant::now();
        sim.step();
        let step_duration = step_start.elapsed();
        
        println!("   ✅ Completed in {:.2} ms", step_duration.as_secs_f64() * 1000.0);

        // Demonstrate reusable reporting every 3 steps
        if (step + 1) % 3 == 0 {
            println!("\n📊 INTERMEDIATE PERFORMANCE REPORT (Step {})", step + 1);
            println!("{}", "=".repeat(50));
            
            // 1. Lightweight summary (always reusable)
            let summary = sim.get_performance_summary();
            println!("{}", summary);
            
            // 2. Component-specific performance (reusable)
            println!("\n🔧 Component Details:");
            for component_name in ["thermal_conduction", "core_radiance", "convection_plumes"] {
                if let Some(component_report) = sim.get_component_performance(component_name) {
                    println!("{}", component_report);
                }
            }
            
            // 3. Full intermediate report (reusable - doesn't end simulation)
            if step + 1 == 6 {
                println!("\n📋 FULL INTERMEDIATE REPORT (Step 6):");
                println!("{}", "-".repeat(40));
                let intermediate_report = sim.generate_intermediate_report();
                println!("{}", intermediate_report);
                println!("✅ Simulation continues after intermediate report...");
            }
        }
        
        // Show that we can get performance data multiple times
        if step == 4 {
            println!("\n🔄 Multiple Performance Queries (Step 5):");
            
            // Query 1
            let summary1 = sim.get_performance_summary();
            println!("Query 1 - Summary:");
            println!("{}", summary1);
            
            // Query 2 (immediately after)
            let summary2 = sim.get_performance_summary();
            println!("Query 2 - Same Summary (reusable):");
            println!("{}", summary2);
            
            // Verify they're identical
            if summary1 == summary2 {
                println!("✅ Confirmed: Performance reporting is reusable!");
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("🏁 SIMULATION COMPLETE - FINAL PERFORMANCE REPORT");
    println!("{}", "=".repeat(60));

    // Final comprehensive report (this ends the simulation timing)
    let final_report = sim.generate_performance_report();
    println!("{}", final_report);

    // Demonstrate the difference between reusable and final reports
    println!("\n🔍 REUSABILITY DEMONSTRATION");
    println!("============================");
    
    println!("✅ Reusable methods (can call multiple times during simulation):");
    println!("   • get_performance_summary() - Lightweight component ranking");
    println!("   • get_component_performance(name) - Specific component details");
    println!("   • generate_intermediate_report() - Full report without ending simulation");
    
    println!("\n⚠️  Final method (call once at end):");
    println!("   • generate_performance_report() - Ends simulation timing");
    
    println!("\n🎯 Use Cases:");
    println!("   📊 Real-time monitoring: Use reusable methods");
    println!("   🔧 Component debugging: Use get_component_performance()");
    println!("   📈 Progress tracking: Use get_performance_summary()");
    println!("   📋 Final analysis: Use generate_performance_report()");
    
    println!("\n✅ Reusable Performance Reporting Demo Complete!");
}
