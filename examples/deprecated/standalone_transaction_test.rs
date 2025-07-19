// Standalone transaction manager test
// This tests the transaction management concepts without depending on the full library

use std::collections::HashMap;

/// Transaction types for tracking origination
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransactionSource {
    ThermalConduction,
    ConvectionPlume,
    CoreRadiance,
    SurfaceCooling,
}

/// Individual transaction record
#[derive(Debug, Clone)]
pub struct Transaction {
    pub source: TransactionSource,
    pub source_cell_id: u64,
    pub target_cell_id: Option<u64>,
    pub energy_delta_joules: f64,
    pub mass_delta_kg: f64,
    pub description: String,
}

/// Cell state snapshot for validation
#[derive(Debug, Clone)]
pub struct CellSnapshot {
    pub cell_id: u64,
    pub mass_kg: f64,
    pub energy_joules: f64,
    pub temperature_kelvin: f64,
}

/// Simple transaction manager for testing
#[derive(Debug)]
pub struct SimpleTransactionManager {
    pending_transactions: Vec<Transaction>,
    transaction_journal: Vec<Transaction>,
    max_mass_transfer_rate_per_year: f64,
    max_energy_transfer_rate_per_year: f64,
    baseline_snapshots: HashMap<u64, CellSnapshot>,
}

impl SimpleTransactionManager {
    pub fn new() -> Self {
        Self {
            pending_transactions: Vec::new(),
            transaction_journal: Vec::new(),
            max_mass_transfer_rate_per_year: 0.001,  // 0.1% per year
            max_energy_transfer_rate_per_year: 0.005, // 0.5% per year
            baseline_snapshots: HashMap::new(),
        }
    }

    pub fn record_baseline_snapshot(&mut self, cell_id: u64, snapshot: CellSnapshot) {
        self.baseline_snapshots.insert(cell_id, snapshot);
    }

    pub fn propose_transaction(&mut self, transaction: Transaction) {
        println!("📝 Proposed: {} -> {:?}: {:.2e}J, {:.2e}kg ({})", 
            transaction.source_cell_id, 
            transaction.target_cell_id,
            transaction.energy_delta_joules,
            transaction.mass_delta_kg,
            transaction.description);
        
        self.pending_transactions.push(transaction);
    }

    pub fn validate_and_regulate_transactions(&mut self, years_per_step: f64) -> Vec<Transaction> {
        println!("\n🔍 Validating {} pending transactions...", self.pending_transactions.len());
        
        let mut regulated_transactions = Vec::new();
        let mut total_violations = 0;
        let mut total_scaled = 0;

        // Group transactions by source cell
        let mut transactions_by_cell: HashMap<u64, Vec<Transaction>> = HashMap::new();
        
        for transaction in self.pending_transactions.drain(..) {
            transactions_by_cell
                .entry(transaction.source_cell_id)
                .or_insert_with(Vec::new)
                .push(transaction);
        }

        // Validate each cell's transactions
        for (cell_id, mut cell_transactions) in transactions_by_cell {
            if let Some(baseline) = self.baseline_snapshots.get(&cell_id) {
                let (is_valid, scaling_factor, reason) = self.validate_cell_transactions(
                    &cell_transactions, 
                    baseline, 
                    years_per_step
                );

                if scaling_factor < 1.0 {
                    total_scaled += cell_transactions.len();
                    println!("⚖️  Scaling transactions for cell {}: factor {:.3} ({})", 
                        cell_id, scaling_factor, reason);
                    
                    // Apply scaling
                    for transaction in &mut cell_transactions {
                        transaction.energy_delta_joules *= scaling_factor;
                        transaction.mass_delta_kg *= scaling_factor;
                        transaction.description = format!("{} [SCALED {:.3}x]", 
                            transaction.description, scaling_factor);
                    }
                }

                if !is_valid {
                    total_violations += 1;
                }

                regulated_transactions.extend(cell_transactions);
            } else {
                println!("⚠️  No baseline for cell {}, allowing transactions", cell_id);
                regulated_transactions.extend(cell_transactions);
            }
        }

        println!("📊 Regulation summary: {} violations, {} scaled", total_violations, total_scaled);
        regulated_transactions
    }

    fn validate_cell_transactions(
        &self,
        transactions: &[Transaction],
        baseline: &CellSnapshot,
        years_per_step: f64,
    ) -> (bool, f64, String) {
        // Calculate total proposed changes
        let total_mass_delta: f64 = transactions.iter()
            .map(|t| t.mass_delta_kg.abs())
            .sum();
        let total_energy_delta: f64 = transactions.iter()
            .map(|t| t.energy_delta_joules.abs())
            .sum();

        // Calculate maximum allowed changes per step
        let max_mass_change = baseline.mass_kg * self.max_mass_transfer_rate_per_year * years_per_step;
        let max_energy_change = baseline.energy_joules * self.max_energy_transfer_rate_per_year * years_per_step;

        // Check if changes exceed limits
        let mass_violation_factor = if max_mass_change > 0.0 {
            total_mass_delta / max_mass_change
        } else {
            1.0
        };

        let energy_violation_factor = if max_energy_change > 0.0 {
            total_energy_delta / max_energy_change
        } else {
            1.0
        };

        let max_violation_factor = mass_violation_factor.max(energy_violation_factor);

        if max_violation_factor > 1.0 {
            let scaling_factor = 1.0 / max_violation_factor;
            (false, scaling_factor, format!("Mass: {:.1}x limit, Energy: {:.1}x limit", 
                mass_violation_factor, energy_violation_factor))
        } else {
            (true, 1.0, "Within limits".to_string())
        }
    }

