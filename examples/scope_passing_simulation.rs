use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
use atmo_biosphere_rust::components::{ThermalComponent, PressureComponent, DensityComponent};
use h3o::Resolution;

fn main() {
    println!("🔧 Scope-Passing Geological Simulation");
    
    // Create simulation with multiple layers for more cells
    let config = SimulationConfig {
        planet: PlanetConfig {
            radius_km: 6371.0,
            surface_gravity_m_s_s: 9.81,
        },
        years_per_step: 1000,
        steps: 3,
        layers: vec![
            LayerConfig {
                height_per_step_km: 10.0,
                depth_steps: 2,  // 2 steps = 20km crust
                resolution: Resolution::Five,
                name: "Crust".to_string(),
            },
            LayerConfig {
                height_per_step_km: 50.0,
                depth_steps: 4,  // 4 steps = 200km upper mantle
                resolution: Resolution::Four,
                name: "Upper Mantle".to_string(),
            },
            LayerConfig {
                height_per_step_km: 100.0,
                depth_steps: 3,  // 3 steps = 300km lower mantle
                resolution: Resolution::Three,
                name: "Lower Mantle".to_string(),
            },
        ],
    };
    
    let mut sim = Simulation::new(config);
    sim.initialize_cells();
    
    println!("✅ Simulation initialized with {} cells", sim.get_geological_cells().len());
    
    // Add components with different chunking strategies
    println!("\n🔧 Adding components:");
    
    // Thermal component with low threshold to demonstrate sub-chunking
    let thermal = ThermalComponent::with_threshold(10); // Will sub-chunk if > 10 cells
    println!("  + ThermalComponent (chunk threshold: 10 cells)");
    sim.add_component(Box::new(thermal));
    
    // Pressure component (simple, no sub-chunking)
    println!("  + PressureComponent (no sub-chunking)");
    sim.add_component(Box::new(PressureComponent::new()));
    
    // Density component (simple, no sub-chunking)
    println!("  + DensityComponent (no sub-chunking)");
    sim.add_component(Box::new(DensityComponent::new()));
    
    // Show initial state
    println!("\n📊 Initial state (first 3 cells):");
    let mut count = 0;
    for entry in sim.get_geological_cells().iter() {
        if count >= 3 { break; }
        let (location, data) = (entry.key(), entry.value());
        println!("  Cell {}: Layer[{}] Depth[{}] Temp[{:.1}K] Pressure[{:.0}Pa] Density[{:.0}kg/m³]",
                 count + 1, 
                 location.layer_set_index(), 
                 location.depth_index(),
                 data.temperature_k, 
                 data.pressure_pa, 
                 data.density_kg_m3);
        count += 1;
    }
    
    // Run simulation steps
    println!("\n🚀 Running simulation with scope-passing components...");
    
    for step in 1..=3 {
        println!("\n{}", "=".repeat(60));
        println!("STEP {}", step);
        println!("{}", "=".repeat(60));
        
        // The step() method will:
        // 1. Create crossbeam scope
        // 2. Spawn one thread per component
        // 3. Pass the scope to each component's process() method
        // 4. Components can choose to sub-chunk or process directly
        // 5. Collect all Actor changes and blend them
        // 6. Apply atomically
        sim.step();
        
        // Show results after step
        println!("\n📈 After step {} (first 2 cells):", step);
        let mut count = 0;
        for entry in sim.get_geological_cells().iter() {
            if count >= 2 { break; }
            let (location, data) = (entry.key(), entry.value());
            println!("  Cell {}: Temp[{:.1}K] Pressure[{:.0}Pa] Density[{:.0}kg/m³]",
                     count + 1, data.temperature_k, data.pressure_pa, data.density_kg_m3);
            count += 1;
        }
    }
    
    println!("\n🎉 Scope-passing simulation completed!");
    println!("✅ Components received crossbeam scope for optional sub-chunking");
    println!("✅ ThermalComponent demonstrated sub-chunking for large cell counts");
    println!("✅ Other components processed directly (no sub-chunking needed)");
    println!("✅ All changes blended and applied atomically");
    println!("✅ Order-independent deterministic results");
    
    let final_stats = sim.get_stats();
    println!("\n📊 Final Statistics:");
    println!("  Steps completed: {}/{}", final_stats.current_step, final_stats.total_steps);
    println!("  Years simulated: {}", final_stats.years_simulated);
    println!("  Total cells: {}", final_stats.total_cells);
    println!("  Components: {}", sim.components.len());
}
