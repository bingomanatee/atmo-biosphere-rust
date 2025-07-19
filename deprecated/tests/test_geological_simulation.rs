#[cfg(test)]
mod tests {
    use crate::component::conduction_component::ConductionComponent;
    use crate::component::core_radiance_component::CoreRadianceComponent;
    use crate::component::convection_plume_component::ConvectionPlumeComponent;
    use crate::component::SimComponent;
    use crate::sim::simulation::{Simulation, SimulationConfig};
    use crate::sim::layer_set::LayerSetParams;
    use h3o::Resolution;

    /// Create a realistic geological simulation with all components
    fn create_realistic_geological_simulation() -> Simulation {
        println!("🌍 Creating realistic geological simulation...");

        // No thermal gradient config needed

        // Realistic geological layer structure (0-300km depth)
        let layer_params = vec![
            // Surface layer: 0-50km (crust)
            LayerSetParams {
                resolution: Resolution::Two,        // High detail for surface
                start_height_km: 0.0,
                cell_height_km: 25.0,              // 25km thick cells
                material_name: "basalt".to_string(),
                column_count: 2,                   // 50km total depth
                planet_radius_km: 6371.0,
            },
            // Upper mantle: 50-150km
            LayerSetParams {
                resolution: Resolution::One,        // Medium detail
                start_height_km: 50.0,
                cell_height_km: 50.0,              // 50km thick cells
                material_name: "granite".to_string(),
                column_count: 2,                   // 100km total depth
                planet_radius_km: 6371.0,
            },
            // Lower mantle: 150-300km
            LayerSetParams {
                resolution: Resolution::Zero,       // Lower detail for deep layers
                start_height_km: 150.0,
                cell_height_km: 75.0,              // 75km thick cells
                material_name: "basalt".to_string(),
                column_count: 2,                   // 150km total depth
                planet_radius_km: 6371.0,
            },
        ];

        // Simulation configuration
        let config = SimulationConfig {
            steps: 50,                             // 50 steps
            years_per_step: 2000.0,               // 2000 years per step = 100,000 years total
            warmup_steps: 0,
            layer_set_params: layer_params.clone(),
        };

        // Create all realistic geological components
        let mut components: Vec<Box<dyn SimComponent>> = vec![
            Box::new(CoreRadianceComponent::new()),                    // Core energy input
            Box::new(ConductionComponent::new()),                     // Heat conduction
            Box::new(ConvectionPlumeComponent::with_seed(42)),         // Convection plumes
        ];

        let mut sim = Simulation::new(config, &mut components);

        println!("🌍 Realistic geological simulation created:");
        for (idx, params) in layer_params.iter().enumerate() {
            let total_depth = params.column_count as f64 * params.cell_height_km;
            let start = params.start_height_km;
            let end = start + total_depth;
            println!("   Layer {}: {:.0}-{:.0}km ({:.0}km thick), res {}, {} material",
                idx, start, end, total_depth, params.resolution as u8, params.material_name);
        }
        println!("   Total depth: 300km (crust to upper mantle)");
        println!("   Components: Core radiance, Conduction, Convection");

        sim
    }

    /// Generate final geological report in reference format
    fn generate_final_geological_report(sim: &Simulation) {
        // Get the first H3 cell for detailed layer breakdown
        if let Some((first_h3_index, first_column)) = sim.layer_sets.get(0)
            .and_then(|layer_set| layer_set.layers.iter().next()) {
            
            println!("🔬 Final Geological Analysis (Cell {}):", first_h3_index);
            
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

    #[test]
    fn test_realistic_geological_simulation() {
        println!("🧪 Testing realistic geological simulation with all components");
        println!("🎯 Goal: Complete Earth-like geological system over 100,000 years");
        
        // Create realistic simulation
        let mut sim = create_realistic_geological_simulation();
        
        // Initialize simulation
        sim.initialize();
        
        println!("\n🚀 Running geological simulation...");
        println!("   Duration: 100,000 years (50 steps × 2,000 years/step)");
        
        // Run the complete simulation
        for step in 0..50 {
            sim.step();
            
            // Progress indicator every 10 steps
            if step % 10 == 9 {
                println!("   Step {}/50 complete ({} years)", step + 1, sim.current_year());
            }
        }
        
        println!("\n✅ Geological simulation complete!");
        println!("   Final step: {}", sim.current_step());
        println!("   Total time: {} years", sim.current_year());
        
        // Generate final comprehensive report
        generate_final_geological_report(&sim);
        
        println!("\n🎯 Geological simulation test completed successfully!");
    }
}
