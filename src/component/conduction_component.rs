use crate::component::SimComponent;
use crate::deprecated::sim::Simulation;
use crate::deprecated::sim::energy_mass_cell::EnergyMassCell;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::utils::h3_utils::H3Utils;
use h3o::CellIndex;
use rayon::prelude::*;
use std::collections::HashMap;

/// Enhanced thermal conduction component with neighbor-based heat transfer
/// Handles both horizontal (within layer) and vertical (between layer) conduction
#[derive(Debug)]
pub struct ConductionComponent {
    /// Minimum temperature difference to trigger conduction (K)
    min_temp_difference_k: f64,
    /// Use threading for large simulations
    threading_threshold: usize,
    /// Cached neighbor map: cell_index -> list of neighbor cell_indices (horizontal neighbors)
    /// Pre-computed in prepare() method for performance
    neighbor_cache: HashMap<CellIndex, Vec<CellIndex>>,
    /// Cached vertical neighbor map: (layer_idx, cell_index) -> list of (target_layer_idx, target_cell_index, weight)
    /// Handles resolution differences between layers
    vertical_neighbor_cache: HashMap<(usize, CellIndex), Vec<(usize, CellIndex, f64)>>,
    /// Cache validity flag - set to false when layer structure changes
    cache_valid: bool,
}

/// Data for thermal conduction between layers
/// Enhanced conduction data for neighbor-based heat transfer
#[derive(Debug, Clone)]
struct ConductionData {
    source_layer_idx: usize,
    source_cell_idx: usize,
    source_cell_index: CellIndex,
    target_layer_idx: usize,
    target_cell_idx: usize,
    target_cell_index: CellIndex,
    energy_transfer: f64,
    temp_difference: f64,
    is_vertical: bool, // true for vertical (layer-to-layer), false for horizontal (within layer)
}

impl ConductionComponent {
    /// Create new enhanced conduction component with default parameters
    pub fn new() -> Self {
        Self {
            min_temp_difference_k: 5.0,      // Minimum 5K difference (as specified)
            threading_threshold: 10000,      // Use threading for >10k cells
            neighbor_cache: HashMap::new(),
            vertical_neighbor_cache: HashMap::new(),
            cache_valid: false,
        }
    }

    /// Create conduction component with custom parameters
    pub fn with_parameters(min_temp_difference_k: f64) -> Self {
        Self {
            min_temp_difference_k,
            threading_threshold: 10000,
            neighbor_cache: HashMap::new(),
            vertical_neighbor_cache: HashMap::new(),
            cache_valid: false,
        }
    }

    /// Build neighbor cache for all cells in the simulation
    /// Pre-computes both horizontal and vertical neighbor relationships for performance
    fn build_neighbor_cache(&mut self, sim: &Simulation) {
        println!("🔗 Building enhanced neighbor cache for thermal conduction...");
        self.neighbor_cache.clear();
        self.vertical_neighbor_cache.clear();

        let mut total_horizontal_neighbors = 0;
        let mut total_vertical_neighbors = 0;

        // Build horizontal neighbor cache for each layer
        for (_layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (cell_index, _column) in layer_set.layers.iter() {
                // Get horizontal neighbors (within same layer)
                let horizontal_neighbors = H3Utils::neighbors_for(*cell_index);

                // Filter to only include neighbors that exist in this layer
                let valid_neighbors: Vec<CellIndex> = horizontal_neighbors
                    .into_iter()
                    .filter(|neighbor| layer_set.layers.contains_key(neighbor))
                    .collect();

                total_horizontal_neighbors += valid_neighbors.len();
                self.neighbor_cache.insert(*cell_index, valid_neighbors);
            }
        }

        // Build vertical neighbor cache (resolution-aware)
        for layer_idx in 0..sim.layer_sets.len() {
            let current_layer = &sim.layer_sets[layer_idx];

            for (cell_index, column) in current_layer.layers.iter() {
                for _cell_idx in 0..column.cells.len() {
                    let mut vertical_neighbors = Vec::new();

                    // Check adjacent layers (above and below)
                    for target_layer_idx in 0..sim.layer_sets.len() {
                        if target_layer_idx == layer_idx {
                            continue; // Skip same layer
                        }

                        let target_layer = &sim.layer_sets[target_layer_idx];

                        // Get the resolution of the target layer
                        if let Some((first_cell_index, _)) = target_layer.layers.iter().next() {
                            let target_resolution = first_cell_index.resolution();

                            // Map current cell to target resolution
                            let overlapping_cells = H3Utils::get_overlapping_cells_at_resolution(
                                *cell_index, target_resolution
                            );

                            // Add valid overlapping cells that exist in target layer
                            for (target_cell_index, weight) in overlapping_cells {
                                if target_layer.layers.contains_key(&target_cell_index) {
                                    vertical_neighbors.push((target_layer_idx, target_cell_index, weight));
                                    total_vertical_neighbors += 1;
                                }
                            }
                        }
                    }

                    if !vertical_neighbors.is_empty() {
                        self.vertical_neighbor_cache.insert((layer_idx, *cell_index), vertical_neighbors);
                    }
                }
            }
        }

        self.cache_valid = true;
        println!("🔗 Enhanced neighbor cache built:");
        println!("   Horizontal: {} cells, {} neighbor relationships",
            self.neighbor_cache.len(), total_horizontal_neighbors);
        println!("   Vertical: {} cell-layer pairs, {} neighbor relationships",
            self.vertical_neighbor_cache.len(), total_vertical_neighbors);
    }

