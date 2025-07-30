use crate::binary_pair::BinaryPairType;
use crate::cell_location::CellLocation;
use crate::collections::Actor;

/// Trait for components that want to listen to binary pair events
/// This follows the efficient pattern from simulation_immut.rs where we:
/// 1. Process all binary pairs once per step
/// 2. Pass pair IDs and relationship type to listeners
/// 3. Let listeners decide what to do with each pair
pub trait BinaryPairListener {
    /// Called for each binary pair during the simulation step
    /// 
    /// # Arguments
    /// * `cell_a` - First cell location in the pair
    /// * `cell_b` - Second cell location in the pair  
    /// * `relationship` - Type of relationship (Vertical, Horizontal)
    /// * `actor` - Actor for recording changes
    /// * `step` - Current simulation step
    /// * `year` - Current simulation year
    fn on_binary_pair(
        &self,
        cell_a: CellLocation,
        cell_b: CellLocation,
        relationship: BinaryPairType,
        actor: &mut Actor,
        step: u32,
        year: f64,
    );
    
    /// Return which pair types this component is interested in
    /// This allows the system to skip calling the listener for irrelevant pairs
    fn interested_pair_types(&self) -> Vec<BinaryPairType>;
    
    /// Component identifier for debugging and performance tracking
    fn component_name(&self) -> &'static str;
}

/// Binary pair processor that efficiently processes all pairs once per step
/// and calls interested listeners
pub struct BinaryPairProcessor {
    /// Components listening to binary pair events
    listeners: Vec<Box<dyn BinaryPairListener>>,
    /// Performance tracking
    total_pairs_processed: u64,
    total_listener_calls: u64,
}

impl BinaryPairProcessor {
    /// Create new binary pair processor
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            total_pairs_processed: 0,
            total_listener_calls: 0,
        }
    }
    
    /// Add a component listener
    pub fn add_listener(&mut self, listener: Box<dyn BinaryPairListener>) {
        println!("🎧 Adding binary pair listener: {}", listener.component_name());
        self.listeners.push(listener);
    }
    
    /// Process all binary pairs once - call all interested listeners
    /// This is the core efficient pattern: iterate through pairs once,
    /// let multiple components process each pair as needed
    pub fn process_all_pairs(
        &mut self,
        coll_mgr: &crate::collections::CollectionsManager,
        actor: &mut Actor,
        step: u32,
        year: f64,
    ) {
        // Get binary pairs collection
        let pairs = match coll_mgr.get::<crate::binary_pair::BinaryPairId, crate::binary_pair::BinaryPair>("binary_pairs") {
            Some(pairs) => pairs,
            None => {
                println!("🔗 BinaryPairProcessor: No binary pairs found, skipping processing");
                return;
            }
        };

        let mut pairs_processed = 0;
        let mut listener_calls = 0;

        println!("🔗 BinaryPairProcessor: Processing {} binary pairs with {} listeners", 
                 pairs.len(), self.listeners.len());

        // Sequential processing - iterate through each pair once
        for entry in pairs.iter() {
            let pair = entry.value();
            let (cell_a, cell_b) = pair.get_cells();
            let relationship = pair.pair_type;
            
            pairs_processed += 1;

            // Call all interested listeners for this pair
            for listener in &self.listeners {
                if listener.interested_pair_types().contains(&relationship) {
                    listener.on_binary_pair(cell_a, cell_b, relationship, actor, step, year);
                    listener_calls += 1;
                }
            }
        }

        self.total_pairs_processed += pairs_processed as u64;
        self.total_listener_calls += listener_calls as u64;

        if pairs_processed > 0 {
            println!("🔗 BinaryPairProcessor: Processed {} pairs, made {} listener calls", 
                     pairs_processed, listener_calls);
        }
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, u64, usize) {
        (self.total_pairs_processed, self.total_listener_calls, self.listeners.len())
    }
}

impl Default for BinaryPairProcessor {
    fn default() -> Self {
        Self::new()
    }
}
