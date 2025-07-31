use crate::binary_pair::BinaryPairType;
use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::simulation::{GeologicalCellData, SimulationConfig};
use crate::utils::h3_utils::H3Utils;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Efficient parallel cell processor that eliminates pre-computed binary pairs
/// Instead, it processes cells in chunks and computes neighbors on-demand
pub struct ParallelCellProcessor {
    /// Cached neighbor relationships per cell
    neighbor_cache: HashMap<CellLocation, Vec<(CellLocation, BinaryPairType)>>,
    /// Performance tracking with atomic counters for thread safety
    total_cells_processed: AtomicU64,
    total_neighbor_calculations: AtomicU64,
}

impl ParallelCellProcessor {
    /// Create new parallel cell processor
    pub fn new() -> Self {
        Self {
            neighbor_cache: HashMap::new(),
            total_cells_processed: AtomicU64::new(0),
            total_neighbor_calculations: AtomicU64::new(0),
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
        let mut cell_locations: Vec<CellLocation> = existing_cells.into_iter().collect();

        // Sort by H3 cell index for better cache locality during neighbor lookup
        cell_locations.sort_by_key(|loc| loc.h3_cell_index());

        // Use larger chunks to reduce threading overhead during initialization
        let optimal_chunk_size = (cell_locations.len() / rayon::current_num_threads()).max(2000);
        let num_chunks = (cell_locations.len() + optimal_chunk_size - 1) / optimal_chunk_size;

        println!("   📊 Processing {} cells in {} chunks of ~{} across {} threads",
                 cell_locations.len(), num_chunks, optimal_chunk_size, rayon::current_num_threads());
        
        // Process chunks in parallel with optimized chunk size
        let neighbor_results: Vec<HashMap<CellLocation, Vec<(CellLocation, BinaryPairType)>>> =
            cell_locations.par_chunks(optimal_chunk_size)
                .map(|chunk| {
                    let mut local_neighbors = HashMap::with_capacity(chunk.len());

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
        
        // Convert to vector for parallel processing with spatial locality optimization
        let mut cell_entries: Vec<_> = cells.iter().collect();

        // Sort by H3 cell index for better cache locality
        cell_entries.sort_by_key(|entry| entry.key().h3_cell_index());

        // Optimal chunk size: larger chunks to reduce overhead
        // Original was ~2450 per chunk, let's try 4x larger
        let optimal_chunk_size = (cell_entries.len() / 3).max(8000); // Only 3 chunks total

        if step % 1000 == 0 {
            let num_chunks = (cell_entries.len() + optimal_chunk_size - 1) / optimal_chunk_size;
            println!("🔗 {} using {} chunks of ~{} cells each across {} threads",
                     component_name, num_chunks, optimal_chunk_size, rayon::current_num_threads());
        }

        // Test: try single-threaded for comparison
        let use_single_thread = cell_entries.len() < 50000; // Single thread for smaller problems

        let results: Vec<Vec<(CellLocation, CellLocation, BinaryPairType, f64)>> = if use_single_thread {
            if step % 1000 == 0 {
                println!("🔗 {} using SINGLE-THREADED processing for {} cells", component_name, cell_entries.len());
            }

            // Single-threaded processing
            vec![{
                let mut all_transfers = Vec::with_capacity(cell_entries.len() * 10);

                for entry in &cell_entries {
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
                                    all_transfers.push((
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

                all_transfers
            }]
        } else {
            // Multi-threaded chunked approach
            cell_entries.par_chunks(optimal_chunk_size)
                .map(|chunk| {
                    let mut local_transfers = Vec::with_capacity(chunk.len() * 10); // Pre-allocate more

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
                .collect()
        };
        
        // Flatten all results and batch apply transfers for better performance
        let all_transfers: Vec<_> = results.into_iter().flatten().collect();
        let total_transfers = all_transfers.len();

        // Apply transfers in batches to reduce Actor overhead
        const BATCH_SIZE: usize = 1000;
        for batch in all_transfers.chunks(BATCH_SIZE) {
            for &(source_cell, target_cell, _relationship, energy_transfer) in batch {
                if energy_transfer > 0.0 {
                    // Source is hotter, transfers to target
                    actor.add("geological_cells", source_cell, "energy_joules", -energy_transfer);
                    actor.add("geological_cells", target_cell, "energy_joules", energy_transfer);
                } else {
                    // Target is hotter, transfers to source
                    actor.add("geological_cells", target_cell, "energy_joules", energy_transfer);
                    actor.add("geological_cells", source_cell, "energy_joules", -energy_transfer);
                }
            }
        }
        
        self.total_cells_processed.fetch_add(cells.len() as u64, Ordering::Relaxed);
        self.total_neighbor_calculations.fetch_add(total_transfers as u64, Ordering::Relaxed);
        
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
    
    /// Calculate energy transfer between two cells (optimized)
    fn calculate_energy_transfer(
        source_data: &GeologicalCellData,
        target_data: &GeologicalCellData,
        relationship_type: BinaryPairType,
        config: &SimulationConfig,
    ) -> f64 {
        // Pre-computed constants for performance
        const STEFAN_BOLTZMANN_EMISSIVITY: f64 = 5.670374419e-8 * 0.95; // Combined constant
        const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 3600.0;

        let source_temp = source_data.temperature_k;
        let target_temp = target_data.temperature_k;

        // Early exit for small temperature differences (optimization)
        let temp_diff = (source_temp - target_temp).abs();
        if temp_diff < 1.0 { // Less than 1K difference
            return 0.0;
        }

        // Contact area based on relationship type
        let contact_area_m2 = match relationship_type {
            BinaryPairType::Vertical => 1000.0,   // Vertical contact area
            BinaryPairType::Horizontal => 500.0,  // Lateral contact area
        };

        // Optimized calculation: pre-compute temp^4 values
        let source_temp4 = source_temp * source_temp * source_temp * source_temp;
        let target_temp4 = target_temp * target_temp * target_temp * target_temp;

        // Net radiant heat transfer: Q = ε * σ * A * (T₁⁴ - T₂⁴)
        let net_power = STEFAN_BOLTZMANN_EMISSIVITY * contact_area_m2 * (source_temp4 - target_temp4);

        // Convert to energy over time step (pre-computed constant)
        let time_step_seconds = config.years_per_step as f64 * SECONDS_PER_YEAR;

        net_power * time_step_seconds // Joules
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, u64, usize) {
        (
            self.total_cells_processed.load(Ordering::Relaxed),
            self.total_neighbor_calculations.load(Ordering::Relaxed),
            self.neighbor_cache.len()
        )
    }
}

impl Clone for ParallelCellProcessor {
    fn clone(&self) -> Self {
        Self {
            neighbor_cache: self.neighbor_cache.clone(),
            total_cells_processed: AtomicU64::new(self.total_cells_processed.load(Ordering::Relaxed)),
            total_neighbor_calculations: AtomicU64::new(self.total_neighbor_calculations.load(Ordering::Relaxed)),
        }
    }
}

impl Default for ParallelCellProcessor {
    fn default() -> Self {
        Self::new()
    }
}
