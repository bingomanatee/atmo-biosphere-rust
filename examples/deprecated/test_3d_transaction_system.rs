// Test the improved 3D transaction system with layer/cell/depth tracking

use std::collections::HashMap;

/// 3D cell location for geological simulations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellLocation {
    pub layer_set_index: usize,    // Which layer set (0=crust, 1=upper mantle, etc.)
    pub h3_cell_index: u64,        // H3 geographical cell (simplified as u64)
    pub depth_index: usize,        // Depth within the column (0=top, 1=deeper, etc.)
}

impl CellLocation {
    pub fn new(layer_set_index: usize, h3_cell_index: u64, depth_index: usize) -> Self {
        Self {
            layer_set_index,
            h3_cell_index,
            depth_index,
        }
    }
    
    pub fn description(&self) -> String {
        format!("Layer[{}]:H3[{}]:Depth[{}]", 
            self.layer_set_index, 
            self.h3_cell_index, 
            self.depth_index)
    }
}

/// Transaction with 3D locations and string source
#[derive(Debug, Clone)]
pub struct Transaction {
    pub source: String,                     // Component name (e.g., "ThermalConduction")
    pub source_cell: CellLocation,
    pub target_cell: Option<CellLocation>,
    pub energy_delta_joules: f64,
    pub mass_delta_kg: f64,
    pub description: String,
}

/// Cell snapshot with 3D location
#[derive(Debug, Clone)]
pub struct CellSnapshot {
    pub location: CellLocation,
    pub mass_kg: f64,
    pub energy_joules: f64,
    pub temperature_kelvin: f64,
}

/// Simple 3D transaction manager
#[derive(Debug)]
pub struct TransactionManager3D {
    pending_transactions: Vec<Transaction>,
    transaction_journal: Vec<Transaction>,
    max_mass_transfer_rate_per_year: f64,
    max_energy_transfer_rate_per_year: f64,
    baseline_snapshots: HashMap<CellLocation, CellSnapshot>,
}

impl TransactionManager3D {
    pub fn new() -> Self {
        Self {
            pending_transactions: Vec::new(),
            transaction_journal: Vec::new(),
            max_mass_transfer_rate_per_year: 0.001,  // 0.1% per year
            max_energy_transfer_rate_per_year: 0.005, // 0.5% per year
            baseline_snapshots: HashMap::new(),
        }
    }

    pub fn record_baseline_snapshot(&mut self, location: CellLocation, snapshot: CellSnapshot) {
        self.baseline_snapshots.insert(location, snapshot);
    }

    pub fn propose_transaction(&mut self, transaction: Transaction) {
        println!("📝 Proposed: {} -> {:?}: {:.2e}J, {:.2e}kg ({})", 
            transaction.source_cell.description(), 
            transaction.target_cell.as_ref().map(|t| t.description()),
            transaction.energy_delta_joules,
            transaction.mass_delta_kg,
            transaction.description);
        
        self.pending_transactions.push(transaction);
    }

