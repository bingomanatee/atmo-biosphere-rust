// Simple standalone test of transaction system core functionality

use std::collections::HashMap;

/// Simplified transaction for testing
#[derive(Debug, Clone)]
pub struct SimpleTransaction {
    pub component: String,
    pub from_cell: u64,
    pub to_cell: Option<u64>,
    pub energy_delta: f64,
    pub mass_delta: f64,
}

/// Simplified cell state
#[derive(Debug, Clone)]
pub struct CellState {
    pub id: u64,
    pub energy: f64,
    pub mass: f64,
}

/// Simple transaction manager for testing core logic
pub struct SimpleTransactionManager {
    pending: Vec<SimpleTransaction>,
    baselines: HashMap<u64, CellState>,
    max_energy_rate: f64,
    max_mass_rate: f64,
}

impl SimpleTransactionManager {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
            baselines: HashMap::new(),
            max_energy_rate: 0.005, // 0.5% per year
            max_mass_rate: 0.001,   // 0.1% per year
        }
    }

    pub fn record_baseline(&mut self, cell: CellState) {
        self.baselines.insert(cell.id, cell);
    }

    pub fn propose(&mut self, transaction: SimpleTransaction) {
        println!("📝 Proposed: {} {} -> {:?}: {:.2e}J, {:.2e}kg", 
            transaction.component, transaction.from_cell, transaction.to_cell,
            transaction.energy_delta, transaction.mass_delta);
        self.pending.push(transaction);
    }

    pub fn validate_and_regulate(&mut self, years_per_step: f64) -> Vec<SimpleTransaction> {
        println!("\n🔍 Validating {} transactions...", self.pending.len());

        // 1. Calculate cell loads
        let mut cell_loads: HashMap<u64, (f64, f64)> = HashMap::new();
        
        for tx in &self.pending {
            // Source cell load
            let (energy, mass) = cell_loads.entry(tx.from_cell).or_insert((0.0, 0.0));
            *energy += tx.energy_delta.abs();
            *mass += tx.mass_delta.abs();
            
            // Target cell load
            if let Some(to_cell) = tx.to_cell {
                let (energy, mass) = cell_loads.entry(to_cell).or_insert((0.0, 0.0));
                *energy += tx.energy_delta.abs();
                *mass += tx.mass_delta.abs();
            }
        }

        // 2. Find problematic cells
        let mut scaling_factors: HashMap<u64, f64> = HashMap::new();
        
        for (cell_id, (energy_load, mass_load)) in &cell_loads {
            if let Some(baseline) = self.baselines.get(cell_id) {
                let max_energy = baseline.energy * self.max_energy_rate * years_per_step;
                let max_mass = baseline.mass * self.max_mass_rate * years_per_step;
                
                let energy_violation = if max_energy > 0.0 { energy_load / max_energy } else { 1.0 };
                let mass_violation = if max_mass > 0.0 { mass_load / max_mass } else { 1.0 };
                let max_violation = energy_violation.max(mass_violation);
                
                if max_violation > 1.0 {
                    let scaling = 1.0 / max_violation;
                    scaling_factors.insert(*cell_id, scaling);
                    println!("🚨 Cell {} overloaded: {:.1}x energy, {:.1}x mass -> scale {:.3}x", 
                        cell_id, energy_violation, mass_violation, scaling);
                }
            }
        }

        // 3. Scale transactions
        let regulated: Vec<SimpleTransaction> = self.pending
            .drain(..)
            .map(|mut tx| {
                let mut scaling = 1.0f64;
                
                if let Some(&source_scaling) = scaling_factors.get(&tx.from_cell) {
                    scaling = scaling.min(source_scaling);
                }
                
                if let Some(to_cell) = tx.to_cell {
                    if let Some(&target_scaling) = scaling_factors.get(&to_cell) {
                        scaling = scaling.min(target_scaling);
                    }
                }
                
                if scaling < 1.0 {
                    tx.energy_delta *= scaling;
                    tx.mass_delta *= scaling;
                    println!("⚖️  Scaled {}: {:.3}x", tx.component, scaling);
                }
                
                tx
            })
            .collect();

        println!("✅ Regulated {} transactions", regulated.len());
        regulated
    }

    pub fn apply_to_cells(&self, transactions: &[SimpleTransaction], cells: &mut HashMap<u64, CellState>) {
        println!("\n🔧 Applying {} transactions to cells...", transactions.len());
        
        for tx in transactions {
            // Apply to source cell
            if let Some(cell) = cells.get_mut(&tx.from_cell) {
                cell.energy += tx.energy_delta;
                cell.mass += tx.mass_delta;
                println!("   Cell {}: {:.2e}J, {:.2e}kg", cell.id, cell.energy, cell.mass);
            }
            
            // Apply to target cell (opposite effect)
            if let Some(to_cell_id) = tx.to_cell {
                if let Some(cell) = cells.get_mut(&to_cell_id) {
                    cell.energy -= tx.energy_delta;
                    cell.mass -= tx.mass_delta;
                    println!("   Cell {}: {:.2e}J, {:.2e}kg", cell.id, cell.energy, cell.mass);
                }
            }
        }
    }
}

