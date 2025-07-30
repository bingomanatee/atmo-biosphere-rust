use crate::binary_pair::BinaryPairType;
use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::simulation::{GeologicalCellData, SimulationConfig};
use crate::utils::h3_utils::H3Utils;
use rayon::prelude::*;
use std::collections::HashMap;

/// Efficient parallel cell processor that eliminates pre-computed binary pairs
/// Instead, it processes cells in chunks and computes neighbors on-demand
#[derive(Clone)]
pub struct ParallelCellProcessor {
    /// Cached neighbor relationships per cell
    neighbor_cache: HashMap<CellLocation, Vec<(CellLocation, BinaryPairType)>>,
    /// Performance tracking
    total_cells_processed: u64,
    total_neighbor_calculations: u64,
}

impl ParallelCellProcessor {
    /// Create new parallel cell processor
    pub fn new() -> Self {
        Self {
            neighbor_cache: HashMap::new(),
            total_cells_processed: 0,
            total_neighbor_calculations: 0,
        }
    }
    
    /// Initialize neighbor cache for all cells (done once at startup)
    pub fn initialize_neighbor_cache(&mut self, coll_mgr: &CollectionsManager) {
        let cells = match coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells") {
            Some(cells) => cells,
            None => {
                println!("⚠️  No geological cells found for neighbor cache initialization");
                return;
            }
        };
        
        println!("🔗 Initializing neighbor cache for {} cells...", cells.len());
        
        // Create a set of all existing cell locations for fast lookup
        let existing_cells: std::collections::HashSet<CellLocation> = cells.iter()
            .map(|entry| *entry.key())
            .collect();
        
        // Group cells by H3 for efficient vertical neighbor lookup
        let mut cells_by_h3: HashMap<h3o::CellIndex, Vec<CellLocation>> = HashMap::new();
        for cell_location in existing_cells.iter() {
            cells_by_h3.entry(cell_location.h3_cell_index())
                .or_insert_with(Vec::new)
                .push(*cell_location);
        }
        
        // Process cells in parallel chunks to build neighbor cache
        let cell_locations: Vec<CellLocation> = existing_cells.into_iter().collect();
        let chunk_size = (cell_locations.len() / rayon::current_num_threads()).max(1000);
        
        println!("   📊 Processing {} cells in chunks of {} across {} threads", 
                 cell_locations.len(), chunk_size, rayon::current_num_threads());
        
        // Process chunks in parallel
        let neighbor_results: Vec<HashMap<CellLocation, Vec<(CellLocation, BinaryPairType)>>> = 
            cell_locations.par_chunks(chunk_size)
                .map(|chunk| {
                    let mut local_neighbors = HashMap::new();
                    
                    for &cell_location in chunk {
                        let mut neighbors = Vec::new();
                        
                        // 1. Find vertical neighbors (same H3, different layer/depth)
                        if let Some(same_h3_cells) = cells_by_h3.get(&cell_location.h3_cell_index()) {
                            for &other_cell in same_h3_cells {
                                if other_cell != cell_location {
                                    // Check if it's a vertical neighbor
                                    if Self::is_vertical_neighbor(&cell_location, &other_cell) {
                                        neighbors.push((other_cell, BinaryPairType::Vertical));
                                    }
                                }
                            }
                        }
                        
                        // 2. Find horizontal neighbors (different H3, same layer/depth)
                        let h3_neighbors = H3Utils::get_neighbors(cell_location.h3_cell_index());
                        for neighbor_h3 in h3_neighbors {
                            let neighbor_cell = CellLocation::new(
                                cell_location.layer_set_index(),
                                neighbor_h3,
                                cell_location.depth_index()
                            );
                            
                            // Only add if the neighbor cell actually exists
                            if cell_locations.contains(&neighbor_cell) {
                                neighbors.push((neighbor_cell, BinaryPairType::Horizontal));
                            }
                        }
                        
                        local_neighbors.insert(cell_location, neighbors);
                    }
                    
                    local_neighbors
                })
                .collect();
        
        // Merge results from all threads
        for local_result in neighbor_results {
            self.neighbor_cache.extend(local_result);
        }
        
        let total_neighbor_relationships: usize = self.neighbor_cache.values()
            .map(|neighbors| neighbors.len())
            .sum();
        
