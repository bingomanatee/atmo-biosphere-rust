use atmo_biosphere_rust::simulation::{Simulation, SimulationConfig, PlanetConfig, LayerConfig};
use atmo_biosphere_rust::components::{LayerCellComponent, BinaryPairComponent};
use atmo_biosphere_rust::binary_pair::{BinaryPairId, BinaryPair, BinaryPairType};
use h3o::Resolution;

fn main() {
    println!("🔗 Binary Pairs System Example");
    
    // Create simulation with multiple layers for interesting binary pairs
    let config = SimulationConfig {
        planet: PlanetConfig {
            radius_km: 6371.0,
            surface_gravity_m_s_s: 9.81,
        },
        years_per_step: 1000,
        steps: 2,
        layers: vec![
            LayerConfig {
                height_per_step_km: 5.0,   // 5km per step
                depth_steps: 3,            // 3 steps = 15km crust
                resolution: Resolution::Four, // Medium resolution
                name: "Crust".to_string(),
            },
            LayerConfig {
                height_per_step_km: 20.0,  // 20km per step
                depth_steps: 2,            // 2 steps = 40km upper mantle
                resolution: Resolution::Three, // Coarser resolution
                name: "Upper Mantle".to_string(),
            },
        ],
    };
    
    let mut sim = Simulation::new(config);
    sim.initialize_cells();
    
    println!("✅ Simulation initialized with {} cells", sim.get_geological_cells().len());
    
    // Add components
    println!("\n🔧 Adding components:");
    sim.add_component(Box::new(LayerCellComponent::new())); // Initialize geological properties
    sim.add_component(Box::new(BinaryPairComponent::new())); // Build binary pairs
    
    println!("✅ Added {} components", sim.components.len());
    
    // Initialize components (this will build the binary pairs)
    sim.initialize_components();
    
    // Check if binary pairs were created
    if let Some(binary_pairs) = sim.coll_mgr.get::<BinaryPairId, BinaryPair>("binary_pairs") {
        println!("\n🔗 Binary Pairs Analysis:");
        println!("  Total pairs: {}", binary_pairs.len());
        
        // Analyze pair types
        let mut vertical_count = 0;
        let mut horizontal_count = 0;
        let mut total_thermal_conductance = 0.0;
        let mut sample_pairs = Vec::new();
        
        for entry in binary_pairs.iter() {
            let pair = entry.value();
            
            match pair.pair_type {
                BinaryPairType::Vertical => vertical_count += 1,
                BinaryPairType::Horizontal => horizontal_count += 1,
            }
            
            // Calculate thermal conductance (assuming typical rock conductivity)
            let conductivity = 3.0; // W/m/K for typical rock
            let conductance = pair.thermal_conductance(conductivity);
            total_thermal_conductance += conductance;
            
            // Collect sample pairs for detailed analysis
            if sample_pairs.len() < 5 {
                sample_pairs.push(pair.clone());
            }
        }
        
        println!("  Vertical pairs (above/below): {}", vertical_count);
        println!("  Horizontal pairs (H3 neighbors): {}", horizontal_count);
        println!("  Total thermal conductance: {:.2e} W/K", total_thermal_conductance);
        
        if total_thermal_conductance > 0.0 {
            let avg_conductance = total_thermal_conductance / binary_pairs.len() as f64;
            println!("  Average thermal conductance: {:.2e} W/K", avg_conductance);
        }
        
        // Show sample pairs
        println!("\n📋 Sample Binary Pairs:");
        for (i, pair) in sample_pairs.iter().enumerate() {
            let (cell_a, cell_b) = pair.get_cells();
            println!("  Pair {}: {:?}", i + 1, pair.pair_type);
            println!("    Cell A: Layer[{}] Depth[{}]", 
                     cell_a.layer_set_index(), cell_a.depth_index());
            println!("    Cell B: Layer[{}] Depth[{}]", 
                     cell_b.layer_set_index(), cell_b.depth_index());
            println!("    Distance: {:.2} km", pair.distance_km);
            println!("    Contact area: {:.2} km²", pair.contact_area_km2);
            
            let conductivity = 3.0; // W/m/K
            let conductance = pair.thermal_conductance(conductivity);
            println!("    Thermal conductance: {:.2e} W/K", conductance);
            
            // Calculate mass transfer coefficient
            let permeability = 1e-15; // m² (typical rock permeability)
            let viscosity = 1e-3; // Pa·s (water viscosity)
            let mass_coeff = pair.mass_transfer_coefficient(permeability, viscosity);
            println!("    Mass transfer coeff: {:.2e} m³/Pa/s", mass_coeff);
            println!();
        }
        
        // Analyze pair distribution by layer
        println!("📊 Pair Distribution by Layer:");
        let mut layer_pair_counts = std::collections::HashMap::new();
        
        for entry in binary_pairs.iter() {
            let pair = entry.value();
            let (cell_a, cell_b) = pair.get_cells();
            
            let key = if cell_a.layer_set_index() == cell_b.layer_set_index() {
                format!("Layer {} (internal)", cell_a.layer_set_index())
            } else {
                format!("Layer {} ↔ Layer {}", 
                       cell_a.layer_set_index().min(cell_b.layer_set_index()),
                       cell_a.layer_set_index().max(cell_b.layer_set_index()))
            };
            
            *layer_pair_counts.entry(key).or_insert(0) += 1;
        }
        
        for (layer_desc, count) in layer_pair_counts {
            println!("  {}: {} pairs", layer_desc, count);
        }
        
    } else {
        println!("❌ No binary pairs collection found!");
    }
    
    // Run simulation steps
    println!("\n🚀 Running simulation steps...");
    sim.run();
    
    let final_stats = sim.get_stats();
    println!("\n📊 Final Statistics:");
    println!("  Steps completed: {}/{}", final_stats.current_step, final_stats.total_steps);
    println!("  Years simulated: {}", final_stats.years_simulated);
    println!("  Total cells: {}", final_stats.total_cells);
    println!("  Components: {}", sim.components.len());
    
    // Final binary pairs check
    if let Some(binary_pairs) = sim.coll_mgr.get::<BinaryPairId, BinaryPair>("binary_pairs") {
        println!("  Binary pairs: {}", binary_pairs.len());
    }
    
    println!("\n🎉 Binary pairs system demonstration completed!");
    println!("✅ Binary pairs enable efficient geological operations:");
    println!("   - Thermal conduction between neighboring cells");
    println!("   - Mass transfer through geological layers");
    println!("   - Pressure equilibration across boundaries");
    println!("   - Efficient parallel processing of cell interactions");
}
