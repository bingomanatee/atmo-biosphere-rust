// Enhanced geological simulation with detailed component-by-component logging

use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::thermal_gradient_config::ThermalGradientConfig;
use atmo_biosphere_rust::component::conduction_component::ConductionComponent;
use atmo_biosphere_rust::component::core_radiance_component::CoreRadianceComponent;
use atmo_biosphere_rust::component::convection_plume_component::ConvectionPlumeComponent;
use h3o::Resolution;

fn main() {
    println!("🌍 Geological Simulation with Detailed Component Logging");
    println!("========================================================");
    println!("Demonstrates: Component-by-component performance tracking and transaction logging\n");

    // Create simulation configuration
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
            cells_per_column: 5,
            material_name: "granite".to_string(),
            surface_temperature_kelvin: 288.0,
            thermal_gradient_per_km: 25.0,
        },
        LayerSetParams {
            name: "Upper Mantle".to_string(),
            start_height_km: -50.0,
            end_height_km: -200.0,
            cells_per_column: 8,
            material_name: "peridotite".to_string(),
            surface_temperature_kelvin: 1500.0,
            thermal_gradient_per_km: 15.0,
        },
        LayerSetParams {
            name: "Lower Mantle".to_string(),
            start_height_km: -200.0,
            end_height_km: -300.0,
            cells_per_column: 5,
            material_name: "peridotite".to_string(),
            surface_temperature_kelvin: 2000.0,
            thermal_gradient_per_km: 10.0,
        },
    ];

    let config = SimulationConfig {
        steps: 5,
        years_per_step: 10000.0, // 10,000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Create components with detailed logging
    let mut components: Vec<Box<dyn atmo_biosphere_rust::component::SimComponent>> = vec![
        Box::new(ConductionComponent::new()),
        Box::new(CoreRadianceComponent::new(140.0, 3.5e6)), // 140 W/m³, 3.5M year tau
        Box::new(ConvectionPlumeComponent::with_seed(12345)),
    ];

    println!("🔧 Components initialized:");
    println!("   • ThermalConduction: Heat transfer between layers");
    println!("   • CoreRadiance: Exponential cooling from core");
    println!("   • ConvectionPlumes: Buoyancy-driven mass transport");

    // Create simulation
    let mut sim = Simulation::new(config.clone(), &mut components);
    sim.initialize();

    println!("\n📊 Initial System State:");
    print_system_summary(&sim);

    println!("\n🚀 Running {} steps with detailed component logging...", config.steps);
    println!("{}", "=".repeat(80));

    // Run simulation with detailed logging
    for step in 0..config.steps {
        println!("\n🔄 STEP {} - Year {}", step + 1, sim.current_year());
        println!("{}", "-".repeat(50));
        
        let step_start = std::time::Instant::now();
        
        // Run step with transaction debug for first step
        if step == 0 {
            println!("🔍 Running with transaction debug enabled...");
            sim.step_with_debug(true);
        } else {
            sim.step();
        }
        
        let step_duration = step_start.elapsed();
        
        // Print step summary
        println!("\n📈 Step {} Summary:", step + 1);
        println!("   Total time: {:.2} ms", step_duration.as_secs_f64() * 1000.0);
        print_component_performance(&sim, step + 1);
        
        // Print system state changes
        if step % 2 == 0 {
            println!("\n📊 System State After Step {}:", step + 1);
            print_system_summary(&sim);
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("🏁 SIMULATION COMPLETE");
    println!("{}", "=".repeat(80));

    // Generate comprehensive performance report
    println!("\n📊 COMPREHENSIVE PERFORMANCE ANALYSIS");
    println!("=====================================");
    let performance_report = sim.generate_performance_report();
    println!("{}", performance_report);

    // Additional component analysis
    print_detailed_component_analysis(&sim);

    println!("\n✅ Detailed component logging demonstration complete!");
}

fn print_system_summary(sim: &Simulation) {
    let total_cells = sim.layer_sets.iter()
        .map(|ls| ls.layers.len() * ls.layers.values().next().map_or(0, |col| col.cells.len()))
        .sum::<usize>();
    
    let total_energy: f64 = sim.layer_sets.iter()
        .flat_map(|ls| ls.layers.values())
        .flat_map(|col| &col.cells)
        .map(|cell| cell.energy_joules())
        .sum();
    
    let total_mass: f64 = sim.layer_sets.iter()
        .flat_map(|ls| ls.layers.values())
        .flat_map(|col| &col.cells)
        .map(|cell| cell.mass_kg())
        .sum();
    
    let avg_temp: f64 = sim.layer_sets.iter()
        .flat_map(|ls| ls.layers.values())
        .flat_map(|col| &col.cells)
        .map(|cell| cell.temperature_kelvin())
        .sum::<f64>() / total_cells as f64;

    println!("   Total cells: {}", total_cells);
    println!("   Total energy: {:.2e} J", total_energy);
    println!("   Total mass: {:.2e} kg", total_mass);
    println!("   Average temperature: {:.1} K ({:.1}°C)", avg_temp, avg_temp - 273.15);
    println!("   Active plumes: {}", sim.plumes.len());
}

fn print_component_performance(sim: &Simulation, step: usize) {
    let component_summary = sim.profiler.get_component_summary();
    
    println!("   Component Performance:");
    for (component_name, metrics) in component_summary {
        let total_time_ms = metrics.total_time_ms();
        let method_count = metrics.methods.len();
        
        println!("     • {}: {:.2} ms ({} methods)", 
            component_name, total_time_ms, method_count);
        
        // Show top 2 methods for each component
        let mut methods: Vec<_> = metrics.methods.iter().collect();
        methods.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
        
        for (method_name, method_metrics) in methods.iter().take(2) {
            let method_time_ms = method_metrics.total_time.as_secs_f64() * 1000.0;
            println!("       - {}: {:.2} ms ({} calls)", 
                method_name, method_time_ms, method_metrics.call_count);
        }
    }
}

fn print_detailed_component_analysis(sim: &Simulation) {
    println!("\n🔬 DETAILED COMPONENT ANALYSIS");
    println!("==============================");
    
    let component_summary = sim.profiler.get_component_summary();
    let total_sim_time = sim.profiler.total_time_ms();
    
    for (component_name, metrics) in component_summary {
        let component_time = metrics.total_time_ms();
        let percentage = if total_sim_time > 0.0 {
            (component_time / total_sim_time) * 100.0
        } else {
            0.0
        };
        
        println!("\n🔧 Component: {}", component_name);
        println!("   Total time: {:.2} ms ({:.1}% of simulation)", component_time, percentage);
        println!("   Methods executed: {}", metrics.methods.len());
        
        // Method breakdown
        let mut methods: Vec<_> = metrics.methods.iter().collect();
        methods.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
        
        println!("   Method breakdown:");
        for (method_name, method_metrics) in methods {
            let method_time_ms = method_metrics.total_time.as_secs_f64() * 1000.0;
            let method_percentage = if component_time > 0.0 {
                (method_time_ms / component_time) * 100.0
            } else {
                0.0
            };
            
            println!("     • {}: {:.2} ms ({:.1}%) - {} calls, avg {:.2} ms/call", 
                method_name, 
                method_time_ms, 
                method_percentage,
                method_metrics.call_count,
                method_time_ms / method_metrics.call_count as f64);
        }
        
        // Performance insights
        if percentage > 50.0 {
            println!("   🚨 HIGH IMPACT: This component uses >50% of simulation time");
        } else if percentage > 25.0 {
            println!("   ⚠️  MODERATE IMPACT: This component uses >25% of simulation time");
        } else {
            println!("   ✅ LOW IMPACT: This component is well-optimized");
        }
    }
    
    // Overall insights
    println!("\n🎯 OPTIMIZATION INSIGHTS");
    println!("========================");
    
    let mut all_methods: Vec<_> = component_summary.iter()
        .flat_map(|(comp_name, metrics)| {
            metrics.methods.iter().map(move |(method_name, method_metrics)| {
                (comp_name, method_name, method_metrics)
            })
        })
        .collect();
    
    all_methods.sort_by(|a, b| b.2.total_time.cmp(&a.2.total_time));
    
    println!("🏆 Top 3 most expensive methods:");
    for (i, (comp_name, method_name, metrics)) in all_methods.iter().take(3).enumerate() {
        let time_ms = metrics.total_time.as_secs_f64() * 1000.0;
        let percentage = (time_ms / total_sim_time) * 100.0;
        println!("   {}. {}::{}: {:.2} ms ({:.1}%)", 
            i + 1, comp_name, method_name, time_ms, percentage);
    }
    
    all_methods.sort_by(|a, b| b.2.call_count.cmp(&a.2.call_count));
    
    println!("\n🔄 Top 3 most called methods:");
    for (i, (comp_name, method_name, metrics)) in all_methods.iter().take(3).enumerate() {
        println!("   {}. {}::{}: {} calls", 
            i + 1, comp_name, method_name, metrics.call_count);
    }
}