    pub fn validate_and_regulate_transactions(&mut self, years_per_step: f64) -> Vec<Transaction> {
        println!("\n🔍 Validating {} pending transactions...", self.pending_transactions.len());
        
        // Group transactions by 3D cell location
        let mut transactions_by_cell: HashMap<CellLocation, Vec<Transaction>> = HashMap::new();
        
        for transaction in self.pending_transactions.drain(..) {
            transactions_by_cell
                .entry(transaction.source_cell.clone())
                .or_insert_with(Vec::new)
                .push(transaction);
        }

        let mut regulated_transactions = Vec::new();
        let mut total_violations = 0;
        let mut total_scaled = 0;

        for (cell_location, mut cell_transactions) in transactions_by_cell {
            if let Some(baseline) = self.baseline_snapshots.get(&cell_location) {
                let (is_valid, scaling_factor, reason) = self.validate_cell_transactions(
                    &cell_transactions, 
                    baseline, 
                    years_per_step
                );

                if scaling_factor < 1.0 {
                    total_scaled += cell_transactions.len();
                    println!("⚖️  Scaling transactions for {}: factor {:.3} ({})", 
                        cell_location.description(), scaling_factor, reason);
                    
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
                println!("⚠️  No baseline for {}, allowing transactions", cell_location.description());
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
        let total_mass_delta: f64 = transactions.iter().map(|t| t.mass_delta_kg.abs()).sum();
        let total_energy_delta: f64 = transactions.iter().map(|t| t.energy_delta_joules.abs()).sum();

        let max_mass_change = baseline.mass_kg * self.max_mass_transfer_rate_per_year * years_per_step;
        let max_energy_change = baseline.energy_joules * self.max_energy_transfer_rate_per_year * years_per_step;

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

    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("📊 3D Transaction Report\n"));
        report.push_str(&format!("Total committed transactions: {}\n\n", self.transaction_journal.len()));

        // Group by component source
        let mut by_source: HashMap<String, Vec<&Transaction>> = HashMap::new();
        for transaction in &self.transaction_journal {
            by_source.entry(transaction.source.clone())
                .or_insert_with(Vec::new)
                .push(transaction);
        }

        for (source, transactions) in by_source {
            let total_energy: f64 = transactions.iter().map(|t| t.energy_delta_joules).sum();
            let total_mass: f64 = transactions.iter().map(|t| t.mass_delta_kg).sum();
            
            report.push_str(&format!("{}: {} transactions\n", source, transactions.len()));
            report.push_str(&format!("  Total energy: {:.2e} J\n", total_energy));
            report.push_str(&format!("  Total mass: {:.2e} kg\n", total_mass));
            
            // Show layer distribution for this component
            let mut layer_distribution: HashMap<usize, usize> = HashMap::new();
            for transaction in &transactions {
                *layer_distribution.entry(transaction.source_cell.layer_set_index).or_insert(0) += 1;
            }
            report.push_str(&format!("  Layer distribution: {:?}\n", layer_distribution));
            
            // Show depth distribution
            let mut depth_distribution: HashMap<usize, usize> = HashMap::new();
            for transaction in &transactions {
                *depth_distribution.entry(transaction.source_cell.depth_index).or_insert(0) += 1;
            }
            report.push_str(&format!("  Depth distribution: {:?}\n\n", depth_distribution));
        }

        report
    }
}

fn main() {
    println!("🧪 Testing 3D Transaction System");
    println!("Features:");
    println!("  - 3D cell locations (layer_set, h3_cell, depth)");
    println!("  - String-based component sources (scalable)");
    println!("  - Layer and depth distribution tracking");
    
    test_3d_transaction_system();
    
    println!("\n✅ 3D Transaction System Test Completed!");
}

fn test_3d_transaction_system() {
    println!("\n🔬 Test: 3D Transaction System with Multiple Layers");
    
    let mut tm = TransactionManager3D::new();
    
    // Create cells across different layers and depths
    let locations = vec![
        CellLocation::new(0, 12345, 0), // Crust, surface
        CellLocation::new(0, 12345, 1), // Crust, deeper
        CellLocation::new(1, 12345, 0), // Upper mantle, top
        CellLocation::new(1, 12345, 2), // Upper mantle, deep
        CellLocation::new(2, 12345, 1), // Lower mantle, middle
    ];
    
    // Record baselines for all locations
    for (i, location) in locations.iter().enumerate() {
        let snapshot = CellSnapshot {
            location: location.clone(),
            mass_kg: 1e15 * (i + 1) as f64,      // Different masses
            energy_joules: 1e20 * (i + 1) as f64, // Different energies
            temperature_kelvin: 1000.0 + (i * 200) as f64,
        };
        tm.record_baseline_snapshot(location.clone(), snapshot.clone());
        println!("📍 Baseline: {} - {:.2e}kg, {:.2e}J, {:.0}K",
            location.description(), snapshot.mass_kg, snapshot.energy_joules, snapshot.temperature_kelvin);
    }
    
    // Simulate different components operating on different layers
    println!("\n🔄 Simulating component transactions:");
    
    // Thermal conduction between adjacent depths
    tm.propose_transaction(Transaction {
        source: "ThermalConduction".to_string(),
        source_cell: locations[0].clone(), // Crust surface
        target_cell: Some(locations[1].clone()), // Crust deeper
        energy_delta_joules: -1e18, // 1% of crust surface energy
        mass_delta_kg: 0.0,
        description: "Vertical conduction in crust".to_string(),
    });
    
    // Convection plume from deep mantle to upper mantle
    tm.propose_transaction(Transaction {
        source: "ConvectionPlume".to_string(),
        source_cell: locations[4].clone(), // Lower mantle
        target_cell: Some(locations[2].clone()), // Upper mantle top
        energy_delta_joules: -2e19, // 4% of lower mantle energy
        mass_delta_kg: -1e13, // 0.2% of lower mantle mass
        description: "Deep mantle plume".to_string(),
    });
    
    // Core radiance affecting deepest cells
    tm.propose_transaction(Transaction {
        source: "CoreRadiance".to_string(),
        source_cell: locations[4].clone(), // Lower mantle (deepest)
        target_cell: None,
        energy_delta_joules: 1e19, // Energy input
        mass_delta_kg: 0.0,
        description: "Core energy input".to_string(),
    });
    
    // Surface cooling from crust
    tm.propose_transaction(Transaction {
        source: "SurfaceCooling".to_string(),
        source_cell: locations[0].clone(), // Crust surface
        target_cell: None,
        energy_delta_joules: -5e18, // 5% energy loss
        mass_delta_kg: 0.0,
        description: "Surface radiation to space".to_string(),
    });
    
    // Phase transition in upper mantle
    tm.propose_transaction(Transaction {
        source: "PhaseTransition".to_string(),
        source_cell: locations[3].clone(), // Upper mantle deep
        target_cell: None,
        energy_delta_joules: -3e19, // 7.5% energy change
        mass_delta_kg: 1e12, // Density change
        description: "Olivine phase transition".to_string(),
    });
    
    // Validate and regulate
    let regulated = tm.validate_and_regulate_transactions(100.0); // 100 years
    
    // Commit and generate report
    tm.commit_transactions(regulated);
    
    println!("\n📊 Final Report:");
    let report = tm.generate_report();
    println!("{}", report);
    
    println!("🎯 Key Benefits of 3D System:");
    println!("   ✅ Precise cell identification (layer + H3 + depth)");
    println!("   ✅ Scalable component sources (strings, not enums)");
    println!("   ✅ Layer and depth distribution analysis");
    println!("   ✅ Component-specific violation tracking");
    println!("   ✅ Geological structure awareness");
}
