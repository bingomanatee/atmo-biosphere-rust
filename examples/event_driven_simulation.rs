use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
use atmo_biosphere_rust::collections::EventEmitter;
use atmo_biosphere_rust::cell_location::CellLocation;
use h3o::Resolution;
use std::sync::Arc;

/// Example thermal component that emits events instead of direct modification
struct ThermalComponent;

impl ThermalComponent {
    fn process(&self, sim: &Simulation, emitter: &EventEmitter) {
        let cells = sim.get_geological_cells();
        
        for entry in cells.iter() {
            let (location, data) = (entry.key(), entry.value());
            
            // Calculate thermal changes based on current state
            let temp_delta = if location.layer_set_index() == 0 {
                // Surface layer gets solar heating
                5.0
            } else {
                // Deeper layers get geothermal heating
                2.0
            };
            
            // EMIT EVENT instead of direct modification
            emitter.add_to_field("GEOLOGICAL_CELLS", *location, "temperature_k", temp_delta);
            
            // Also add some energy
            let energy_delta = temp_delta * 1000.0; // Simple conversion
            emitter.add_to_field("GEOLOGICAL_CELLS", *location, "energy_joules", energy_delta);
        }
    }
}

/// Example pressure component
struct PressureComponent;

impl PressureComponent {
    fn process(&self, sim: &Simulation, emitter: &EventEmitter) {
        let cells = sim.get_geological_cells();
        
        for entry in cells.iter() {
            let (location, data) = (entry.key(), entry.value());
            
            // Calculate pressure changes based on depth
            let depth_factor = (location.depth_index() + 1) as f64;
            let pressure_delta = depth_factor * 1000.0; // More pressure at depth
            
            // EMIT EVENT - multiple components can add to same field
            emitter.add_to_field("GEOLOGICAL_CELLS", *location, "pressure_pa", pressure_delta);
        }
    }
}

/// Example density component
struct DensityComponent;

impl DensityComponent {
    fn process(&self, sim: &Simulation, emitter: &EventEmitter) {
        let cells = sim.get_geological_cells();
        
        for entry in cells.iter() {
            let (location, data) = (entry.key(), entry.value());
            
            // Set density based on temperature (hotter = less dense)
            let base_density = 2500.0;
            let temp_factor = 1.0 - (data.temperature_k - 300.0) / 10000.0;
            let new_density = base_density * temp_factor.max(0.5);
            
            // EMIT SET EVENT (absolute value, not additive)
            emitter.set_field("GEOLOGICAL_CELLS", *location, "density_kg_m3", new_density);
        }
    }
}

fn main() {
    println!("🌍 Event-Driven Geological Simulation");
    
    // Create simulation
    let config = SimulationConfig {
        planet: PlanetConfig {
            radius_km: 6371.0,
            surface_gravity_m_s_s: 9.81,
        },
        years_per_step: 1000,
        steps: 5,
        layers: vec![
            LayerConfig {
                height_per_step_km: 10.0,
                resolution: Resolution::Five,
                name: "Crust".to_string(),
            },
            LayerConfig {
                height_per_step_km: 50.0,
                resolution: Resolution::Four,
                name: "Upper Mantle".to_string(),
            },
        ],
    };
    
    let mut sim = Simulation::new(config);
    sim.initialize_cells();
    
    // Create components
    let thermal_component = ThermalComponent;
    let pressure_component = PressureComponent;
    let density_component = DensityComponent;
    
    println!("✅ Simulation initialized with {} cells", sim.get_geological_cells().len());
    
    // Show initial state
    println!("\n📊 Initial state (first 3 cells):");
    let mut count = 0;
    for entry in sim.get_geological_cells().iter() {
        if count >= 3 { break; }
        let (location, data) = (entry.key(), entry.value());
        println!("  Cell {}: Layer[{}] Temp[{:.1}K] Pressure[{:.0}Pa] Density[{:.0}kg/m³]",
                 count + 1, location.layer_set_index(), data.temperature_k, data.pressure_pa, data.density_kg_m3);
        count += 1;
    }
    
    // Run simulation steps with parallel component processing
    for step in 1..=3 {
        println!("\n🚀 Step {}: Processing components in parallel...", step);
        
        // Get event emitter for thread-safe event emission
        let emitter = sim.coll_mgr.get_event_emitter();
        
        // Process components in parallel using crossbeam
        crossbeam::scope(|s| {
            let emitter1 = emitter.clone();
            let emitter2 = emitter.clone();
            let emitter3 = emitter.clone();
            
            // Thermal component in parallel
            s.spawn(|_| {
                println!("  🔥 Thermal component processing...");
                thermal_component.process(&sim, &emitter1);
            });
            
            // Pressure component in parallel
            s.spawn(|_| {
                println!("  💨 Pressure component processing...");
                pressure_component.process(&sim, &emitter2);
            });
            
            // Density component in parallel
            s.spawn(|_| {
                println!("  🪨 Density component processing...");
                density_component.process(&sim, &emitter3);
            });
        }).unwrap();
        
        // Apply all events atomically (with compression)
        println!("  📝 Applying all events atomically...");
        sim.coll_mgr.apply_pending_events().unwrap();
        
        // Show updated state
        println!("  ✅ Step {} completed", step);
        let mut count = 0;
        for entry in sim.get_geological_cells().iter() {
            if count >= 2 { break; }
            let (location, data) = (entry.key(), entry.value());
            println!("    Cell {}: Temp[{:.1}K] Pressure[{:.0}Pa] Density[{:.0}kg/m³]",
                     count + 1, data.temperature_k, data.pressure_pa, data.density_kg_m3);
            count += 1;
        }
        
        sim.current_step += 1;
    }
    
    println!("\n🎉 Event-driven simulation completed!");
    println!("✅ All components processed in parallel");
    println!("✅ All events compressed and applied atomically");
    println!("✅ No direct data modification in components");
}