    pub fn commit_transactions(&mut self, transactions: Vec<Transaction>) {
        println!("💾 Committing {} transactions to journal", transactions.len());
        self.transaction_journal.extend(transactions);
        self.baseline_snapshots.clear();
    }

    pub fn get_stats(&self) -> (usize, usize) {
        (self.pending_transactions.len(), self.transaction_journal.len())
    }

    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("📊 Transaction Report\n"));
        report.push_str(&format!("Total committed transactions: {}\n\n", self.transaction_journal.len()));

        // Group by source
        let mut by_source: HashMap<TransactionSource, Vec<&Transaction>> = HashMap::new();
        for transaction in &self.transaction_journal {
            by_source.entry(transaction.source.clone())
                .or_insert_with(Vec::new)
                .push(transaction);
        }

        for (source, transactions) in by_source {
            let total_energy: f64 = transactions.iter().map(|t| t.energy_delta_joules).sum();
            let total_mass: f64 = transactions.iter().map(|t| t.mass_delta_kg).sum();
            
            report.push_str(&format!("{:?}: {} transactions\n", source, transactions.len()));
            report.push_str(&format!("  Total energy: {:.2e} J\n", total_energy));
            report.push_str(&format!("  Total mass: {:.2e} kg\n\n", total_mass));
        }

        report
    }
}

fn main() {
    println!("🧪 Running Standalone Transaction Manager Tests");
    
    test_basic_functionality();
    test_reasonable_transactions();
    test_excessive_transactions();
    test_multiple_competing_transactions();
    test_mass_conservation();
    test_energy_limits_by_timescale();
    
    println!("\n✅ All Standalone Transaction Tests Passed!");
}

fn test_basic_functionality() {
    println!("\n🔬 Test: Basic Functionality");
    
    let tm = SimpleTransactionManager::new();
    let (pending, committed) = tm.get_stats();
    
    assert_eq!(pending, 0);
    assert_eq!(committed, 0);
    assert_eq!(tm.max_mass_transfer_rate_per_year, 0.001);
    assert_eq!(tm.max_energy_transfer_rate_per_year, 0.005);
    
    println!("   ✅ Transaction manager created with correct limits");
}

fn test_reasonable_transactions() {
    println!("\n🔬 Test: Reasonable Transactions");
    
    let mut tm = SimpleTransactionManager::new();
    
    // Create baseline cell
    let cell_a = CellSnapshot {
        cell_id: 1,
        mass_kg: 1e15,
        energy_joules: 1e20,
        temperature_kelvin: 1500.0,
    };
    tm.record_baseline_snapshot(1, cell_a.clone());

    // Small reasonable transactions
    tm.propose_transaction(Transaction {
        source: TransactionSource::ThermalConduction,
        source_cell_id: 1,
        target_cell_id: Some(2),
        energy_delta_joules: -cell_a.energy_joules * 0.001, // 0.1%
        mass_delta_kg: -cell_a.mass_kg * 0.0001,           // 0.01%
        description: "Small conduction".to_string(),
    });

    let regulated = tm.validate_and_regulate_transactions(10000.0);
    assert_eq!(regulated.len(), 1);
    assert!(!regulated[0].description.contains("SCALED"));
    
    tm.commit_transactions(regulated);
    println!("   ✅ Reasonable transactions processed without scaling");
}

fn test_excessive_transactions() {
    println!("\n🔬 Test: Excessive Transactions");
    
    let mut tm = SimpleTransactionManager::new();
    
    let cell_a = CellSnapshot {
        cell_id: 1,
        mass_kg: 1e15,
        energy_joules: 1e20,
        temperature_kelvin: 1500.0,
    };
    tm.record_baseline_snapshot(1, cell_a.clone());

    // Excessive transaction
    tm.propose_transaction(Transaction {
        source: TransactionSource::ConvectionPlume,
        source_cell_id: 1,
        target_cell_id: Some(2),
        energy_delta_joules: -cell_a.energy_joules * 0.1,  // 10% (way over 0.5% limit)
        mass_delta_kg: -cell_a.mass_kg * 0.01,             // 1% (way over 0.1% limit)
        description: "Excessive plume".to_string(),
    });

    let regulated = tm.validate_and_regulate_transactions(10000.0);
    assert_eq!(regulated.len(), 1);
    assert!(regulated[0].description.contains("SCALED"));
    
    tm.commit_transactions(regulated);
    println!("   ✅ Excessive transactions properly scaled down");
}

