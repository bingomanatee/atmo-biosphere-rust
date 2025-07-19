// Demonstration of transaction merging for efficiency

use std::collections::HashMap;

/// Flat cell ID for demonstration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlatCellId(pub u64);

impl FlatCellId {
    pub const NONE: FlatCellId = FlatCellId(u64::MAX);
    
    pub fn new(layer: u8, h3_cell: u64, depth: u8) -> Self {
        let id = ((layer as u64) << 56) | ((h3_cell & 0xFFFFFFFFFFFF) << 8) | (depth as u64);
        Self(id)
    }
    
    pub fn description(&self) -> String {
        if *self == Self::NONE {
            "NONE".to_string()
        } else {
            let layer = (self.0 >> 56) as u8;
            let h3_cell = (self.0 >> 8) & 0xFFFFFFFFFFFF;
            let depth = self.0 as u8;
            format!("L{}:H{}:D{}", layer, h3_cell, depth)
        }
    }
}

/// Minimal transaction structure
#[derive(Debug, Clone)]
pub struct Transaction {
    pub source: String,        // Component name for tracking
    pub from: FlatCellId,      // Source cell
    pub to: FlatCellId,        // Target cell (NONE for no target)
    pub energy_delta: f32,     // Energy change
    pub mass_delta: f32,       // Mass change
}

/// Transaction manager with proper merging
#[derive(Debug)]
pub struct MergingTransactionManager {
    pending_transactions: Vec<Transaction>,
}

impl MergingTransactionManager {
    pub fn new() -> Self {
        Self {
            pending_transactions: Vec::new(),
        }
    }
    
    pub fn propose_transaction(&mut self, transaction: Transaction) {
        println!("📝 Proposed: {} {} → {}: {:.1e}J, {:.1e}kg", 
            transaction.source,
            transaction.from.description(), 
            transaction.to.description(),
            transaction.energy_delta,
            transaction.mass_delta);
        
        self.pending_transactions.push(transaction);
    }
    
    /// Merge transactions with same source and destination
    pub fn merge_transactions(&mut self) -> Vec<Transaction> {
        println!("\n🔄 Merging {} pending transactions...", self.pending_transactions.len());
        
        // Key: (from, to) - merge all transactions between same cells
        let mut merged: HashMap<(FlatCellId, FlatCellId), Transaction> = HashMap::new();
        
        for transaction in self.pending_transactions.drain(..) {
            let key = (transaction.from, transaction.to);
            
            if let Some(existing) = merged.get_mut(&key) {
                // Merge with existing transaction
                existing.energy_delta += transaction.energy_delta;
                existing.mass_delta += transaction.mass_delta;
                existing.source = format!("{}+{}", existing.source, transaction.source);
                
                println!("🔗 Merged: {} → {} (now {:.1e}J, {:.1e}kg)", 
                    transaction.from.description(),
                    transaction.to.description(),
                    existing.energy_delta,
                    existing.mass_delta);
            } else {
                // First transaction for this source-destination pair
                merged.insert(key, transaction);
            }
        }
        
        let merged_transactions: Vec<Transaction> = merged.into_values().collect();
        
        println!("✅ Merged into {} unique source-destination pairs", merged_transactions.len());
        
        merged_transactions
    }
    
    /// Show the effect of merging
    pub fn demonstrate_merging_efficiency(&mut self) {
        let original_count = self.pending_transactions.len();
        let merged = self.merge_transactions();
        let merged_count = merged.len();
        
        println!("\n📊 Merging Efficiency:");
        println!("   Original transactions: {}", original_count);
        println!("   Merged transactions: {}", merged_count);
        println!("   Reduction: {:.1}% ({} fewer)", 
            (1.0 - merged_count as f32 / original_count as f32) * 100.0,
            original_count - merged_count);
        
        println!("\n🎯 Final merged transactions:");
        for transaction in &merged {
            println!("   {} {} → {}: {:.1e}J, {:.1e}kg", 
                transaction.source,
                transaction.from.description(),
                transaction.to.description(),
                transaction.energy_delta,
                transaction.mass_delta);
        }
    }
}

fn main() {
    println!("🧪 Transaction Merging Demonstration");
    println!("Problem: Multiple transactions between same cells are inefficient");
    println!("Solution: Merge transactions with same source and destination\n");
    
    test_transaction_merging();
    
    println!("\n✅ Transaction Merging Demo Completed!");
}

fn test_transaction_merging() {
    println!("🔬 Test: Multiple Components Affecting Same Cell Pairs");
    
    let mut tm = MergingTransactionManager::new();
    
    // Create some cells
    let cell_a = FlatCellId::new(0, 12345, 0); // Crust surface
    let cell_b = FlatCellId::new(0, 12345, 1); // Crust deeper
    let cell_c = FlatCellId::new(1, 12345, 0); // Upper mantle
    
    // Multiple components affecting the same cell pairs
    
    // Thermal conduction: A → B
    tm.propose_transaction(Transaction {
        source: "ThermalConduction".to_string(),
        from: cell_a,
        to: cell_b,
        energy_delta: -1e18,
        mass_delta: 0.0,
    });
    
    // Convection also: A → B (same pair!)
    tm.propose_transaction(Transaction {
        source: "ConvectionPlume".to_string(),
        from: cell_a,
        to: cell_b,
        energy_delta: -5e17,
        mass_delta: -1e12,
    });
    
    // Phase transition also: A → B (same pair again!)
    tm.propose_transaction(Transaction {
        source: "PhaseTransition".to_string(),
        from: cell_a,
        to: cell_b,
        energy_delta: -2e17,
        mass_delta: 5e11,
    });
    
    // Different pair: B → C
    tm.propose_transaction(Transaction {
        source: "ThermalConduction".to_string(),
        from: cell_b,
        to: cell_c,
        energy_delta: -3e17,
        mass_delta: 0.0,
    });
    
    // Another different pair: B → C (same as above!)
    tm.propose_transaction(Transaction {
        source: "ConvectionPlume".to_string(),
        from: cell_b,
        to: cell_c,
        energy_delta: -1e18,
        mass_delta: -2e12,
    });
    
    // Absolute change (no target)
    tm.propose_transaction(Transaction {
        source: "CoreRadiance".to_string(),
        from: cell_c,
        to: FlatCellId::NONE,
        energy_delta: 2e18,
        mass_delta: 0.0,
    });
    
    // Another absolute change to same cell
    tm.propose_transaction(Transaction {
        source: "SurfaceCooling".to_string(),
        from: cell_c,
        to: FlatCellId::NONE,
        energy_delta: -5e17,
        mass_delta: 0.0,
    });
    
    // Show the merging effect
    tm.demonstrate_merging_efficiency();
    
    println!("\n🎯 Benefits of Transaction Merging:");
    println!("   ✅ Fewer transactions to validate");
    println!("   ✅ Single energy/mass transfer per cell pair");
    println!("   ✅ Better cache performance");
    println!("   ✅ Simpler application logic");
    println!("   ✅ Maintains component tracking in merged names");
}
