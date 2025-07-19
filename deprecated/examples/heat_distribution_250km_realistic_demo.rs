use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌍 Heat Distribution in 250km Realistic Geological Structure: Plumes + Radiance");
    println!("=================================================================================");

    // Realistic thermal gradient for 250km depth (proper asthenosphere range)
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 30.0,    // 30K/km in crust/lithosphere
        deep_gradient_k_per_km: 5.0,        // 5K/km in asthenosphere (lower gradient)
        reference_depth_km: 80.0,           // Transition at lithosphere-asthenosphere boundary
    };

    // Create realistic geological layer structure (0-250km)
    let layer_params = vec![
        // Continental Crust (0-35km)
        LayerSetParams {
            resolution: Resolution::Three,   // Moderate resolution for analysis
            start_height_km: 0.0,
            cell_height_km: 17.5,           // Smaller cells for crust detail
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 35km
            planet_radius_km: 6371.0,
        },
        // Lithospheric Mantle (35-80km) - Rigid layer
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 35.0,
            cell_height_km: 22.5,           // 2 cells = 45km
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
        // Upper Asthenosphere (80-120km) - Main convection zone
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 80.0,
            cell_height_km: 20.0,           // 2 cells = 40km
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
        // Mid Asthenosphere (120-160km) - Main convection zone
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 120.0,
            cell_height_km: 20.0,           // 2 cells = 40km
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
        // Lower Asthenosphere (160-200km) - Main convection zone
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 160.0,
            cell_height_km: 20.0,           // 2 cells = 40km
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
        // Upper Mantle Transition Zone (200-250km) - Core radiance target
        LayerSetParams {
            resolution: Resolution::Three,
            start_height_km: 200.0,
            cell_height_km: 25.0,           // 2 cells = 50km
            material_name: "basalt".to_string(),
            column_count: 2,
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 2,                           // Just 2 steps for quick analysis
        years_per_step: 5000.0,            // 5000 years per step (10,000 years total)
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    // Calculate expected temperatures at key depths
    println!("\n🌡️ Expected Temperature Profile (Realistic Geological Structure):");
    let surface_temp = config.thermal_config.surface_temperature_k;
    let temp_at_35km = surface_temp + 35.0 * config.thermal_config.surface_gradient_k_per_km;
    let temp_at_80km = surface_temp + 80.0 * config.thermal_config.surface_gradient_k_per_km;
    let temp_at_200km = temp_at_80km + 120.0 * config.thermal_config.deep_gradient_k_per_km;
    let temp_at_250km = temp_at_80km + 170.0 * config.thermal_config.deep_gradient_k_per_km;

    println!("   Surface (0km): {:.0}K ({:.0}°C)", surface_temp, surface_temp - 273.15);
    println!("   Crust base (35km): {:.0}K ({:.0}°C)", temp_at_35km, temp_at_35km - 273.15);
    println!("   Lithosphere base (80km): {:.0}K ({:.0}°C)", temp_at_80km, temp_at_80km - 273.15);
    println!("   Mid Asthenosphere (200km): {:.0}K ({:.0}°C)", temp_at_200km, temp_at_200km - 273.15);
    println!("   Deep Transition (250km): {:.0}K ({:.0}°C)", temp_at_250km, temp_at_250km - 273.15);
    println!("   🌋 Plume threshold: 1800K (1527°C)");

    if temp_at_200km > 1800.0 {
        println!("   ✅ Asthenosphere temperatures should trigger plume formation!");
        println!("   📊 Expected pressure at 200km: ~6 GPa (should keep basalt solid/liquid)");
    } else {
        println!("   ⚠️  May need core radiance to reach plume threshold");
    }

    // Helper function to analyze heat distribution with per-cell averages
    fn analyze_heat_distribution(sim: &Simulation, test_name: &str, step: i64) {
        println!("\n🌡️ Heat Distribution Analysis: {} (Step {})", test_name, step);
        println!("============================================================");

        let mut total_energy_by_layer = Vec::new();
        let mut avg_temp_by_layer = Vec::new();
        let mut avg_energy_per_cell_by_layer = Vec::new();
        let mut cell_count_by_layer = Vec::new();

        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            let mut layer_total_energy = 0.0;
            let mut layer_total_cells = 0;
            let mut layer_total_temp = 0.0;

            for column in layer_set.layers.values() {
                for cell in &column.cells {
                    layer_total_energy += cell.energy_joules();
                    layer_total_temp += cell.temperature_kelvin();
                    layer_total_cells += 1;
                }
            }

            let avg_temp = if layer_total_cells > 0 {
                layer_total_temp / layer_total_cells as f64
            } else { 0.0 };

            let avg_energy_per_cell = if layer_total_cells > 0 {
                layer_total_energy / layer_total_cells as f64
            } else { 0.0 };

            total_energy_by_layer.push(layer_total_energy);
            avg_temp_by_layer.push(avg_temp);
            avg_energy_per_cell_by_layer.push(avg_energy_per_cell);
            cell_count_by_layer.push(layer_total_cells);

            // Calculate layer height from cells
            let layer_height_km = if let Some(first_column) = layer_set.layers.values().next() {
                first_column.cells.len() as f64 * 25.0 // Assuming 25km per cell
            } else {
                50.0 // Default
            };

            let depth_range = format!("{}-{}km",
                layer_set.start_height_km,
                layer_set.start_height_km + layer_height_km);

            println!("   Layer {}: {} | {} cells | {:.2e}J/cell | {:.2e}J total | {:.0}K ({:.0}°C)",
                layer_idx,
                depth_range,
                layer_total_cells,
                avg_energy_per_cell,
                layer_total_energy,
                avg_temp,
                avg_temp - 273.15);
        }

        // Calculate energy gradients
        println!("\n📊 Energy Distribution:");
        let total_energy: f64 = total_energy_by_layer.iter().sum();
        for (i, energy) in total_energy_by_layer.iter().enumerate() {
            let percentage = (energy / total_energy) * 100.0;
            println!("   Layer {}: {:.1}% of total energy ({:.2e}J/cell)", i, percentage, avg_energy_per_cell_by_layer[i]);
        }

        // Store data for comparison if this is start or end
        if step == 0 || step == 25 {
            println!("\n📋 Per-Cell Energy Summary:");
            for (i, avg_energy) in avg_energy_per_cell_by_layer.iter().enumerate() {
                println!("   Layer {}: {:.2e} J/cell ({} cells)", i, avg_energy, cell_count_by_layer[i]);
            }
        }
    }

    // Helper function to compare start vs end energy per cell
    fn compare_start_end_energy(sim_start: &Simulation, sim_end: &Simulation, test_name: &str) {
        println!("\n📈 START vs END Comparison: {}", test_name);
        println!("============================================================");
        println!("   Layer | Start J/cell | End J/cell   | Change       | % Change");
        println!("   ------|--------------|--------------|--------------|----------");

        for (layer_idx, (start_layer, end_layer)) in sim_start.layer_sets.iter().zip(sim_end.layer_sets.iter()).enumerate() {
            // Calculate start energy per cell
            let mut start_total_energy = 0.0;
            let mut start_total_cells = 0;
            for column in start_layer.layers.values() {
                for cell in &column.cells {
                    start_total_energy += cell.energy_joules();
                    start_total_cells += 1;
                }
            }
            let start_avg = if start_total_cells > 0 { start_total_energy / start_total_cells as f64 } else { 0.0 };

            // Calculate end energy per cell
            let mut end_total_energy = 0.0;
            let mut end_total_cells = 0;
            for column in end_layer.layers.values() {
                for cell in &column.cells {
                    end_total_energy += cell.energy_joules();
                    end_total_cells += 1;
                }
            }
            let end_avg = if end_total_cells > 0 { end_total_energy / end_total_cells as f64 } else { 0.0 };

            let change = end_avg - start_avg;
            let percent_change = if start_avg > 0.0 { (change / start_avg) * 100.0 } else { 0.0 };

            println!("   {:5} | {:12.2e} | {:12.2e} | {:+12.2e} | {:+8.1}%",
                layer_idx, start_avg, end_avg, change, percent_change);
        }
    }

    // Test 1: BASELINE (no components)
    println!("\n🔍 Test 1: BASELINE (no radiance, no plumes)");
    println!("=============================================");

    let mut components_baseline: Vec<Box<dyn SimComponent>> = vec![];
    let mut sim_baseline = Simulation::new(config.clone(), &mut components_baseline);
    sim_baseline.initialize();

    // Store initial state for comparison
    let mut components_baseline_initial: Vec<Box<dyn SimComponent>> = vec![];
    let sim_baseline_initial = Simulation::new(config.clone(), &mut components_baseline_initial);

    analyze_heat_distribution(&sim_baseline, "BASELINE Initial", 0);

    println!("Running 2 steps (10,000 years) with no components...");
    for step in 0..2 {
        sim_baseline.step();
        println!("   Step {} completed", step + 1);
    }
    analyze_heat_distribution(&sim_baseline, "BASELINE Final", 2);
    compare_start_end_energy(&sim_baseline_initial, &sim_baseline, "BASELINE");

    // Test 2: PLUMES + RADIANCE (full system)
    println!("\n🔥 Test 2: PLUMES + RADIANCE (full geological system)");
    println!("====================================================");

    let mut components_full: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(3e20)        // Substantial energy injection
            .with_noise_amplitude(0.15)    // ±15% spatial variation
            .with_spatial_scale(0.08)      // Coarse features for hot spots
            .with_geological_drift()),     // Temporal evolution
        Box::new(ConvectionPlumeComponent::with_seed(42)
            .with_plume_config(1e-11, 0.4)     // Moderate plume frequency, high energy transfer
            .with_temperature_threshold(1800.0)),
    ];
    let mut sim_full = Simulation::new(config.clone(), &mut components_full);
    sim_full.initialize();

    // Store initial state for comparison
    let mut components_full_initial: Vec<Box<dyn SimComponent>> = vec![];
    let sim_full_initial = Simulation::new(config.clone(), &mut components_full_initial);

    analyze_heat_distribution(&sim_full, "FULL SYSTEM Initial", 0);

    println!("Running 2 steps (10,000 years) with plumes + radiance...");
    for step in 0..2 {
        sim_full.step();
        println!("   Step {} completed", step + 1);
    }
    analyze_heat_distribution(&sim_full, "FULL SYSTEM Final", 2);
    compare_start_end_energy(&sim_full_initial, &sim_full, "FULL SYSTEM");

    println!("\n🔬 Comparative Analysis");
    println!("=======================");
    println!("   Compare the heat distribution patterns above:");
    println!("   📊 BASELINE: Natural thermal gradient only");
    println!("   🔥 FULL SYSTEM: Core radiance + plume transport");

    println!("\n🎯 Expected Effects (Realistic Geological Structure):");
    println!("   1. 🔥 Core Radiance Effects:");
    println!("      - Deep transition zone (200-250km) gets energy injection");
    println!("      - Realistic temperatures: 1800-2100K (not 5000K!)");
    println!("      - Spatial hot spots from Perlin noise variation");
    println!("      - Temporal evolution from geological drift");
    println!();
    println!("   2. 🌋 Plume Formation Effects:");
    println!("      - Asthenosphere (80-200km) is main plume generation zone");
    println!("      - Realistic pressure (2-6 GPa) keeps basalt solid/liquid");
    println!("      - Exponential probability with temperature");
    println!("      - Layer height affects plume generation");
    println!();
    println!("   3. 🌊 Heat Transport Effects:");
    println!("      - Plumes transport energy from asthenosphere to lithosphere");
    println!("      - Energy redistribution across realistic 250km column");
    println!("      - Proper geological convective heat transfer");
    println!();
    println!("   4. 🏔️ Realistic Layer Behavior:");
    println!("      - Crust (0-35km): Cool, rigid");
    println!("      - Lithosphere (35-80km): Warm, rigid");
    println!("      - Asthenosphere (80-200km): Hot, convective");
    println!("      - Transition (200-250km): Very hot, target for core energy");

    println!("\n✅ This demonstrates realistic geological convection:");
    println!("   Core Energy → Asthenosphere Heating → Plume Formation → Lithosphere Transport → Surface Heat Flow");
}
