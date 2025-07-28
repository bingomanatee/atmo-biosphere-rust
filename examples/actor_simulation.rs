use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
use atmo_biosphere_rust::collections::{Actor, ChangeController};
use atmo_biosphere_rust::cell_location::CellLocation;
use h3o::Resolution;

/// Thermal component using Actor pattern
struct ThermalComponent;

impl ThermalComponent {
    fn process(&self, manager: &atmo_biosphere_rust::collections::CollectionsManager, actor: &mut Actor) {
        let cells = manager.get::<CellLocation, atmo_biosphere_rust::simulation::GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        
        for entry in cells.iter() {
            let (location, cell_data) = (entry.key(), entry.value());
            
            // Calculate thermal changes (READ-ONLY access to cell_data)
            let temp_delta = if location.layer_set_index() == 0 {
                // Surface layer: solar heating
                5.0 + (cell_data.temperature_k - 300.0) * 0.01 // Temperature-dependent
            } else {
                // Deeper layers: geothermal heating
                2.0 + location.depth_index() as f64 * 0.5
            };
            
            // ADD to actor's change queue
            actor.add("GEOLOGICAL_CELLS", *location, "temperature_k", temp_delta);
            
            // Also add energy based on temperature change
            let energy_delta = temp_delta * 1000.0;
            actor.add("GEOLOGICAL_CELLS", *location, "energy_joules", energy_delta);
        }
    }
}

/// Pressure component using Actor pattern
struct PressureComponent;

impl PressureComponent {
    fn process(&self, manager: &atmo_biosphere_rust::collections::CollectionsManager, actor: &mut Actor) {
        let cells = manager.get::<CellLocation, atmo_biosphere_rust::simulation::GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        
        for entry in cells.iter() {
            let (location, cell_data) = (entry.key(), entry.value());
            
            // Calculate pressure changes based on depth and temperature
            let depth_factor = (location.depth_index() + 1) as f64;
            let temp_factor = cell_data.temperature_k / 300.0; // Normalized temperature
            let pressure_delta = depth_factor * 1000.0 * temp_factor;
            
            // ADD to actor's change queue
            actor.add("GEOLOGICAL_CELLS", *location, "pressure_pa", pressure_delta);
        }
    }
}

/// Density component using Actor pattern
struct DensityComponent;

impl DensityComponent {
    fn process(&self, manager: &atmo_biosphere_rust::collections::CollectionsManager, actor: &mut Actor) {
        let cells = manager.get::<CellLocation, atmo_biosphere_rust::simulation::GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        
        for entry in cells.iter() {
            let (location, cell_data) = (entry.key(), entry.value());
            
            // Calculate new density based on temperature and pressure
            let base_density = 2500.0;
            let temp_factor = 1.0 - (cell_data.temperature_k - 300.0) / 10000.0;
            let pressure_factor = 1.0 + (cell_data.pressure_pa - 101325.0) / 1000000.0;
            let new_density = base_density * temp_factor * pressure_factor;
            
            // REPLACE (set absolute value) in actor's change queue
            actor.replace("GEOLOGICAL_CELLS", *location, "density_kg_m3", new_density);
        }
    }
}

fn main() {
    println!("🎭 Actor-Based Geological Simulation");
    
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
    
    // Run simulation steps with Actor pattern
    for step in 1..=3 {
        println!("\n🚀 Step {}: Processing components with Actor pattern...", step);
        
        // Each component gets its own Actor
        let (actor1, actor2, actor3) = crossbeam::scope(|s| {
            let manager = &sim.coll_mgr;
            
            let handle1 = s.spawn(|_| {
                let mut actor = Actor::new();
                println!("  🔥 Thermal component processing...");
                thermal_component.process(manager, &mut actor);
                println!("    Thermal: {} changes queued", actor.change_count());
                actor
            });
            
            let handle2 = s.spawn(|_| {
                let mut actor = Actor::new();
                println!("  💨 Pressure component processing...");
                pressure_component.process(manager, &mut actor);
                println!("    Pressure: {} changes queued", actor.change_count());
                actor
            });
            
            let handle3 = s.spawn(|_| {
                let mut actor = Actor::new();
                println!("  🪨 Density component processing...");
                density_component.process(manager, &mut actor);
                println!("    Density: {} changes queued", actor.change_count());
                actor
            });
            
            (handle1.join().unwrap(), handle2.join().unwrap(), handle3.join().unwrap())
        }).unwrap();
        
        // Blend all actor change queues with compression
        println!("  🔄 Blending actor changes...");
        let total_changes_before = actor1.change_count() + actor2.change_count() + actor3.change_count();
        
        let blended_changes = ChangeController::blend(vec![actor1, actor2, actor3]);
        
        println!("    Compressed {} changes into {} optimized changes", 
                 total_changes_before, blended_changes.len());
        
        // Apply blended changes to collections manager
        println!("  📝 Applying blended changes atomically...");
        sim.coll_mgr.apply_events(blended_changes).unwrap();
        
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
    
    println!("\n🎉 Actor-based simulation completed!");
    println!("✅ Each component had its own Actor with change queue");
    println!("✅ All Actor changes blended with automatic compression");
    println!("✅ Order-independent deterministic results");
    println!("✅ No direct data modification - only read-only access");
}
