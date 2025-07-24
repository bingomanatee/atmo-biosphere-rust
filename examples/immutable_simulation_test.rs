use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;

fn main() {
    println!("🌍 Immutable Simulation Test");
    println!("============================");
    println!("Testing immutable layer sets for better performance");

    // Create simulation configuration with immutable layer sets
    let config = SimulationConfigImmut {
        steps: 3,
        years_per_step: 10000.0, // 10,000 years per step
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: default_layer_set_params_immut(Resolution::Two, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig {
            years_per_step: 10000.0,
            max_transfer_rate: 0.005, // 0.5% max transfer per step
            enable_space_radiation: true,
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: false, // Disable for initial testing
        },
    };

    // Create components (disabled for now - requires component trait adaptation)
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        // TODO: Add immutable-compatible components
    ];

    // Create immutable simulation
    let mut sim = SimulationImmut::new(config, &mut components);

    println!("📊 Initial Simulation State:");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    println!("   - Total cells: {}", sim.total_cells());

    // Print initial layer set information
    for (i, layer_set) in sim.layer_sets.iter().enumerate() {
        println!("   - Layer Set {}: {} columns, start height: {:.1}km", 
                 i, layer_set.layers.len(), layer_set.start_height_km);
        
        // Show first few cells from first column
        if let Some((_, column)) = layer_set.layers.iter().next() {
            println!("     First column cells:");
            for (j, cell) in column.cells.iter().take(3).enumerate() {
                println!("       Cell {}: {:.1}K ({:.1}°C), mass: {:.2e}kg", 
                         j, 
                         cell.temperature_kelvin(), 
                         cell.temperature_kelvin() - 273.15,
                         cell.mass_kg());
            }
        }
    }

    println!("\n🔄 Running simulation steps...");

    // Run simulation steps
    for step in 0..3 {
        println!("\n--- Step {} ---", step + 1);
        sim.step();
        
        // Print some statistics after each step
        println!("   Step completed. Current state:");
        println!("   - Simulation step: {}", sim.step);
        println!("   - Total cells: {}", sim.total_cells());
        
        // Show temperature changes in first layer set
        if let Some(first_layer_set) = sim.get_layer_set(0) {
            if let Some((_, column)) = first_layer_set.layers.iter().next() {
                if let Some(first_cell) = column.cells.first() {
                    println!("   - First cell temp: {:.1}K ({:.1}°C)", 
                             first_cell.temperature_kelvin(),
                             first_cell.temperature_kelvin() - 273.15);
                }
            }
        }
    }

    println!("\n✅ Immutable simulation test completed!");
    println!("   - Successfully created and ran immutable layer sets");
    println!("   - Demonstrated immutable pattern for geological simulation");
    println!("   - Ready for performance comparison with mutable version");
}