    /// Get horizontal neighbors for a cell, using cache if available
    fn get_neighbors(&self, cell_index: CellIndex) -> Vec<CellIndex> {
        self.neighbor_cache.get(&cell_index).cloned().unwrap_or_default()
    }

    /// Get vertical neighbors for a cell, using cache if available
    /// Returns (target_layer_idx, target_cell_index, weight) tuples
    fn get_vertical_neighbors(&self, layer_idx: usize, cell_index: CellIndex) -> Vec<(usize, CellIndex, f64)> {
        self.vertical_neighbor_cache.get(&(layer_idx, cell_index)).cloned().unwrap_or_default()
    }

    /// Calculate enhanced thermal conduction (horizontal + vertical)
    fn calculate_conduction(&mut self, sim: &mut Simulation, years_per_step: f64) {
        // Ensure neighbor cache is built
        if !self.cache_valid {
            self.build_neighbor_cache(sim);
        }

        // Count total cells to decide on threading
        let total_cells: usize = sim.layer_sets.iter()
            .map(|layer| layer.layers.len() * layer.layers.values().next().map_or(1, |col| col.cells.len()))
            .sum();

        if total_cells > self.threading_threshold {
            self.calculate_conduction_threaded(sim, years_per_step);
        } else {
            self.calculate_conduction_sequential(sim, years_per_step);
        }
    }
    
    /// Enhanced sequential conduction calculation (horizontal + vertical)
    fn calculate_conduction_sequential(&mut self, sim: &mut Simulation, years_per_step: f64) {
        let mut conduction_data = Vec::new();

        // Step 1: Collect all cell data for neighbor-based conduction
        let mut all_cells = Vec::new();
        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (cell_index, column) in layer_set.layers.iter() {
                for (cell_idx, cell) in column.cells.iter().enumerate() {
                    all_cells.push((layer_idx, cell_idx, *cell_index, cell));
                }
            }
        }

        // Step 2: Process horizontal conduction (within layers)
        for (layer_idx, cell_idx, cell_index, cell) in &all_cells {
            let neighbors = self.get_neighbors(*cell_index);

            for neighbor_index in neighbors {
                // Find the neighbor cell in the same layer
                if let Some(neighbor_data) = all_cells.iter().find(|(l_idx, c_idx, c_index, _)|
                    *l_idx == *layer_idx && *c_index == neighbor_index) {

                    let (_, neighbor_cell_idx, _, neighbor_cell) = neighbor_data;

                    // Only process if this cell is hotter (to avoid duplicate processing)
                    if cell.temperature_kelvin() > neighbor_cell.temperature_kelvin() {
                        if let Some(conduction) = self.calculate_neighbor_conduction(
                            cell, neighbor_cell, *layer_idx, *layer_idx,
                            *cell_idx, *neighbor_cell_idx, *cell_index, neighbor_index,
                            years_per_step, false // horizontal
                        ) {
                            conduction_data.push(conduction);
                        }
                    }
                }
            }
        }

