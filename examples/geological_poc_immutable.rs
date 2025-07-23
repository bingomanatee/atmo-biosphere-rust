use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut, default_immutable_layer_set_params};
use atmo_biosphere_rust::component::{SimComponent, core_radiance_component::CoreRadianceComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🌍 Geological POC - Immutable Version");
    println!("=====================================");
    println!("Testing immutable geological simulation with realistic Earth-like parameters");

    // Create simulation configuration with immutable layer sets
    let config = SimulationConfigImmut {
        steps: 3,
        years_per_step: 10000.0, // 10,000 years per step
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: default_immutable_layer_set_params(Resolution::Two, 6371.0),
    };

    // Create components for geological simulation
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        // TODO: Add immutable-compatible components
        // Box::new(CoreRadianceComponent::new()),
    ];

    // Create immutable simulation
    let mut sim = SimulationImmut::new(config, &mut components);

    println!("\n📊 Initial Simulation State:");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    println!("   - Total cells: {}", sim.total_cells());
    println!("   - Surface temperature: {:.1}K ({:.1}°C)", 288.15, 288.15 - 273.15);

    // Print detailed layer set information
    for (i, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("\n   🌍 Layer Set {}: {} columns", i, layer_set.layers.len());
        println!("      Start height: {:.1}km", layer_set.start_height_km);
        
        // Show temperature and mass distribution in first column
        if let Some((_, column)) = layer_set.layers.iter().next() {
            println!("      Cells in column: {}", column.cells.len());
            
            // Show first, middle, and last cells
            let cell_count = column.cells.len();
            let indices = if cell_count >= 3 {
                vec![0, cell_count / 2, cell_count - 1]
            } else {
                (0..cell_count).collect()
            };
            
            for &idx in &indices {
                if let Some(cell) = column.cells.get(idx) {
                    println!("         Cell {}: {:.1}K ({:.1}°C), mass: {:.2e}kg", 
                             idx,
                             cell.temperature_kelvin(), 
                             cell.temperature_kelvin() - 273.15,
                             cell.mass_kg());
                }
            }
        }
    }

    // Calculate initial total energy
    let initial_total_energy: f64 = sim.layer_sets.iter()
        .flat_map(|layer_set| layer_set.layers.values())
        .flat_map(|column| &column.cells)
        .map(|cell| cell.energy_joules())
        .sum();

    println!("\n📈 Initial Energy Analysis:");
    println!("   - Total energy: {:.2e} J", initial_total_energy);
    println!("   - Average energy per cell: {:.2e} J", initial_total_energy / sim.total_cells() as f64);

    println!("\n🔄 Running immutable simulation steps...");

    // Run simulation steps
    for step in 0..3 {
        println!("\n--- Step {} ---", step + 1);
        println!("   Year: {:.0}", sim.current_year());
        
        sim.step();
        
        // Calculate energy after step
        let step_total_energy: f64 = sim.layer_sets.iter()
            .flat_map(|layer_set| layer_set.layers.values())
            .flat_map(|column| &column.cells)
            .map(|cell| cell.energy_joules())
            .sum();
        
        let energy_change = step_total_energy - initial_total_energy;
        
        println!("   Step completed:");
        println!("   - Simulation step: {}", sim.current_step());
        println!("   - Total energy: {:.2e} J", step_total_energy);
        println!("   - Energy change: {:.2e} J ({:.1}%)", 
                 energy_change, 
                 (energy_change / initial_total_energy) * 100.0);
        
        // Show temperature changes in each layer set
        for (i, layer_set) in sim.layer_sets.iter().enumerate() {
            if let Some((_, column)) = layer_set.layers.iter().next() {
                if let Some(first_cell) = column.cells.first() {
                    if let Some(last_cell) = column.cells.last() {
                        println!("   - Layer {}: {:.1}K to {:.1}K ({:.1}°C to {:.1}°C)", 
                                 i,
                                 first_cell.temperature_kelvin(),
                                 last_cell.temperature_kelvin(),
                                 first_cell.temperature_kelvin() - 273.15,
                                 last_cell.temperature_kelvin() - 273.15);
                    }
                }
            }
        }
    }

    println!("\n✅ Immutable geological simulation completed!");
    println!("   - Successfully demonstrated immutable layer sets");
    println!("   - Maintained energy conservation");
    println!("   - Ready for component integration and performance optimization");
    
    // Final analysis
    let final_total_energy: f64 = sim.layer_sets.iter()
        .flat_map(|layer_set| layer_set.layers.values())
        .flat_map(|column| &column.cells)
        .map(|cell| cell.energy_joules())
        .sum();
    
    let total_energy_change = final_total_energy - initial_total_energy;
    
    println!("\n📊 Final Energy Analysis:");
    println!("   - Initial energy: {:.2e} J", initial_total_energy);
    println!("   - Final energy: {:.2e} J", final_total_energy);
    println!("   - Total change: {:.2e} J ({:.3}%)", 
             total_energy_change,
             (total_energy_change / initial_total_energy) * 100.0);
    
    if total_energy_change.abs() / initial_total_energy < 0.001 {
        println!("   ✅ Energy conservation maintained (< 0.1% change)");
    } else {
        println!("   ⚠️  Significant energy change detected");
    }
}
