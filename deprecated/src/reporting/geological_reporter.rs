use crate::sim_immut::simulation_immut::SimulationImmut;
use crate::energy_mass::energy_mass::EnergyMass;

/// Geological simulation reporting utilities
pub struct GeologicalReporter;

impl GeologicalReporter {
    /// Print comprehensive cell-by-cell thermal analysis
    pub fn print_detailed_thermal_structure(sim: &SimulationImmut, million_years: f64) {
        println!("\n📊 COMPREHENSIVE CELL-BY-CELL THERMAL ANALYSIS at {:.0} Million Years:", million_years);
        println!("================================================");
        println!("| Layer | Cell | Depth | Temp(K) | Temp(°C) | Energy(J)  | Mass(kg)   | Material |");
        println!("|-------|------|-------|---------|----------|------------|------------|----------|");

        let mut total_cells = 0;
        let mut total_energy = 0.0;
        let mut total_mass = 0.0;

        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            if layer_idx >= sim.config.layer_set_params.len() { break; }
            let layer_params = &sim.config.layer_set_params[layer_idx];

            // Get first column for detailed analysis
            if let Some(first_column) = layer_set.layers.values().next() {
                for (cell_idx, cell) in first_column.cells.iter().enumerate() {
                    let depth_km = layer_params.start_height_km +
                                  (cell_idx as f64 * layer_params.cell_height_km);
                    let temp_k = cell.temperature_kelvin();
                    let temp_c = temp_k - 273.15;
                    let energy_j = cell.energy_joules();
                    let mass_kg = cell.mass_kg();

                    total_cells += 1;
                    total_energy += energy_j;
                    total_mass += mass_kg;

                    println!("| {:5} | {:4} | {:5.0} | {:7.1} | {:8.1} | {:10.2e} | {:10.2e} | {:8} |",
                             layer_idx + 1,
                             cell_idx + 1,
                             depth_km,
                             temp_k,
                             temp_c,
                             energy_j,
                             mass_kg,
                             layer_params.material_name);
                }

                // Add separator between layers
                if layer_idx < sim.layer_sets.len() - 1 {
                    println!("|-------|------|-------|---------|----------|------------|------------|----------|");
                }
            }
        }

        println!("|-------|------|-------|---------|----------|------------|------------|----------|");
        println!("| TOTAL | {:4} |       |         |          | {:10.2e} | {:10.2e} |          |",
                 total_cells, total_energy, total_mass);

        // Thermal gradient analysis
        Self::print_thermal_gradient_analysis(sim);
        println!("================================================");
    }

    /// Print thermal gradient analysis
    pub fn print_thermal_gradient_analysis(sim: &SimulationImmut) {
        println!("\n🌡️ THERMAL GRADIENT ANALYSIS:");
        println!("=============================");
        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            if layer_idx >= sim.config.layer_set_params.len() { break; }
            let layer_params = &sim.config.layer_set_params[layer_idx];

            if let Some(first_column) = layer_set.layers.values().next() {
                if first_column.cells.len() >= 2 {
                    let first_cell = &first_column.cells[0];
                    let last_cell = &first_column.cells[first_column.cells.len() - 1];

                    let depth_diff = (first_column.cells.len() - 1) as f64 * layer_params.cell_height_km;
                    let temp_diff = last_cell.temperature_kelvin() - first_cell.temperature_kelvin();
                    let gradient = temp_diff / depth_diff;

                    println!("Layer {}: {:.1}K/km gradient ({:.0}-{:.0}km depth)",
                             layer_idx + 1,
                             gradient,
                             layer_params.start_height_km,
                             layer_params.start_height_km + (first_column.cells.len() as f64 * layer_params.cell_height_km));
                }
            }
        }
    }

    /// Print summary geological state (layer averages)
    pub fn print_geological_state_summary(sim: &SimulationImmut, million_years: f64) {
        println!("\n🌍 GEOLOGICAL STATE at {:.0} Million Years:", million_years);
        println!("=======================================");
        println!("| Layer | Cells | Avg Temp(K) | Total Energy(J) | Material   |");
        println!("|-------|-------|-------------|-----------------|------------|");
        
        let mut total_energy = 0.0;
        
        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            if let Some((_, column)) = layer_set.layers.iter().next() {
                let avg_temp: f64 = column.cells.iter()
                    .map(|cell| cell.temperature_kelvin())
                    .sum::<f64>() / column.cells.len() as f64;

                let layer_energy: f64 = column.cells.iter()
                    .map(|cell| cell.energy_joules())
                    .sum();
                
                total_energy += layer_energy;
                
                let material = match layer_idx {
                    0 => "basalt",
                    1 => "peridotite",
                    2 => "eclogite", 
                    _ => "deep_mantle",
                };
                
                println!("| {:5} | {:5} | {:11.1} | {:13.2e} | {:<10} |",
                         layer_idx + 1, column.cells.len(), avg_temp, layer_energy, material);
            }
        }
        println!("|-------|-------|-------------|-----------------|------------|");
        println!("| TOTAL | {:5} |             | {:13.2e} |            |", sim.total_cells(), total_energy);
    }

    /// Print single column cell details (for debugging specific columns)
    pub fn print_column_details(sim: &SimulationImmut, h3_cell: h3o::CellIndex) {
        println!("\n🔍 COLUMN DETAILS for cell {:?}:", h3_cell);
        println!("================================");
        
        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            if let Some(column) = layer_set.layers.get(&h3_cell) {
                println!("Layer {} ({}):", layer_idx + 1, 
                         if layer_idx < sim.config.layer_set_params.len() {
                             &sim.config.layer_set_params[layer_idx].material_name
                         } else {
                             "unknown"
                         });
                
                for (depth_index, cell) in column.cells.iter().enumerate() {
                    let temp_k = cell.temperature_kelvin();
                    let temp_c = temp_k - 273.15;
                    let top_km = cell.top_km;
                    let center_km = top_km + cell.height_km / 2.0;

                    println!("  Depth {}: {:.1}km center, {:.1}K ({:.1}°C), {:.2e}J, {:.2e}kg",
                           depth_index, center_km, temp_k, temp_c,
                           cell.energy_joules(), cell.mass_kg());
                }
            }
        }
    }
}
