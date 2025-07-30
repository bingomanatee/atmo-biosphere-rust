use crate::collections::{Actor, CollectionsManager};
use crate::parallel_cell_processor::ParallelCellProcessor;
use crate::simulation::{Component, Simulation, SimulationConfig};

/// Parallel RadianceComponent that uses chunked cell processing instead of binary pairs
/// This eliminates the need to pre-compute 14M binary pairs and processes cells efficiently
pub struct ParallelRadianceComponent {
    /// Parallel cell processor for efficient neighbor computation
    processor: ParallelCellProcessor,
    /// Whether the neighbor cache has been initialized
    initialized: bool,
}

impl ParallelRadianceComponent {
    /// Create new parallel radiance component
    pub fn new() -> Self {
        Self {
            processor: ParallelCellProcessor::new(),
            initialized: false,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config() -> Self {
        Self::new()
    }
}

impl Component for ParallelRadianceComponent {
    fn name(&self) -> &'static str {
        "ParallelRadianceComponent"
    }
    
    fn initialize(&mut self, coll_mgr: &mut CollectionsManager, _config: &SimulationConfig) {
        println!("🌟 ParallelRadianceComponent: Initializing parallel thermal radiance...");
        
        // Initialize neighbor cache once at startup
        self.processor.initialize_neighbor_cache(coll_mgr);
        self.initialized = true;
        
        println!("✅ ParallelRadianceComponent: Neighbor cache initialized");
    }
    
    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, year: f64, config: &SimulationConfig) {
        if !self.initialized {
            println!("⚠️  ParallelRadianceComponent: Not initialized, skipping step");
            return;
        }
        
        // Process all cells in parallel chunks for thermal radiance
        // This is much more efficient than iterating through 14M binary pairs
        let mut processor = self.processor.clone(); // TODO: Make this more efficient
        processor.process_cells_parallel(
            coll_mgr, 
            actor, 
            step, 
            year, 
            config, 
            "ParallelRadianceComponent"
        );
    }
    
    fn complete(&mut self, _sim: &Simulation, _config: &SimulationConfig) {
        let (cells_processed, calculations, cache_size) = self.processor.get_performance_stats();
        
        println!("🌟 ParallelRadianceComponent Performance Summary:");
        println!("   - Total cells processed: {}", cells_processed);
        println!("   - Total energy calculations: {}", calculations);
        println!("   - Neighbor cache size: {}", cache_size);
        
        if cells_processed > 0 {
            println!("   - Avg calculations per cell: {:.1}", 
                     calculations as f64 / cells_processed as f64);
        }
    }
}

impl Default for ParallelRadianceComponent {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Make ParallelCellProcessor cloneable or use Arc<Mutex<>> for sharing
impl Clone for ParallelRadianceComponent {
    fn clone(&self) -> Self {
        Self {
            processor: ParallelCellProcessor::new(), // Create new processor for now
            initialized: false,
        }
    }
}