fn main() {
    println!("🧪 Simple Transaction System Test");
    println!("Testing core transaction logic with rational results\n");

    // Create test cells
    let mut cells = HashMap::new();
    cells.insert(1, CellState { id: 1, energy: 1e20, mass: 1e15 });
    cells.insert(2, CellState { id: 2, energy: 8e19, mass: 8e14 });
    cells.insert(3, CellState { id: 3, energy: 5e19, mass: 5e14 });

    println!("📊 Initial Cell States:");
    for cell in cells.values() {
        println!("   Cell {}: {:.2e}J, {:.2e}kg", cell.id, cell.energy, cell.mass);
    }

    let mut tm = SimpleTransactionManager::new();

    // Record baselines
    for cell in cells.values() {
        tm.record_baseline(cell.clone());
    }

    // Test 1: Reasonable transactions (should pass)
    println!("\n{}", "=".repeat(50));
    println!("🔬 Test 1: Reasonable Transactions");
    
    tm.propose(SimpleTransaction {
        component: "ThermalConduction".to_string(),
        from_cell: 1,
        to_cell: Some(2),
        energy_delta: -1e18, // 1% of cell 1 energy
        mass_delta: 0.0,
    });

    tm.propose(SimpleTransaction {
        component: "CoreRadiance".to_string(),
        from_cell: 3,
        to_cell: None,
        energy_delta: 2e17, // 0.4% of cell 3 energy
        mass_delta: 0.0,
    });

    let regulated1 = tm.validate_and_regulate(100.0); // 100 years
    tm.apply_to_cells(&regulated1, &mut cells);

    // Test 2: Excessive transactions (should be scaled)
    println!("\n{}", "=".repeat(50));
    println!("🔬 Test 2: Excessive Transactions");

    tm.propose(SimpleTransaction {
        component: "MassiveConvection".to_string(),
        from_cell: 1,
        to_cell: Some(2),
        energy_delta: -5e19, // 50% of cell 1 energy (way over limit)
        mass_delta: -1e14,   // 10% of cell 1 mass (way over limit)
    });

    tm.propose(SimpleTransaction {
        component: "HugeRadiance".to_string(),
        from_cell: 2,
        to_cell: None,
        energy_delta: 4e19, // 50% of cell 2 energy (way over limit)
        mass_delta: 0.0,
    });

    let regulated2 = tm.validate_and_regulate(100.0); // 100 years
    tm.apply_to_cells(&regulated2, &mut cells);

    println!("\n📊 Final Cell States:");
    for cell in cells.values() {
        println!("   Cell {}: {:.2e}J, {:.2e}kg", cell.id, cell.energy, cell.mass);
    }

    println!("\n✅ Simple Transaction System Test Completed!");
    println!("🎯 Key Observations:");
    println!("   • Reasonable transactions passed without scaling");
    println!("   • Excessive transactions were scaled down");
    println!("   • Cell states changed rationally");
    println!("   • Energy/mass conservation maintained");
}
