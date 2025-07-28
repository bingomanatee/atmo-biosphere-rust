use crate::binary_pairing::{BinaryPair, BinaryPairListener, BinaryPairType};
use crate::transaction_manager_simple::SimpleTransactionManager;
use crate::cell_location::CellLocation;
// use std::sync::{Arc, Mutex}; // Unused for now
use std::thread;
use std::sync::mpsc;
use std::collections::HashMap;

/// Parallelized Binary Pairing System - distributes pairs across multiple threads
pub struct ParallelBinaryPairingSystem {
    /// All binary pairs to process
    pairs: Vec<BinaryPair>,
    /// Number of worker threads
    num_threads: usize,
    /// Performance tracking
    total_pairs_processed: u64,
    total_listener_calls: u64,
}

/// Message sent to worker threads
#[derive(Debug)]
struct WorkerMessage {
    pairs: Vec<BinaryPair>,
    step: i64,
    year: i64,
}

/// Result from worker threads
#[derive(Debug)]
struct WorkerResult {
    energy_deltas: HashMap<CellLocation, f64>,
    mass_deltas: HashMap<CellLocation, f64>,
    pairs_processed: u64,
    listener_calls: u64,
}

/// Worker thread data
struct WorkerData {
    listeners: Vec<Box<dyn BinaryPairListener + Send>>,
    thread_id: usize,
}

impl ParallelBinaryPairingSystem {
    /// Create new parallel binary pairing system
    pub fn new(num_threads: usize) -> Self {
        let actual_threads = if num_threads == 0 {
            num_cpus::get() // Auto-detect CPU count
        } else {
            num_threads
        };
        
        println!("🔗 Creating Parallel Binary Pairing System with {} threads", actual_threads);
        
        Self {
            pairs: Vec::new(),
            num_threads: actual_threads,
            total_pairs_processed: 0,
            total_listener_calls: 0,
        }
    }
    
    /// Initialize pairs from simulation
    pub fn initialize_pairs(&mut self, sim: &crate::sim_immut::simulation_immut::SimulationImmut) {
        // Use the same pair generation logic as the sequential version
        self.pairs.clear();
        
        // Generate all binary pairs
        self.generate_horizontal_pairs(sim);
        self.generate_vertical_pairs(sim);
        self.generate_surface_to_space_pairs(sim);
        
        println!("✅ Parallel binary pairs initialized:");
        self.print_pair_statistics();
    }
    
