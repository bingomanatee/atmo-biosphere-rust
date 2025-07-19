// Trial run of the transaction system to verify rational results

use atmo_biosphere_rust::sim::simulation::Simulation;
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::SimComponent;
use h3o::Resolution;

/// Simple test component that generates predictable transactions
pub struct TestComponent {
    pub name: String,
}

impl TestComponent {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl SimComponent for TestComponent {
    fn step(&mut self, simulation: &mut Simulation, step: i64, _year: i64) {
        println!("🔧 {} running step {}", self.name, step);
        
        // Generate some test transactions
        for (layer_index, layer_set) in simulation.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for depth_index in 0..column.cells.len() {
                    if let Some(cell) = simulation.get_cell(layer_index, *h3_cell, depth_index) {
                        // Small energy change (should be within limits)
                        let energy_change = cell.energy_joules() * 0.001; // 0.1%
                        
                        simulation.propose_energy_transaction(
                            &self.name,
                            layer_index,
                            *h3_cell,
                            depth_index,
                            energy_change,
                            &format!("Test energy change: {:.2e}J", energy_change),
                        );
                        
                        // Only process first few cells to keep output manageable
                        if depth_index >= 2 { break; }
                    }
                }
                // Only process first cell to keep output manageable
                break;
            }
            // Only process first layer to keep output manageable
            if layer_index >= 1 { break; }
        }
    }
}

fn main() {
    println!("🧪 Transaction System Trial Run");
    println!("Testing if the system produces rational results\n");
    
    // Create a minimal simulation
    let mut simulation = create_test_simulation();
    
    // Add a test component
    let test_component = Box::new(TestComponent::new("TestComponent"));
    simulation.add_component("test", test_component);
    
    println!("📊 Initial State:");
    print_simulation_state(&simulation);
    
    // Run a few steps
    for step in 0..3 {
        println!("\n{}", "=".repeat(50));
        println!("🔄 Running Step {}", step);
        
        // Run with debug on first step to see what happens
        if step == 0 {
            simulation.step_with_debug(true);
        } else {
            simulation.step();
        }
        
        println!("\n📊 State After Step {}:", step);
        print_simulation_state(&simulation);
    }
    
    println!("\n✅ Transaction System Trial Completed!");
    println!("Check the output above to verify:");
    println!("   • Transactions are generated correctly");
    println!("   • Energy changes are within geological limits");
    println!("   • Cell states change rationally");
    println!("   • No mass/energy is lost or created inappropriately");
}

fn create_test_simulation() -> Simulation {
    // Create minimal layer set parameters
    let layer_params = vec![
        LayerSetParams {
            name: "Test Crust".to_string(),
            start_height_km: 0.0,
            end_height_km: -50.0,
            cells_per_column: 3,
            material_name: "granite".to_string(),
            surface_temperature_kelvin: 288.0,
            thermal_gradient_per_km: 25.0,
        },
        LayerSetParams {
            name: "Test Mantle".to_string(),
            start_height_km: -50.0,
            end_height_km: -200.0,
            cells_per_column: 3,
            material_name: "peridotite".to_string(),
            surface_temperature_kelvin: 1500.0,
            thermal_gradient_per_km: 15.0,
        },
    ];
    
    // Create simulation with minimal H3 resolution for testing
    let mut simulation = Simulation::new(
        layer_params,
        Resolution::Two, // Very coarse resolution for testing
        1000.0,         // 1000 years per step
    );
    
    // Initialize with some basic state
    simulation.initialize_h3_cells();
    
    simulation
}

fn print_simulation_state(simulation: &Simulation) {
    println!("   Layers: {}", simulation.layer_sets.len());
    
    for (layer_index, layer_set) in simulation.layer_sets.iter().enumerate() {
        println!("   Layer {}: {} columns", layer_index, layer_set.layers.len());
        
        for (h3_cell, column) in &layer_set.layers {
            println!("     H3 {}: {} cells", h3_cell, column.cells.len());
            
            for (depth_index, cell) in column.cells.iter().enumerate() {
                println!("       Depth {}: {:.2e}J, {:.2e}kg, {:.1}K", 
                    depth_index,
                    cell.energy_joules(),
                    cell.mass_kg(),
                    cell.temperature_kelvin());
            }
            
            // Only show first column to keep output manageable
            break;
        }
    }
}