        println!("✅ Neighbor cache initialized:");
        println!("   - Cells with neighbors: {}", self.neighbor_cache.len());
        println!("   - Total neighbor relationships: {}", total_neighbor_relationships);
        println!("   - Average neighbors per cell: {:.1}", 
                 total_neighbor_relationships as f64 / self.neighbor_cache.len() as f64);
    }
    
    /// Process all cells in parallel chunks for a simulation step
    pub fn process_cells_parallel(
        &mut self,
        coll_mgr: &CollectionsManager,
        actor: &mut Actor,
        step: u32,
        year: f64,
        config: &SimulationConfig,
        component_name: &str,
    ) {
        let cells = match coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells") {
            Some(cells) => cells,
            None => return,
        };
        
        if step % 1000 == 0 {
            println!("🔗 {} processing {} cells in parallel (step {})", 
                     component_name, cells.len(), step);
        }
        
        // Convert to vector for parallel processing
        let cell_entries: Vec<_> = cells.iter().collect();
        let chunk_size = (cell_entries.len() / rayon::current_num_threads()).max(100);
        
        // Process cells in parallel chunks
        let results: Vec<Vec<(CellLocation, CellLocation, BinaryPairType, f64)>> = 
            cell_entries.par_chunks(chunk_size)
                .map(|chunk| {
                    let mut local_transfers = Vec::new();
                    
                    for entry in chunk {
                        let cell_location = *entry.key();
                        let cell_data = entry.value();
                        
                        // Get cached neighbors for this cell
                        if let Some(neighbors) = self.neighbor_cache.get(&cell_location) {
                            for &(neighbor_location, relationship_type) in neighbors {
                                // Get neighbor cell data
                                if let Some(neighbor_data) = cells.get(&neighbor_location) {
                                    // Calculate energy transfer based on component type
                                    let energy_transfer = Self::calculate_energy_transfer(
                                        &*cell_data, &*neighbor_data, relationship_type, config
                                    );
                                    
                                    if energy_transfer.abs() > 1e6 {
                                        local_transfers.push((
                                            cell_location, 
                                            neighbor_location, 
                                            relationship_type, 
                                            energy_transfer
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    
                    local_transfers
                })
                .collect();
        
        // Apply all transfers via actor (sequential to avoid conflicts)
        let mut total_transfers = 0;
        for chunk_results in results {
            for (source_cell, target_cell, _relationship, energy_transfer) in chunk_results {
                if energy_transfer > 0.0 {
                    // Source is hotter, transfers to target
                    actor.add("geological_cells", source_cell, "energy_joules", -energy_transfer);
                    actor.add("geological_cells", target_cell, "energy_joules", energy_transfer);
                } else {
                    // Target is hotter, transfers to source
                    actor.add("geological_cells", target_cell, "energy_joules", energy_transfer);
                    actor.add("geological_cells", source_cell, "energy_joules", -energy_transfer);
                }
                total_transfers += 1;
            }
        }
        
        self.total_cells_processed += cells.len() as u64;
        self.total_neighbor_calculations += total_transfers as u64;
        
        if total_transfers > 0 && step % 1000 == 0 {
            println!("🔗 {} calculated {} energy transfers at step {}", 
                     component_name, total_transfers, step);
        }
    }
    
    /// Check if two cells are vertical neighbors
    fn is_vertical_neighbor(cell_a: &CellLocation, cell_b: &CellLocation) -> bool {
        // Same H3 cell, but different layer or depth
        cell_a.h3_cell_index() == cell_b.h3_cell_index() &&
        (cell_a.layer_set_index() != cell_b.layer_set_index() || 
         cell_a.depth_index() != cell_b.depth_index())
    }
    
    /// Calculate energy transfer between two cells (simplified for now)
    fn calculate_energy_transfer(
        source_data: &GeologicalCellData,
        target_data: &GeologicalCellData,
        relationship_type: BinaryPairType,
        config: &SimulationConfig,
    ) -> f64 {
        // Stefan-Boltzmann constant (W/m²/K⁴)
        const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;
        const DEFAULT_EMISSIVITY: f64 = 0.95;
        
        let source_temp = source_data.temperature_k;
        let target_temp = target_data.temperature_k;
        
        // Contact area based on relationship type
        let contact_area_m2 = match relationship_type {
            BinaryPairType::Vertical => 1000.0,   // Vertical contact area
            BinaryPairType::Horizontal => 500.0,  // Lateral contact area
        };
        
        // Net radiant heat transfer: Q = ε * σ * A * (T₁⁴ - T₂⁴)
        let net_power = DEFAULT_EMISSIVITY * STEFAN_BOLTZMANN * contact_area_m2 * 
                       (source_temp.powi(4) - target_temp.powi(4));
        
        // Convert to energy over time step
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = config.years_per_step as f64 * seconds_per_year;
        
        net_power * time_step_seconds // Joules
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, u64, usize) {
        (self.total_cells_processed, self.total_neighbor_calculations, self.neighbor_cache.len())
    }
}

impl Default for ParallelCellProcessor {
    fn default() -> Self {
        Self::new()
    }
}