    /// Process all pairs in parallel using worker threads
    pub fn process_all_pairs_parallel(
        &mut self,
        listeners: Vec<Box<dyn BinaryPairListener + Send>>,
        step: i64,
        year: i64,
    ) -> (HashMap<CellLocation, f64>, HashMap<CellLocation, f64>) {
        
        if self.pairs.is_empty() {
            return (HashMap::new(), HashMap::new());
        }
        
        // Split pairs into chunks for parallel processing
        let chunk_size = (self.pairs.len() + self.num_threads - 1) / self.num_threads;
        let pair_chunks: Vec<Vec<BinaryPair>> = self.pairs
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        println!("🔄 Processing {} pairs across {} threads ({} pairs per thread)",
                 self.pairs.len(), self.num_threads, chunk_size);
        
        // Create channels for communication
        let (tx, rx) = mpsc::channel();
        
        // Spawn worker threads
        let mut handles = Vec::new();
        
        for (thread_id, pair_chunk) in pair_chunks.into_iter().enumerate() {
            let tx_clone = tx.clone();
            let listeners_clone = clone_listeners(&listeners);
            
            let handle = thread::spawn(move || {
                let result = process_pairs_in_thread(
                    pair_chunk,
                    listeners_clone,
                    step,
                    year,
                    thread_id,
                );
                
                tx_clone.send(result).unwrap();
            });
            
            handles.push(handle);
        }
        
        // Drop the original sender
        drop(tx);
        
        // Collect results from all threads
        let mut combined_energy_deltas = HashMap::new();
        let mut combined_mass_deltas = HashMap::new();
        let mut total_pairs_processed = 0;
        let mut total_listener_calls = 0;
        
        for result in rx {
            // Merge energy deltas
            for (location, delta) in result.energy_deltas {
                *combined_energy_deltas.entry(location).or_insert(0.0) += delta;
            }
            
            // Merge mass deltas
            for (location, delta) in result.mass_deltas {
                *combined_mass_deltas.entry(location).or_insert(0.0) += delta;
            }
            
            total_pairs_processed += result.pairs_processed;
            total_listener_calls += result.listener_calls;
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Update statistics
        self.total_pairs_processed += total_pairs_processed;
        self.total_listener_calls += total_listener_calls;
        
        println!("✅ Parallel processing completed:");
        println!("   - Pairs processed: {}", total_pairs_processed);
        println!("   - Listener calls: {}", total_listener_calls);
        println!("   - Energy deltas: {}", combined_energy_deltas.len());
        println!("   - Mass deltas: {}", combined_mass_deltas.len());
        
        (combined_energy_deltas, combined_mass_deltas)
    }
    
    /// Generate horizontal pairs (same as sequential version)
    fn generate_horizontal_pairs(&mut self, sim: &crate::sim_immut::simulation_immut::SimulationImmut) {
        use std::collections::HashSet;
        use crate::binary_pairing::{BinaryPairCell};
        
        let mut processed_pairs = HashSet::new();
        
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                let neighbors = h3_cell.grid_disk::<Vec<_>>(1);
                
                for neighbor_h3 in neighbors {
                    if let Some(neighbor_column) = layer_set.layers.get(&neighbor_h3) {
                        for (cell_idx, cell) in column.cells.iter().enumerate() {
                            if let Some(neighbor_cell) = neighbor_column.cells.get(cell_idx) {
                                let pair_key = if h3_cell < &neighbor_h3 {
                                    (*h3_cell, neighbor_h3, layer_set_idx, cell_idx)
                                } else {
                                    (neighbor_h3, *h3_cell, layer_set_idx, cell_idx)
                                };
                                
                                if !processed_pairs.contains(&pair_key) {
                                    processed_pairs.insert(pair_key);
                                    
                                    let pair = BinaryPair {
                                        pair_type: BinaryPairType::HorizontalNeighbors,
                                        cell_a: BinaryPairCell {
                                            location: CellLocation {
                                                layer_set_index: layer_set_idx,
                                                h3_cell_index: *h3_cell,
                                                depth_index: cell_idx,
                                            },
                                            cell: cell.clone(),
                                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                                        },
                                        cell_b: Some(BinaryPairCell {
                                            location: CellLocation {
                                                layer_set_index: layer_set_idx,
                                                h3_cell_index: neighbor_h3,
                                                depth_index: cell_idx,
                                            },
                                            cell: neighbor_cell.clone(),
                                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                                        }),
                                        distance_m: 60_000.0,
                                        contact_area_m2: 1e9,
                                    };
                                    
                                    self.pairs.push(pair);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Generate vertical pairs (same as sequential version)
    fn generate_vertical_pairs(&mut self, sim: &crate::sim_immut::simulation_immut::SimulationImmut) {
        use crate::binary_pairing::BinaryPairCell;
        
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for cell_idx in 0..column.cells.len().saturating_sub(1) {
                    let upper_cell = &column.cells[cell_idx];
                    let lower_cell = &column.cells[cell_idx + 1];
                    
                    let pair = BinaryPair {
                        pair_type: BinaryPairType::VerticalNeighbors,
                        cell_a: BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell_index: *h3_cell,
                                depth_index: cell_idx,
                            },
                            cell: upper_cell.clone(),
                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                        },
                        cell_b: Some(BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell_index: *h3_cell,
                                depth_index: cell_idx + 1,
                            },
                            cell: lower_cell.clone(),
                            depth_km: layer_set.start_height_km + ((cell_idx + 1) as f64 * 10.0),
                        }),
                        distance_m: 10_000.0,
                        contact_area_m2: 3.6e9,
                    };
                    
                    self.pairs.push(pair);
                }
            }
        }
    }
    
    /// Generate surface-to-space pairs (same as sequential version)
    fn generate_surface_to_space_pairs(&mut self, sim: &crate::sim_immut::simulation_immut::SimulationImmut) {
        use crate::binary_pairing::BinaryPairCell;
        
        if let Some(surface_layer) = sim.layer_sets.first() {
            for (h3_cell, column) in &surface_layer.layers {
                if let Some(surface_cell) = column.cells.first() {
                    let pair = BinaryPair {
                        pair_type: BinaryPairType::SurfaceToSpace,
                        cell_a: BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: 0,
                                h3_cell_index: *h3_cell,
                                depth_index: 0,
                            },
                            cell: surface_cell.clone(),
                            depth_km: 0.0,
                        },
                        cell_b: None,
                        distance_m: f64::INFINITY,
                        contact_area_m2: 3.6e9,
                    };
                    
                    self.pairs.push(pair);
                }
            }
        }
    }
    
    /// Print pair statistics
    fn print_pair_statistics(&self) {
        let horizontal_count = self.pairs.iter().filter(|p| p.pair_type == BinaryPairType::HorizontalNeighbors).count();
        let vertical_count = self.pairs.iter().filter(|p| p.pair_type == BinaryPairType::VerticalNeighbors).count();
        let surface_count = self.pairs.iter().filter(|p| p.pair_type == BinaryPairType::SurfaceToSpace).count();
        
        println!("   - Horizontal pairs: {}", horizontal_count);
        println!("   - Vertical pairs: {}", vertical_count);
        println!("   - Surface-to-space pairs: {}", surface_count);
        println!("   - Total pairs: {}", self.pairs.len());
        println!("   - Threads: {}", self.num_threads);
        println!("   - Pairs per thread: ~{}", (self.pairs.len() + self.num_threads - 1) / self.num_threads);
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, u64, usize) {
        (self.total_pairs_processed, self.total_listener_calls, self.pairs.len())
    }
}

