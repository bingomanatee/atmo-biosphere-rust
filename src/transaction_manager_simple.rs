use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Simple hash-based transaction system for energy/mass tracking
/// Replaces the complex atomic transaction system when scaling is not needed
#[derive(Debug, Clone)]
pub struct SimpleTransactionManager {
    /// Energy and mass deltas per cell location
    energy_deltas: HashMap<CellLocation, f64>,
    mass_deltas: HashMap<CellLocation, f64>,
    
    /// Optional debug mode for validation
    debug_mode: bool,
    debug_journal: Vec<SimpleTransaction>,
    
    /// Performance tracking
    current_step: i64,
    total_transactions: u64,
}

/// Simplified cell location identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellLocation {
    pub layer_set_index: usize,
    pub h3_cell: h3o::CellIndex,
    pub cell_index: usize,
}

/// Simplified transaction for debug mode only
#[derive(Debug, Clone)]
pub struct SimpleTransaction {
    pub location: CellLocation,
    pub energy_delta: f64,
    pub mass_delta: f64,
    pub step: i64,
    pub source: String, // For debugging: "radiative_transfer", "component_X", etc.
}

impl SimpleTransactionManager {
    /// Create new simple transaction manager
    pub fn new() -> Self {
        Self {
            energy_deltas: HashMap::new(),
            mass_deltas: HashMap::new(),
            debug_mode: false,
            debug_journal: Vec::new(),
            current_step: 0,
            total_transactions: 0,
        }
    }
    
    /// Create with debug mode enabled
    pub fn new_with_debug() -> Self {
        Self {
            energy_deltas: HashMap::new(),
            mass_deltas: HashMap::new(),
            debug_mode: true,
            debug_journal: Vec::new(),
            current_step: 0,
            total_transactions: 0,
        }
    }
    
    /// Set current simulation step
    pub fn set_current_step(&mut self, step: i64) {
        self.current_step = step;
    }
    
    /// Add energy delta to a cell
    pub fn add_energy_delta(&mut self, location: CellLocation, energy_delta: f64, source: &str) {
        // Add to energy deltas
        *self.energy_deltas.entry(location.clone()).or_insert(0.0) += energy_delta;
        
        // Optional debug logging
        if self.debug_mode {
            self.debug_journal.push(SimpleTransaction {
                location: location.clone(),
                energy_delta,
                mass_delta: 0.0,
                step: self.current_step,
                source: source.to_string(),
            });
        }
        
        self.total_transactions += 1;
    }
    
    /// Add mass delta to a cell
    pub fn add_mass_delta(&mut self, location: CellLocation, mass_delta: f64, source: &str) {
        // Add to mass deltas
        *self.mass_deltas.entry(location.clone()).or_insert(0.0) += mass_delta;
        
        // Optional debug logging
        if self.debug_mode {
            self.debug_journal.push(SimpleTransaction {
                location: location.clone(),
                energy_delta: 0.0,
                mass_delta,
                step: self.current_step,
                source: source.to_string(),
            });
        }
        
        self.total_transactions += 1;
    }
    
    /// Add both energy and mass delta to a cell (atomic operation)
    pub fn add_energy_mass_delta(&mut self, location: CellLocation, energy_delta: f64, mass_delta: f64, source: &str) {
        // Add to deltas
        *self.energy_deltas.entry(location.clone()).or_insert(0.0) += energy_delta;
        *self.mass_deltas.entry(location.clone()).or_insert(0.0) += mass_delta;
        
        // Optional debug logging
        if self.debug_mode {
            self.debug_journal.push(SimpleTransaction {
                location: location.clone(),
                energy_delta,
                mass_delta,
                step: self.current_step,
                source: source.to_string(),
            });
        }
        
        self.total_transactions += 1;
    }
    
    /// Get energy delta for a cell
    pub fn get_energy_delta(&self, location: &CellLocation) -> f64 {
        self.energy_deltas.get(location).copied().unwrap_or(0.0)
    }
    
    /// Get mass delta for a cell
    pub fn get_mass_delta(&self, location: &CellLocation) -> f64 {
        self.mass_deltas.get(location).copied().unwrap_or(0.0)
    }
    
    /// Get all energy deltas (for applying to layer sets)
    pub fn get_all_energy_deltas(&self) -> &HashMap<CellLocation, f64> {
        &self.energy_deltas
    }
    
