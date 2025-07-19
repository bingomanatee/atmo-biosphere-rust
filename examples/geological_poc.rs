use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, conduction_component::ConductionComponent, core_radiance_component::CoreRadianceComponent, convection_plume_component::ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌍 Geological Simulation POC");
    println!("============================");
    println!("Demonstrates: Buoyancy-based physics, thermal conduction, performance profiling");

    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km in crust
        deep_gradient_k_per_km: 10.0,       // 10K/km in asthenosphere
        reference_depth_km: 200.0,          // Transition at 200km depth
    };

    // Realistic geological structure (0-265km) matching RADIANCE.md
    let layer_params = vec![
        // Crust: 0-50km (10 layers × 5km each)
        LayerSetParams {
            resolution: Resolution::Three,   // Increased by one level
            start_height_km: 0.0,
            cell_height_km: 5.0,             // 5km crust layers
            material_name: "basalt".to_string(),
            column_count: 10,                // 50km total
            planet_radius_km: 6371.0,
        },
        // Upper Mantle: 50-150km (10 layers × 10km each)
        LayerSetParams {
            resolution: Resolution::Two,     // Increased by one level
            start_height_km: 50.0,
            cell_height_km: 10.0,            // 10km mid layers
            material_name: "granite".to_string(),
            column_count: 10,                // 100km total
            planet_radius_km: 6371.0,
        },
        // Lower Mantle: 150-225km (5 layers × 15km each)
        LayerSetParams {
            resolution: Resolution::One,     // Increased by one level
            start_height_km: 150.0,
            cell_height_km: 15.0,            // 15km deep layers
            material_name: "basalt".to_string(),
            column_count: 5,                 // 75km total
            planet_radius_km: 6371.0,
        },
        // Asthenosphere: 225-265km (2 layers × 20km each)
        LayerSetParams {
            resolution: Resolution::One,     // Increased by one level
            start_height_km: 225.0,
            cell_height_km: 20.0,            // 20km bottom layers
            material_name: "granite".to_string(),
            column_count: 2,                 // 40km total
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 3,
        years_per_step: 10000.0,
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()),      // Energy input from core
        Box::new(ConductionComponent::new()),        // Heat transfer (slower via material modifier)
        Box::new(ConvectionPlumeComponent::with_seed(42)), // Hotspot energy transport (preserves sources)
    ];

    let mut sim = Simulation::new(config, &mut components);
    sim.initialize();

    println!("\n🌡️ Initial Temperature Profile:");
    print_temperature_profile(&sim);

    println!("\n🚀 Running 3 steps (30,000 years) with full reporting...");

    for step in 0..3 {
        println!("\n{}", "=".repeat(60));
        println!("🔄 STEP {} - Year {}", step + 1, sim.current_year());
        println!("{}", "=".repeat(60));

        let step_start = std::time::Instant::now();

        // Run with transaction debug on first step
        if step == 0 {
            println!("🔍 Running with transaction debug enabled...");
            sim.step_with_debug(true);
        } else {
            sim.step();
        }

        let step_duration = step_start.elapsed();

        // Detailed step reporting
        println!("\n📈 Step {} Summary:", step + 1);
        println!("   Duration: {:.3}s ({:.1} ms)",
            step_duration.as_secs_f64(), step_duration.as_secs_f64() * 1000.0);
        println!("   Year: {}", sim.current_year());
        println!("   Active plumes: {}", sim.plumes.len());

        // Component performance for this step
        print_step_component_performance(&sim, step + 1);

        // System state summary every step
        if step % 1 == 0 {
            print_system_state_summary(&sim, step + 1);
        }
    }

    println!("\n🌡️ Final Temperature Profile:");
    print_temperature_profile(&sim);

    println!("\n✅ Geological simulation completed!");
    println!("   Total time: {} years", sim.current_year());

    println!("\n📊 COMPREHENSIVE FINAL ANALYSIS");
    println!("================================");

    // 1. Performance report
    println!("🏆 PERFORMANCE ANALYSIS:");
    let performance_report = sim.generate_performance_report();
    println!("{}", performance_report);

    // 2. Energy conservation analysis
    print_energy_conservation_analysis(&sim);

    // 3. Component effectiveness analysis
    print_component_effectiveness_analysis(&sim);

    // 4. Transaction system summary
    print_transaction_system_summary(&sim);
}