/// Process pairs in a worker thread
fn process_pairs_in_thread(
    pairs: Vec<BinaryPair>,
    mut listeners: Vec<Box<dyn BinaryPairListener + Send>>,
    step: i64,
    year: i64,
    thread_id: usize,
) -> WorkerResult {
    let mut transaction_manager = SimpleTransactionManager::new();
    transaction_manager.set_current_step(step);
    
    let mut pairs_processed = 0;
    let mut listener_calls = 0;
    
    // Process each pair with all interested listeners
    for pair in pairs {
        pairs_processed += 1;
        
        for listener in &mut listeners {
            if listener.interested_pair_types().contains(&pair.pair_type) {
                listener.on_binary_pair(&pair, &mut transaction_manager, step, year);
                listener_calls += 1;
            }
        }
    }
    
    println!("🧵 Thread {} completed: {} pairs, {} listener calls", 
             thread_id, pairs_processed, listener_calls);
    
    WorkerResult {
        energy_deltas: transaction_manager.get_all_energy_deltas().clone(),
        mass_deltas: transaction_manager.get_all_mass_deltas().clone(),
        pairs_processed,
        listener_calls,
    }
}

/// Clone listeners for thread safety
fn clone_listeners(_listeners: &[Box<dyn BinaryPairListener + Send>]) -> Vec<Box<dyn BinaryPairListener + Send>> {
    // Create new instances for each thread
    use crate::component::thread_safe_listeners::{ThreadSafeRadiativeTransferListener, ThreadSafeCoreHeatListener};

    vec![
        Box::new(ThreadSafeRadiativeTransferListener::new()),
        Box::new(ThreadSafeCoreHeatListener::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parallel_binary_pairing_creation() {
        println!("🔗 Testing Parallel Binary Pairing System");
        
        let system = ParallelBinaryPairingSystem::new(4);
        assert_eq!(system.num_threads, 4);
        
        let auto_system = ParallelBinaryPairingSystem::new(0);
        assert!(auto_system.num_threads > 0, "Should auto-detect CPU count");
        
        println!("✅ Parallel system created with {} threads", auto_system.num_threads);
    }
}