    /// Get all mass deltas (for applying to layer sets)
    pub fn get_all_mass_deltas(&self) -> &HashMap<CellLocation, f64> {
        &self.mass_deltas
    }
    
    /// Clear all deltas (call after applying to layer sets)
    pub fn clear_deltas(&mut self) {
        self.energy_deltas.clear();
        self.mass_deltas.clear();
        
        // Keep debug journal for analysis, but limit size
        if self.debug_mode && self.debug_journal.len() > 10000 {
            self.debug_journal.drain(0..5000); // Keep recent 5000 entries
        }
    }
    
    /// Get transaction statistics
    pub fn get_transaction_stats(&self) -> (usize, u64) {
        // Return (pending_count, total_committed)
        (self.energy_deltas.len() + self.mass_deltas.len(), self.total_transactions)
    }
    
    /// Calculate energy conservation hash for validation
    pub fn calculate_energy_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // Hash all energy deltas
        let mut energy_sum = 0.0;
        for delta in self.energy_deltas.values() {
            energy_sum += delta;
        }
        
        // Hash the sum (should be ~0 for energy conservation)
        energy_sum.to_bits().hash(&mut hasher);
        hasher.finish()
    }
    
    /// Get debug information (only available in debug mode)
    pub fn get_debug_info(&self) -> Option<&Vec<SimpleTransaction>> {
        if self.debug_mode {
            Some(&self.debug_journal)
        } else {
            None
        }
    }
    
    /// Validate energy conservation (debug mode only)
    pub fn validate_energy_conservation(&self, tolerance: f64) -> Result<(), String> {
        if !self.debug_mode {
            return Ok(()); // Skip validation in non-debug mode
        }
        
        let total_energy_delta: f64 = self.energy_deltas.values().sum();
        
        if total_energy_delta.abs() > tolerance {
            Err(format!("Energy conservation violation: total delta = {:.2e} J (tolerance: {:.2e})", 
                       total_energy_delta, tolerance))
        } else {
            Ok(())
        }
    }
    
    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> SimpleTransactionMetrics {
        SimpleTransactionMetrics {
            total_transactions: self.total_transactions,
            pending_energy_deltas: self.energy_deltas.len(),
            pending_mass_deltas: self.mass_deltas.len(),
            debug_journal_size: if self.debug_mode { self.debug_journal.len() } else { 0 },
            current_step: self.current_step,
        }
    }
}

/// Performance metrics for the simple transaction system
#[derive(Debug, Clone)]
pub struct SimpleTransactionMetrics {
    pub total_transactions: u64,
    pub pending_energy_deltas: usize,
    pub pending_mass_deltas: usize,
    pub debug_journal_size: usize,
    pub current_step: i64,
}

impl Default for SimpleTransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use h3o::CellIndex;
    
    #[test]
    fn test_simple_transaction_manager() {
        let mut manager = SimpleTransactionManager::new();
        
        let location = CellLocation {
            layer_set_index: 0,
            h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
            cell_index: 0,
        };
        
        // Add energy delta
        manager.add_energy_delta(location.clone(), 100.0, "test");
        assert_eq!(manager.get_energy_delta(&location), 100.0);
        
        // Add more energy delta (should accumulate)
        manager.add_energy_delta(location.clone(), 50.0, "test");
        assert_eq!(manager.get_energy_delta(&location), 150.0);
        
        // Add mass delta
        manager.add_mass_delta(location.clone(), 10.0, "test");
        assert_eq!(manager.get_mass_delta(&location), 10.0);
        
        // Check stats
        let (pending, total) = manager.get_transaction_stats();
        assert_eq!(total, 3); // 3 transactions added
        
        // Clear deltas
        manager.clear_deltas();
        assert_eq!(manager.get_energy_delta(&location), 0.0);
        assert_eq!(manager.get_mass_delta(&location), 0.0);
    }
    
    #[test]
    fn test_debug_mode() {
        let mut manager = SimpleTransactionManager::new_with_debug();
        
        let location = CellLocation {
            layer_set_index: 0,
            h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
            cell_index: 0,
        };
        
        manager.add_energy_delta(location.clone(), 100.0, "radiative_transfer");
        
        // Check debug info is available
        let debug_info = manager.get_debug_info().unwrap();
        assert_eq!(debug_info.len(), 1);
        assert_eq!(debug_info[0].energy_delta, 100.0);
        assert_eq!(debug_info[0].source, "radiative_transfer");
    }
}