fn print_temperature_profile(sim: &Simulation) {
    // Get the first H3 cell for detailed layer breakdown
    if let Some((first_h3_index, first_column)) = sim.layer_sets.get(0)
        .and_then(|layer_set| layer_set.layers.iter().next()) {

        println!("🔬 Geological Analysis (Cell {}):", first_h3_index);

        // Calculate surface area from first cell
        if let Some(first_cell) = first_column.cells.first() {
            println!("   Surface Area: {:.2e} km²", first_cell.area());
            println!("   Planet: Earth");
        }

        // Count total layers across all layer sets
        let total_layers: usize = sim.layer_sets.iter()
            .map(|layer_set| layer_set.layers.values().next()
                .map(|col| col.cells.len()).unwrap_or(0))
            .sum();
        println!("   Total layers in this cell: {}", total_layers);
        println!("   Simulation: {} steps, {} years total", sim.current_step(), sim.current_year());

        // Print header exactly like reference
        println!();
        println!("   Lyr  Depth Range   Height        Phase     Material  Temp(K) Temp(°C)     Mass(kg)  Volume(km³) Density(kg/m³)  Energy(J)");
        println!("   --- ------------ -------- --- -------- ------------ -------- -------- ------------ ------------ ---------- ------------");

        // Print each layer in the exact reference format
        let mut layer_counter = 0;
        for layer_set in &sim.layer_sets {
            // Find the corresponding column in this layer set
            let column = if let Some(column) = layer_set.layers.get(first_h3_index) {
                column
            } else {
                layer_set.layers.values().next().unwrap()
            };

            for cell in &column.cells {
                let depth_start = cell.top_km;
                let depth_end = cell.top_km + cell.height_km;
                let temp_k = cell.temperature_kelvin();
                let temp_c = temp_k - 273.15;
                let mass_kg = cell.mass_kg();
                let volume_km3 = cell.area() * cell.height_km;
                let density = if volume_km3 > 0.0 { mass_kg / (volume_km3 * 1e9) } else { 0.0 };
                let material_name = &cell.material().name;
                let energy_j = cell.energy_joules();

                // Determine phase symbol and name
                let (phase_symbol, phase_name) = if temp_k < cell.material().melt_temp as f64 {
                    ("🧊", "Solid")
                } else if temp_k < cell.material().boil_temp as f64 {
                    ("🌊", "Liquid")
                } else {
                    ("💨", "Gas")
                };

                // Format exactly like reference
                println!("   🗻{:<2} {:>6.1}-{:<6.1}km {:>8.1}   {} {:>8} {:>12} {:>8.0} {:>8.0} {:>12.2e} {:>12.2e} {:>10.0} {:>12.2e}",
                    layer_counter,
                    depth_start,
                    depth_end,
                    cell.height_km,
                    phase_symbol,
                    phase_name,
                    material_name,
                    temp_k,
                    temp_c,
                    mass_kg,
                    volume_km3,
                    density,
                    energy_j
                );

                layer_counter += 1;
            }
        }
    } else {
        println!("⚠️  No cells found for geological analysis!");
    }
}

/// Print component performance for current step
fn print_step_component_performance(sim: &Simulation, step: usize) {
    println!("\n🔧 Component Performance (Step {}):", step);

    let component_summary = sim.profiler.get_component_summary();
    if component_summary.is_empty() {
        println!("   No component data available");
        return;
    }

    // Sort by total time
    let mut components: Vec<_> = component_summary.iter().collect();
    components.sort_by(|a, b| b.1.total_time().cmp(&a.1.total_time()));

    for (component_name, metrics) in components.iter().take(5) {
        let total_time_ms = metrics.total_time_ms();
        println!("   • {}: {:.2} ms", component_name, total_time_ms);

        // Show top method for each component
        if let Some((method_name, method_metrics)) = metrics.methods.iter()
            .max_by_key(|(_, m)| m.total_time) {
            let method_time_ms = method_metrics.total_time.as_secs_f64() * 1000.0;
            println!("     └─ {}: {:.2} ms ({} calls)",
                method_name, method_time_ms, method_metrics.call_count);
        }
    }
}

/// Print system state summary
fn print_system_state_summary(sim: &Simulation, step: usize) {
    println!("\n📊 System State (Step {}):", step);

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

    println!("   Total energy: {:.2e} J", total_energy);
    println!("   Total mass: {:.2e} kg", total_mass);
    println!("   Average temperature: {:.1} K ({:.1}°C)", avg_temp, avg_temp - 273.15);
    println!("   Active plumes: {}", sim.plumes.len());
    println!("   Cells: {}", total_cells);
}