fn test_multiple_competing_transactions() {
    println!("\n🔬 Test: Multiple Competing Transactions");
    
    let mut tm = SimpleTransactionManager::new();
    
    let cell_a = CellSnapshot {
        cell_id: 1,
        mass_kg: 1e15,
        energy_joules: 1e20,
        temperature_kelvin: 1500.0,
    };
    tm.record_baseline_snapshot(1, cell_a.clone());

    // Multiple transactions that together exceed limits
    let transactions = vec![
        Transaction {
            source: TransactionSource::ThermalConduction,
            source_cell_id: 1,
            target_cell_id: Some(2),
            energy_delta_joules: -cell_a.energy_joules * 0.003, // 0.3%
            mass_delta_kg: -cell_a.mass_kg * 0.0003,           // 0.03%
            description: "Conduction".to_string(),
        },
        Transaction {
            source: TransactionSource::ConvectionPlume,
            source_cell_id: 1,
            target_cell_id: Some(3),
            energy_delta_joules: -cell_a.energy_joules * 0.003, // 0.3%
            mass_delta_kg: -cell_a.mass_kg * 0.0003,           // 0.03%
            description: "Plume".to_string(),
        },
        Transaction {
            source: TransactionSource::SurfaceCooling,
            source_cell_id: 1,
            target_cell_id: None,
            energy_delta_joules: -cell_a.energy_joules * 0.002, // 0.2%
            mass_delta_kg: 0.0,
            description: "Cooling".to_string(),
        },
    ];

    for transaction in transactions {
        tm.propose_transaction(transaction);
    }

    // Total: 0.8% energy, 0.06% mass - should be scaled
    let regulated = tm.validate_and_regulate_transactions(10000.0);
    assert_eq!(regulated.len(), 3);
    
    let scaled_count = regulated.iter()
        .filter(|t| t.description.contains("SCALED"))
        .count();
    assert!(scaled_count > 0);
    
    tm.commit_transactions(regulated);
    println!("   ✅ Multiple competing transactions regulated ({} scaled)", scaled_count);
}

fn test_mass_conservation() {
    println!("\n🔬 Test: Mass Conservation");
    
    let mut tm = SimpleTransactionManager::new();
    
    let cell_a = CellSnapshot { cell_id: 1, mass_kg: 1e15, energy_joules: 1e20, temperature_kelvin: 1500.0 };
    let cell_b = CellSnapshot { cell_id: 2, mass_kg: 8e14, energy_joules: 5e19, temperature_kelvin: 1200.0 };
    
    tm.record_baseline_snapshot(1, cell_a.clone());
    tm.record_baseline_snapshot(2, cell_b.clone());

    let mass_transfer = cell_a.mass_kg * 0.0001; // 0.01%

    // Balanced mass transfer
    tm.propose_transaction(Transaction {
        source: TransactionSource::ConvectionPlume,
        source_cell_id: 1,
        target_cell_id: Some(2),
        energy_delta_joules: 0.0,
        mass_delta_kg: -mass_transfer,
        description: "Mass out".to_string(),
    });

    tm.propose_transaction(Transaction {
        source: TransactionSource::ConvectionPlume,
        source_cell_id: 2,
        target_cell_id: None,
        energy_delta_joules: 0.0,
        mass_delta_kg: mass_transfer,
        description: "Mass in".to_string(),
    });

    let regulated = tm.validate_and_regulate_transactions(10000.0);
    assert_eq!(regulated.len(), 2);
    
    let total_mass_delta: f64 = regulated.iter().map(|t| t.mass_delta_kg).sum();
    assert!(total_mass_delta.abs() < 1e-10);
    
    tm.commit_transactions(regulated);
    println!("   ✅ Mass conservation verified (net: {:.2e})", total_mass_delta);
}

fn test_energy_limits_by_timescale() {
    println!("\n🔬 Test: Energy Limits by Timescale");
    
    let mut tm = SimpleTransactionManager::new();
    
    let cell_a = CellSnapshot {
        cell_id: 1,
        mass_kg: 1e15,
        energy_joules: 1e20,
        temperature_kelvin: 1500.0,
    };

    let test_cases = vec![
        (1000.0, 0.005),   // 1000 years: 0.5%
        (10000.0, 0.05),   // 10000 years: 5%
        (100000.0, 0.5),   // 100000 years: 50%
    ];

    for (years_per_step, expected_max_fraction) in test_cases {
        tm.record_baseline_snapshot(1, cell_a.clone());
        
        let energy_at_limit = cell_a.energy_joules * expected_max_fraction;
        
        tm.propose_transaction(Transaction {
            source: TransactionSource::ThermalConduction,
            source_cell_id: 1,
            target_cell_id: Some(2),
            energy_delta_joules: -energy_at_limit,
            mass_delta_kg: 0.0,
            description: "At limit".to_string(),
        });

        let regulated = tm.validate_and_regulate_transactions(years_per_step);
        assert_eq!(regulated.len(), 1);
        assert!(!regulated[0].description.contains("SCALED"));
        
        tm.commit_transactions(regulated);
        println!("   ✅ {:.0} years: {:.1}% energy allowed", years_per_step, expected_max_fraction * 100.0);
    }
}