        // Step 3: Process vertical conduction (between layers)
        for (layer_idx, cell_idx, cell_index, cell) in &all_cells {
            let vertical_neighbors = self.get_vertical_neighbors(*layer_idx, *cell_index);

            for (target_layer_idx, target_cell_index, _weight) in vertical_neighbors {
                // Find the target cell
                if let Some(target_data) = all_cells.iter().find(|(l_idx, c_idx, c_index, _)|
                    *l_idx == target_layer_idx && *c_index == target_cell_index && *c_idx == *cell_idx) {

                    let (_, target_cell_idx, _, target_cell) = target_data;

                    // Only process if this cell is hotter
                    if cell.temperature_kelvin() > target_cell.temperature_kelvin() {
                        if let Some(conduction) = self.calculate_neighbor_conduction(
                            cell, target_cell, *layer_idx, target_layer_idx,
                            *cell_idx, *target_cell_idx, *cell_index, target_cell_index,
                            years_per_step, true // vertical
                        ) {
                            conduction_data.push(conduction);
                        }
                    }
                }
            }
        }

        println!("🌡️ Enhanced conduction: {} thermal transfers calculated", conduction_data.len());

        // Apply conduction effects
        self.apply_conduction_data(sim, conduction_data);
    }
    
    /// Enhanced threaded conduction calculation (horizontal + vertical)
    fn calculate_conduction_threaded(&mut self, sim: &mut Simulation, years_per_step: f64) {
        println!("🧵 Using threaded enhanced thermal conduction");

        // Step 1: Collect all cell data for neighbor-based conduction
        let mut all_cells = Vec::new();
        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (cell_index, column) in layer_set.layers.iter() {
                for (cell_idx, cell) in column.cells.iter().enumerate() {
                    all_cells.push((layer_idx, cell_idx, *cell_index, cell));
                }
            }
        }

        // Step 2: Process horizontal conduction in parallel
        let horizontal_pairs: Vec<_> = all_cells
            .par_iter()
            .flat_map(|(layer_idx, cell_idx, cell_index, cell)| {
                let neighbors = self.get_neighbors(*cell_index);
                let mut pairs = Vec::new();

                for neighbor_index in neighbors {
                    // Find the neighbor cell in the same layer
                    if let Some(neighbor_data) = all_cells.iter().find(|(l_idx, c_idx, c_index, _)|
                        *l_idx == *layer_idx && *c_index == neighbor_index) {

                        let (_, neighbor_cell_idx, _, neighbor_cell) = neighbor_data;

                        // Only process if this cell is hotter (to avoid duplicate processing)
                        if cell.temperature_kelvin() > neighbor_cell.temperature_kelvin() {
                            pairs.push((
                                *layer_idx, *cell_idx, *cell_index, cell.clone(),
                                *layer_idx, *neighbor_cell_idx, neighbor_index, neighbor_cell.clone(),
                                false // horizontal
                            ));
                        }
                    }
                }
                pairs
            })
            .collect();

        // Step 3: Process vertical conduction in parallel
        let vertical_pairs: Vec<_> = all_cells
            .par_iter()
            .flat_map(|(layer_idx, cell_idx, cell_index, cell)| {
                let vertical_neighbors = self.get_vertical_neighbors(*layer_idx, *cell_index);
                let mut pairs = Vec::new();

                for (target_layer_idx, target_cell_index, weight) in vertical_neighbors {
                    // Find the target cell
                    if let Some(target_data) = all_cells.iter().find(|(l_idx, c_idx, c_index, _)|
                        *l_idx == target_layer_idx && *c_index == target_cell_index && *c_idx == *cell_idx) {

                        let (_, target_cell_idx, _, target_cell) = target_data;

                        // Only process if this cell is hotter
                        if cell.temperature_kelvin() > target_cell.temperature_kelvin() {
                            pairs.push((
                                *layer_idx, *cell_idx, *cell_index, cell.clone(),
                                target_layer_idx, *target_cell_idx, target_cell_index, target_cell.clone(),
                                true // vertical
                            ));
                        }
                    }
                }
                pairs
            })
            .collect();

        // Step 4: Calculate conduction for all pairs in parallel
        let mut all_pairs = horizontal_pairs;
        all_pairs.extend(vertical_pairs);

        let conduction_data: Vec<_> = all_pairs
            .into_par_iter()
            .filter_map(|(src_layer, src_cell_idx, src_cell_index, src_cell,
                         tgt_layer, tgt_cell_idx, tgt_cell_index, tgt_cell, is_vertical)| {
                Self::calculate_neighbor_conduction_static(
                    &src_cell, &tgt_cell, src_layer, tgt_layer,
                    src_cell_idx, tgt_cell_idx, src_cell_index, tgt_cell_index,
                    years_per_step, is_vertical, self.min_temp_difference_k
                )
            })
            .collect();

        println!("🧵 Enhanced threaded conduction: {} thermal transfers calculated", conduction_data.len());

        // Apply conduction effects
        self.apply_conduction_data(sim, conduction_data);
    }
    
    /// Calculate enhanced neighbor-based conduction between two cells
    fn calculate_neighbor_conduction(
        &self,
        hot_cell: &EnergyMassCell,
        cold_cell: &EnergyMassCell,
        source_layer_idx: usize,
        target_layer_idx: usize,
        source_cell_idx: usize,
        target_cell_idx: usize,
        source_cell_index: CellIndex,
        target_cell_index: CellIndex,
        years_per_step: f64,
        is_vertical: bool,
    ) -> Option<ConductionData> {
        let hot_temp = hot_cell.temperature_kelvin();
        let cold_temp = cold_cell.temperature_kelvin();
        let temp_diff = hot_temp - cold_temp;

        // Only conduct if temperature difference is significant (> 5K as specified)
        // AND hot cell is actually hotter than cold cell
        if temp_diff >= self.min_temp_difference_k {
            // Calculate simple conductivity-based transfer
            let timestep_seconds = years_per_step * 365.25 * 24.0 * 3600.0;

            // Simple conductivity calculation without requiring mutable access
            let hot_conductivity = hot_cell.calculate_pressure_adjusted_conductivity_w_m_k();
            let cold_conductivity = cold_cell.calculate_pressure_adjusted_conductivity_w_m_k();
            let interface_conductivity = (hot_conductivity + cold_conductivity) / 2.0;

            // Simple area and distance calculation
            let contact_area = hot_cell.area() * 1e6; // Convert km² to m²
            let distance = if is_vertical { hot_cell.height_km * 1000.0 } else { 1000.0 }; // 1km for horizontal

            // Calculate conductance coefficient (always positive)
            let conductance_coefficient = interface_conductivity * contact_area / distance * timestep_seconds;
            let conductivity_transfer = conductance_coefficient * temp_diff; // temp_diff is positive

            // Calculate maximum allowed transfer (half the energy difference, but only if positive)
            let hot_energy = hot_cell.energy_joules();
            let cold_energy = cold_cell.energy_joules();
            let energy_difference = hot_energy - cold_energy;
            let max_transfer = if energy_difference > 0.0 { energy_difference * 0.5 } else { 0.0 };

            // Use the smaller of the two: conductivity-based or half-difference limit
            let energy_transfer = conductivity_transfer.min(max_transfer);

            // Only create conduction data if energy transfer is positive
            if energy_transfer > 0.0 && energy_transfer.is_finite() {
                Some(ConductionData {
                    source_layer_idx,
                    source_cell_idx,
                    source_cell_index,
                    target_layer_idx,
                    target_cell_idx,
                    target_cell_index,
                    energy_transfer,
                    temp_difference: temp_diff,
                    is_vertical,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Static version for parallel processing of neighbor-based conduction
    fn calculate_neighbor_conduction_static(
        hot_cell: &EnergyMassCell,
        cold_cell: &EnergyMassCell,
        source_layer_idx: usize,
        target_layer_idx: usize,
        source_cell_idx: usize,
        target_cell_idx: usize,
        source_cell_index: CellIndex,
        target_cell_index: CellIndex,
        years_per_step: f64,
        is_vertical: bool,
        min_temp_difference_k: f64,
    ) -> Option<ConductionData> {
        let hot_temp = hot_cell.temperature_kelvin();
        let cold_temp = cold_cell.temperature_kelvin();
        let temp_diff = hot_temp - cold_temp;

        // Only conduct if temperature difference is significant (> 5K as specified)
        // AND hot cell is actually hotter than cold cell
        if temp_diff >= min_temp_difference_k {
            // Calculate simple conductivity-based transfer
            let timestep_seconds = years_per_step * 365.25 * 24.0 * 3600.0;

            // Simple conductivity calculation without requiring mutable access
            let hot_conductivity = hot_cell.calculate_pressure_adjusted_conductivity_w_m_k();
            let cold_conductivity = cold_cell.calculate_pressure_adjusted_conductivity_w_m_k();
            let interface_conductivity = (hot_conductivity + cold_conductivity) / 2.0;

            // Simple area and distance calculation
            let contact_area = hot_cell.area() * 1e6; // Convert km² to m²
            let distance = if is_vertical { hot_cell.height_km * 1000.0 } else { 1000.0 }; // 1km for horizontal

            // Calculate conductance coefficient (always positive)
            let conductance_coefficient = interface_conductivity * contact_area / distance * timestep_seconds;
            let conductivity_transfer = conductance_coefficient * temp_diff; // temp_diff is positive

            // Calculate maximum allowed transfer (half the energy difference, but only if positive)
            let hot_energy = hot_cell.energy_joules();
            let cold_energy = cold_cell.energy_joules();
            let energy_difference = hot_energy - cold_energy;
            let max_transfer = if energy_difference > 0.0 { energy_difference * 0.5 } else { 0.0 };

            // Use the smaller of the two: conductivity-based or half-difference limit
            let energy_transfer = conductivity_transfer.min(max_transfer);

            // Only create conduction data if energy transfer is positive
            if energy_transfer > 0.0 && energy_transfer.is_finite() {
                Some(ConductionData {
                    source_layer_idx,
                    source_cell_idx,
                    source_cell_index,
                    target_layer_idx,
                    target_cell_idx,
                    target_cell_index,
                    energy_transfer,
                    temp_difference: temp_diff,
                    is_vertical,
                })
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Cool surface cells to initial temperature (simulates radiative cooling to space)
    fn apply_surface_cooling(&self, sim: &mut Simulation) {
        if sim.layer_sets.is_empty() {
            return;
        }

        // Get initial surface temperature before borrowing layer_sets mutably
        let initial_temp = sim.config.surface_temp_k;

        let surface_layer = &mut sim.layer_sets[0]; // Topmost layer
        let mut cells_cooled = 0;
        let mut total_energy_removed = 0.0;

        for (cell_index, column) in surface_layer.layers.iter_mut() {
            if let Some(surface_cell) = column.cells.first_mut() {
                let current_temp = surface_cell.temperature_kelvin();

                // Only cool if current temperature is higher than initial
                if current_temp > initial_temp {
                    // Calculate energy needed to reach initial temperature
                    let mass = surface_cell.mass_kg();
                    let specific_heat = surface_cell.material().specific_heat_capacity_j_per_kg_k as f64;
                    let target_energy = initial_temp * mass * specific_heat;
                    let current_energy = surface_cell.energy_joules();

                    if target_energy < current_energy {
                        let energy_to_remove = current_energy - target_energy;
                        surface_cell.remove_energy_joules(energy_to_remove);
                        total_energy_removed += energy_to_remove;
                        cells_cooled += 1;
                    }
                }
            }
        }

        if cells_cooled > 0 {
            println!("❄️  Surface cooling: {} cells cooled, {:.2e} J radiated to space",
                cells_cooled, total_energy_removed);
        }
    }

    /// Apply enhanced conduction data to transfer energy between cells
    fn apply_conduction_data(&mut self, sim: &mut Simulation, conduction_data: Vec<ConductionData>) {
        let mut total_energy_transferred = 0.0;
        let mut horizontal_transfers = 0;
        let mut vertical_transfers = 0;

        for conduction in conduction_data {
            // Safety check: only process positive energy transfers (hotter → cooler)
            if conduction.energy_transfer <= 0.0 {
                continue; // Skip invalid transfers
            }

            // Remove energy from source (hot) cell
            if let Some(source_layer) = sim.layer_sets.get_mut(conduction.source_layer_idx) {
                if let Some(source_column) = source_layer.layers.get_mut(&conduction.source_cell_index) {
                    if let Some(source_cell) = source_column.cells.get_mut(conduction.source_cell_idx) {
                        // Ensure we don't remove more energy than available
                        let available_energy = source_cell.energy_joules();
                        let actual_transfer = conduction.energy_transfer.min(available_energy * 0.9); // Leave 10% buffer

                        if actual_transfer > 0.0 {
                            source_cell.remove_energy_joules(actual_transfer);
                            total_energy_transferred += actual_transfer;

                            // Add energy to target (cold) cell
                            if let Some(target_layer) = sim.layer_sets.get_mut(conduction.target_layer_idx) {
                                if let Some(target_column) = target_layer.layers.get_mut(&conduction.target_cell_index) {
                                    if let Some(target_cell) = target_column.cells.get_mut(conduction.target_cell_idx) {
                                        target_cell.add_energy_joules(actual_transfer);
                                    }
                                }
                            }
                        }

                        // Track transfer types
                        if conduction.is_vertical {
                            vertical_transfers += 1;
                        } else {
                            horizontal_transfers += 1;
                        }
                    }
                }
            }
        }

        if total_energy_transferred > 0.0 {
            println!("🌡️ Enhanced thermal conduction: {:.2e} J transferred", total_energy_transferred);
            println!("   Horizontal: {} transfers, Vertical: {} transfers", horizontal_transfers, vertical_transfers);
        }
    }
}

impl SimComponent for ConductionComponent {
    fn key(&self) -> &'static str {
        "thermal_conduction"
    }

    fn initialize(&mut self, sim: &mut Simulation) {
        println!("🌡️ Enhanced Thermal Conduction Component initialized");
        println!("   - Min temperature difference: {:.1}K", self.min_temp_difference_k);
        println!("   - Threading threshold: {} cells", self.threading_threshold);

        // Count total cells for threading decision preview
        let total_cells: usize = sim.layer_sets.iter()
            .map(|layer| layer.layers.len() * layer.layers.values().next().map_or(1, |col| col.cells.len()))
            .sum();

        if total_cells > self.threading_threshold {
            println!("   - Will use threaded processing ({} cells)", total_cells);
        } else {
            println!("   - Will use sequential processing ({} cells)", total_cells);
        }

        // Build neighbor cache during initialization
        self.build_neighbor_cache(sim);
    }

    fn step(&mut self, sim: &mut Simulation, step: i64, _year: i64) {
        let years_per_step = sim.years_per_step();

        // Apply thermal conduction between layers
        {
            let start = std::time::Instant::now();
            self.calculate_conduction(sim, years_per_step);
            let duration = start.elapsed();
            // Profiling now handled by event system
            println!("⏱️  thermal_conduction: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }

        // Apply surface cooling (radiative cooling to space)
        {
            let start = std::time::Instant::now();
            self.apply_surface_cooling(sim);
            let duration = start.elapsed();
            // Profiling now handled by event system
        }

        // Report status periodically
        if step % 100 == 0 {
            println!("🌡️ Thermal Conduction (Step {}): Processing layer temperature gradients", step);
        }
    }

    fn complete(&mut self, _sim: &Simulation) {
        println!("🌡️ Thermal Conduction Component completed");
    }
}

impl Default for ConductionComponent {
    fn default() -> Self {
        Self::new()
    }
}
