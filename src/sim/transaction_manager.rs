use std::collections::HashMap;
use h3o::CellIndex;

/// Transaction source as a simple string for easy scaling
/// Components can identify themselves with any string (e.g., "ThermalConduction", "ConvectionPlume", etc.)
pub type TransactionSource = String;

/// Individual transaction record with 3D cell locations
#[derive(Debug, Clone)]
pub struct Transaction {
    pub source: TransactionSource,
    pub source_cell: CellLocation,
    pub target_cell: Option<CellLocation>, // None for absolute changes (like radiance input)
    pub energy_delta_joules: f64,          // Positive = add energy, negative = remove energy
    pub mass_delta_kg: f64,                // Positive = add mass, negative = remove mass
    pub description: String,               // Human-readable description
    pub step_id: i64,                      // Simulation step when transaction was created
}

/// Three-dimensional cell identifier for geological simulations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellLocation {
    pub layer_set_index: usize,    // Which layer set (0=crust, 1=upper mantle, etc.)
    pub h3_cell_index: CellIndex,  // H3 geographical cell
    pub depth_index: usize,        // Depth within the column (0=top, 1=deeper, etc.)
}

impl CellLocation {
    pub fn new(layer_set_index: usize, h3_cell_index: CellIndex, depth_index: usize) -> Self {
        Self {
            layer_set_index,
            h3_cell_index,
            depth_index,
        }
    }

    /// Get a human-readable description of this cell location
    pub fn description(&self) -> String {
        format!("Layer[{}]:H3[{}]:Depth[{}]",
            self.layer_set_index,
            self.h3_cell_index,
            self.depth_index)
    }
}

/// Cell state snapshot for validation
#[derive(Debug, Clone)]
pub struct CellSnapshot {
    pub location: CellLocation,
    pub mass_kg: f64,
    pub energy_joules: f64,
    pub temperature_kelvin: f64,
    pub pressure_pa: f64,
}

/// Transaction validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub scaling_factor: f64,  // Factor to scale transaction (1.0 = no scaling, 0.5 = half, etc.)
    pub reason: String,
}

/// Transaction manager for coordinating all system changes
#[derive(Debug)]
pub struct TransactionManager {
    /// Buffer of pending transactions
    pending_transactions: Vec<Transaction>,
    /// Journal of all committed transactions (for debugging)
    transaction_journal: Vec<Transaction>,
    /// Current simulation step
    current_step: i64,
    /// Maximum mass transfer rate per cell per year (0.1% = 0.001)
    max_mass_transfer_rate_per_year: f64,
    /// Maximum energy transfer rate per cell per year (0.5% = 0.005 for geological timescales)
    max_energy_transfer_rate_per_year: f64,
    /// Cell snapshots before transaction application (indexed by 3D location)
    baseline_snapshots: HashMap<CellLocation, CellSnapshot>,
}

impl TransactionManager {
    /// Create new transaction manager
    pub fn new() -> Self {
        Self {
            pending_transactions: Vec::new(),
            transaction_journal: Vec::new(),
            current_step: 0,
            max_mass_transfer_rate_per_year: 0.001,      // 0.1% per year
            max_energy_transfer_rate_per_year: 0.005,    // 0.5% per year (geological timescales)
            baseline_snapshots: HashMap::new(),
        }
    }

    /// Set current simulation step
    pub fn set_current_step(&mut self, step: i64) {
        self.current_step = step;
    }

    /// Record baseline cell states before applying transactions
    pub fn record_baseline_snapshot(&mut self, location: CellLocation, snapshot: CellSnapshot) {
        self.baseline_snapshots.insert(location, snapshot);
    }

    /// Add a proposed transaction to the buffer
    pub fn propose_transaction(&mut self, transaction: Transaction) {
        println!("📝 Proposed transaction: {} -> {:?}: {:.2e}J, {:.2e}kg ({})",
            transaction.source_cell.description(),
            transaction.target_cell.as_ref().map(|t| t.description()),
            transaction.energy_delta_joules,
            transaction.mass_delta_kg,
            transaction.description);

        self.pending_transactions.push(transaction);
    }

    /// Streamlined transaction regulation pipeline
    pub fn validate_and_regulate_transactions(&mut self, years_per_step: f64) -> Vec<Transaction> {
        self.validate_and_regulate_transactions_with_debug(years_per_step, false)
    }

    /// Transaction regulation with optional root cause analysis for debugging
    pub fn validate_and_regulate_transactions_with_debug(&mut self, years_per_step: f64, enable_root_cause: bool) -> Vec<Transaction> {
        use rayon::prelude::*;

        // 1. Determine cell load (parallel)
        let cell_loads = self.determine_cell_loads();

        // 2. Scale transactions involving overloaded cells (parallel)
        let (scaled_transactions, problematic_cells) = self.scale_overloaded_transactions(&cell_loads, years_per_step, enable_root_cause);

        // 3. Optional root cause analysis for problematic runs
        if enable_root_cause && !problematic_cells.is_empty() {
            self.analyze_root_causes(&problematic_cells, &scaled_transactions);
        }

        // 4. Return transactions ready for application to simulation
        self.pending_transactions.clear();
        scaled_transactions
    }

    /// Step 1: Determine total load on each cell (parallelized)
    fn determine_cell_loads(&mut self) -> HashMap<CellLocation, (f64, f64)> {
        use rayon::prelude::*;

        // Parallel processing of transactions to calculate cell loads
        let cell_loads: HashMap<CellLocation, (f64, f64)> = self.pending_transactions
            .par_iter()
            .fold(
                HashMap::new,
                |mut acc: HashMap<CellLocation, (f64, f64)>, transaction| {
                    // Add load to source cell
                    let (energy, mass) = acc.entry(transaction.source_cell.clone()).or_insert((0.0, 0.0));
                    *energy += transaction.energy_delta_joules.abs();
                    *mass += transaction.mass_delta_kg.abs();

                    // Add load to target cell if exists
                    if let Some(ref target) = transaction.target_cell {
                        let (energy, mass) = acc.entry(target.clone()).or_insert((0.0, 0.0));
                        *energy += transaction.energy_delta_joules.abs();
                        *mass += transaction.mass_delta_kg.abs();
                    }

                    acc
                }
            )
            .reduce(
                HashMap::new,
                |mut acc, map| {
                    for (cell, (energy, mass)) in map {
                        let (acc_energy, acc_mass) = acc.entry(cell).or_insert((0.0, 0.0));
                        *acc_energy += energy;
                        *acc_mass += mass;
                    }
                    acc
                }
            );

        cell_loads
    }

    /// Step 2: Scale transactions involving overloaded cells (parallelized)
    fn scale_overloaded_transactions(
        &self,
        cell_loads: &HashMap<CellLocation, (f64, f64)>,
        years_per_step: f64,
        enable_root_cause: bool,
    ) -> (Vec<Transaction>, HashMap<CellLocation, f64>) {
        use rayon::prelude::*;

        // Parallel calculation of scaling factors for each cell
        let scaling_factors: HashMap<CellLocation, f64> = cell_loads
            .par_iter()
            .filter_map(|(cell_location, (energy_load, mass_load))| {
                if let Some(baseline) = self.baseline_snapshots.get(cell_location) {
                    let max_energy = baseline.energy_joules * self.max_energy_transfer_rate_per_year * years_per_step;
                    let max_mass = baseline.mass_kg * self.max_mass_transfer_rate_per_year * years_per_step;

                    let energy_factor = if max_energy > 0.0 { energy_load / max_energy } else { 1.0 };
                    let mass_factor = if max_mass > 0.0 { mass_load / max_mass } else { 1.0 };
                    let violation_factor = energy_factor.max(mass_factor);

                    if violation_factor > 1.0 {
                        Some((cell_location.clone(), 1.0 / violation_factor))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Parallel scaling of transactions
        let scaled_transactions = self.pending_transactions
            .par_iter()
            .map(|transaction| {
                let mut scaled_transaction = transaction.clone();

                // Find minimum scaling factor for this transaction
                let mut scaling_factor = 1.0;

                if let Some(&source_scaling) = scaling_factors.get(&transaction.source_cell) {
                    scaling_factor = scaling_factor.min(source_scaling);
                }

                if let Some(ref target) = transaction.target_cell {
                    if let Some(&target_scaling) = scaling_factors.get(target) {
                        scaling_factor = scaling_factor.min(target_scaling);
                    }
                }

                // Apply scaling if needed
                if scaling_factor < 1.0 {
                    scaled_transaction.energy_delta_joules *= scaling_factor;
                    scaled_transaction.mass_delta_kg *= scaling_factor;

                    // Only add debug info if root cause analysis is enabled
                    if enable_root_cause {
                        scaled_transaction.description = format!("{} [SCALED {:.3}x]",
                            scaled_transaction.description, scaling_factor);
                    }
                }

                scaled_transaction
            })
            .collect();

        (scaled_transactions, scaling_factors)
    }

    /// Step 3: Re-create cell loads from scaled transactions (for verification)
    fn recreate_cell_loads(&self, transactions: &[Transaction]) -> HashMap<CellLocation, (f64, f64)> {
        use rayon::prelude::*;

        transactions
            .par_iter()
            .fold(
                HashMap::new,
                |mut acc: HashMap<CellLocation, (f64, f64)>, transaction| {
                    // Source cell load
                    let (energy, mass) = acc.entry(transaction.source_cell.clone()).or_insert((0.0, 0.0));
                    *energy += transaction.energy_delta_joules.abs();
                    *mass += transaction.mass_delta_kg.abs();

                    // Target cell load
                    if let Some(ref target) = transaction.target_cell {
                        let (energy, mass) = acc.entry(target.clone()).or_insert((0.0, 0.0));
                        *energy += transaction.energy_delta_joules.abs();
                        *mass += transaction.mass_delta_kg.abs();
                    }

                    acc
                }
            )
            .reduce(
                HashMap::new,
                |mut acc, map| {
                    for (cell, (energy, mass)) in map {
                        let (acc_energy, acc_mass) = acc.entry(cell).or_insert((0.0, 0.0));
                        *acc_energy += energy;
                        *acc_mass += mass;
                    }
                    acc
                }
            )
    }

    /// Optional root cause analysis for debugging problematic runs
    fn analyze_root_causes(
        &self,
        problematic_cells: &HashMap<CellLocation, f64>,
        scaled_transactions: &[Transaction],
    ) {
        println!("\n🔍 ROOT CAUSE ANALYSIS (Debug Mode)");
        println!("Problematic cells: {}", problematic_cells.len());

        for (cell_location, scaling_factor) in problematic_cells {
            println!("\n🚨 Cell {}: scaling factor {:.3}",
                cell_location.description(), scaling_factor);

            // Find transactions affecting this cell
            let affecting_transactions: Vec<&Transaction> = scaled_transactions
                .iter()
                .filter(|tx| {
                    tx.source_cell == *cell_location ||
                    tx.target_cell.as_ref() == Some(cell_location)
                })
                .collect();

            println!("   📋 Transactions affecting this cell:");
            for tx in affecting_transactions {
                let direction = if tx.source_cell == *cell_location {
                    "OUT"
                } else {
                    "IN"
                };

                println!("      {} {}: {:.2e}J, {:.2e}kg ({})",
                    direction, tx.source, tx.energy_delta_joules, tx.mass_delta_kg, tx.description);
            }
        }

        // Component breakdown
        let mut component_stats: HashMap<String, (usize, f64, f64)> = HashMap::new();
        for tx in scaled_transactions {
            let (count, energy, mass) = component_stats.entry(tx.source.clone()).or_insert((0, 0.0, 0.0));
            *count += 1;
            *energy += tx.energy_delta_joules.abs();
            *mass += tx.mass_delta_kg.abs();
        }

        println!("\n📊 Component Impact Summary:");
        for (component, (count, energy, mass)) in component_stats {
            println!("   {}: {} transactions, {:.2e}J, {:.2e}kg",
                component, count, energy, mass);
        }
    }

    /// Validate pre-summed totals for a specific cell (more efficient)
    fn validate_cell_totals(
        &self,
        total_energy_delta: f64,
        total_mass_delta: f64,
        baseline: &CellSnapshot,
        years_per_step: f64,
    ) -> ValidationResult {
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
            ValidationResult {
                is_valid: false,
                scaling_factor,
                reason: format!("Mass: {:.1}x limit, Energy: {:.1}x limit",
                    mass_violation_factor, energy_violation_factor),
            }
        } else {
            ValidationResult {
                is_valid: true,
                scaling_factor: 1.0,
                reason: "Within limits".to_string(),
            }
        }
    }

    /// Validate transactions for a single cell (legacy method)
    fn validate_cell_transactions(
        &self,
        transactions: &[&mut Transaction],
        baseline: &CellSnapshot,
        years_per_step: f64,
    ) -> ValidationResult {
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
            // Scale back to maximum allowed rate
            let scaling_factor = 1.0 / max_violation_factor;
            ValidationResult {
                is_valid: false,
                scaling_factor,
                reason: format!("Mass: {:.1}x limit, Energy: {:.1}x limit", 
                    mass_violation_factor, energy_violation_factor),
            }
        } else {
            ValidationResult {
                is_valid: true,
                scaling_factor: 1.0,
                reason: "Within limits".to_string(),
            }
        }
    }

    /// Commit regulated transactions to journal
    pub fn commit_transactions(&mut self, transactions: Vec<Transaction>) {
        println!("💾 Committing {} regulated transactions to journal", transactions.len());
        
        // Add to journal with step information
        for mut transaction in transactions {
            transaction.step_id = self.current_step;
            self.transaction_journal.push(transaction);
        }

        // Clear baseline snapshots for next step
        self.baseline_snapshots.clear();
    }

    /// Generate transaction report for debugging
    pub fn generate_transaction_report(&self, last_n_steps: Option<i64>) -> String {
        let filter_step = last_n_steps.map(|n| self.current_step - n);
        
        let relevant_transactions: Vec<&Transaction> = self.transaction_journal
            .iter()
            .filter(|t| filter_step.map_or(true, |step| t.step_id >= step))
            .collect();

        let mut report = String::new();
        report.push_str(&format!("📊 Transaction Report (last {} steps)\n", 
            last_n_steps.unwrap_or(self.current_step)));
        report.push_str(&format!("Total transactions: {}\n\n", relevant_transactions.len()));

        // Group by source component
        let mut by_source: HashMap<String, Vec<&Transaction>> = HashMap::new();
        for transaction in &relevant_transactions {
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
            report.push_str(&format!("  Avg energy/transaction: {:.2e} J\n",
                total_energy / transactions.len() as f64));
            report.push_str(&format!("  Avg mass/transaction: {:.2e} kg\n", total_mass));

            // Show layer distribution for this component
            let mut layer_distribution: HashMap<usize, usize> = HashMap::new();
            for transaction in &transactions {
                *layer_distribution.entry(transaction.source_cell.layer_set_index).or_insert(0) += 1;
            }
            report.push_str(&format!("  Layer distribution: {:?}\n\n", layer_distribution));
        }

        report
    }

    /// Get transaction statistics
    pub fn get_transaction_stats(&self) -> (usize, usize) {
        (self.pending_transactions.len(), self.transaction_journal.len())
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cell_snapshot(cell_id: u64, mass_kg: f64, energy_joules: f64) -> CellSnapshot {
        CellSnapshot {
            cell_index: h3o::CellIndex::try_from(cell_id).unwrap(),
            mass_kg,
            energy_joules,
            temperature_kelvin: 1500.0,
            pressure_pa: 1e9,
        }
    }

    fn create_test_transaction(
        source: TransactionSource,
        source_cell_id: u64,
        target_cell_id: Option<u64>,
        energy_delta: f64,
        mass_delta: f64,
        description: &str,
    ) -> Transaction {
        Transaction {
            source,
            source_cell: h3o::CellIndex::try_from(source_cell_id).unwrap(),
            target_cell: target_cell_id.map(|id| h3o::CellIndex::try_from(id).unwrap()),
            energy_delta_joules: energy_delta,
            mass_delta_kg: mass_delta,
            description: description.to_string(),
            step_id: 0,
        }
    }

    #[test]
    fn test_transaction_manager_creation() {
        let tm = TransactionManager::new();
        let (pending, committed) = tm.get_transaction_stats();

        assert_eq!(pending, 0);
        assert_eq!(committed, 0);
        assert_eq!(tm.max_mass_transfer_rate_per_year, 0.001); // 0.1%
        assert_eq!(tm.max_energy_transfer_rate_per_year, 0.005); // 0.5%
    }

    #[test]
    fn test_propose_and_validate_reasonable_transactions() {
        let mut tm = TransactionManager::new();
        tm.set_current_step(1);

        // Create baseline cell
        let cell_a = create_test_cell_snapshot(0x85283473fffffff_u64, 1e15, 1e20);
        tm.record_baseline_snapshot(cell_a.cell_index, cell_a.clone());

        // Propose reasonable transactions (well within limits)
        let transaction1 = create_test_transaction(
            TransactionSource::ThermalConduction,
            0x85283473fffffff_u64,
            Some(0x85283477fffffff_u64),
            -cell_a.energy_joules * 0.001, // 0.1% energy
            -cell_a.mass_kg * 0.0001,      // 0.01% mass
            "Small thermal conduction",
        );

        let transaction2 = create_test_transaction(
            TransactionSource::CoreRadiance,
            0x85283473fffffff_u64,
            None,
            cell_a.energy_joules * 0.002, // 0.2% energy input
            0.0,
            "Core radiance input",
        );

        tm.propose_transaction(transaction1);
        tm.propose_transaction(transaction2);

        let (pending, _) = tm.get_transaction_stats();
        assert_eq!(pending, 2);

        // Validate with 10,000 years per step
        let regulated = tm.validate_and_regulate_transactions(10000.0);

        assert_eq!(regulated.len(), 2);
        // All transactions should be unscaled (scaling factor = 1.0)
        for transaction in &regulated {
            assert!(!transaction.description.contains("SCALED"));
        }

        tm.commit_transactions(regulated);
        let (pending, committed) = tm.get_transaction_stats();
        assert_eq!(pending, 0);
        assert_eq!(committed, 2);
    }

    #[test]
    fn test_excessive_transactions_get_scaled() {
        let mut tm = TransactionManager::new();
        tm.set_current_step(1);

        // Create baseline cell
        let cell_a = create_test_cell_snapshot(0x85283473fffffff_u64, 1e15, 1e20);
        tm.record_baseline_snapshot(cell_a.cell_index, cell_a.clone());

        // Propose excessive transaction (way over limits)
        let excessive_transaction = create_test_transaction(
            TransactionSource::ConvectionPlume,
            0x85283473fffffff_u64,
            Some(0x85283477fffffff_u64),
            -cell_a.energy_joules * 0.1,  // 10% energy (way over 0.5% limit)
            -cell_a.mass_kg * 0.01,       // 1% mass (way over 0.1% limit)
            "Excessive plume transport",
        );

        tm.propose_transaction(excessive_transaction);

        // Validate with 10,000 years per step
        let regulated = tm.validate_and_regulate_transactions(10000.0);

        assert_eq!(regulated.len(), 1);

        // Transaction should be scaled down
        let scaled_transaction = &regulated[0];
        assert!(scaled_transaction.description.contains("SCALED"));

        // Energy and mass should be significantly reduced
        assert!(scaled_transaction.energy_delta_joules.abs() < cell_a.energy_joules * 0.1);
        assert!(scaled_transaction.mass_delta_kg.abs() < cell_a.mass_kg * 0.01);

        tm.commit_transactions(regulated);
    }

    #[test]
    fn test_multiple_competing_transactions() {
        let mut tm = TransactionManager::new();
        tm.set_current_step(1);

        // Create baseline cell
        let cell_a = create_test_cell_snapshot(0x85283473fffffff_u64, 1e15, 1e20);
        tm.record_baseline_snapshot(cell_a.cell_index, cell_a.clone());

        // Multiple components trying to modify the same cell
        let transactions = vec![
            create_test_transaction(
                TransactionSource::ThermalConduction,
                0x85283473fffffff_u64,
                Some(0x85283477fffffff_u64),
                -cell_a.energy_joules * 0.003, // 0.3%
                -cell_a.mass_kg * 0.0003,      // 0.03%
                "Conduction transfer",
            ),
            create_test_transaction(
                TransactionSource::ConvectionPlume,
                0x85283473fffffff_u64,
                Some(0x85283477fffffff_u64),
                -cell_a.energy_joules * 0.003, // 0.3%
                -cell_a.mass_kg * 0.0003,      // 0.03%
                "Plume transport",
            ),
            create_test_transaction(
                TransactionSource::SurfaceCooling,
                0x85283473fffffff_u64,
                None,
                -cell_a.energy_joules * 0.002, // 0.2%
                0.0,
                "Surface cooling",
            ),
        ];

        for transaction in transactions {
            tm.propose_transaction(transaction);
        }

        // Total: 0.8% energy, 0.06% mass - should exceed limits and be scaled
        let regulated = tm.validate_and_regulate_transactions(10000.0);

        assert_eq!(regulated.len(), 3);

        // At least some transactions should be scaled
        let scaled_count = regulated.iter()
            .filter(|t| t.description.contains("SCALED"))
            .count();
        assert!(scaled_count > 0);

        tm.commit_transactions(regulated);
    }

    #[test]
    fn test_transaction_report_generation() {
        let mut tm = TransactionManager::new();

        // Add some transactions across multiple steps
        for step in 1..=3 {
            tm.set_current_step(step);

            let cell_a = create_test_cell_snapshot(0x85283473fffffff_u64, 1e15, 1e20);
            tm.record_baseline_snapshot(cell_a.cell_index, cell_a.clone());

            let transaction = create_test_transaction(
                TransactionSource::CoreRadiance,
                0x85283473fffffff_u64,
                None,
                cell_a.energy_joules * 0.001,
                0.0,
                &format!("Step {} radiance", step),
            );

            tm.propose_transaction(transaction);
            let regulated = tm.validate_and_regulate_transactions(10000.0);
            tm.commit_transactions(regulated);
        }

        // Generate report
        let report = tm.generate_transaction_report(Some(3));

        assert!(report.contains("Transaction Report"));
        assert!(report.contains("CoreRadiance"));
        assert!(report.contains("3 transactions"));
    }

    #[test]
    fn test_mass_conservation_validation() {
        let mut tm = TransactionManager::new();
        tm.set_current_step(1);

        // Create two cells
        let cell_a = create_test_cell_snapshot(0x85283473fffffff_u64, 1e15, 1e20);
        let cell_b = create_test_cell_snapshot(0x85283477fffffff_u64, 8e14, 5e19);

        tm.record_baseline_snapshot(cell_a.cell_index, cell_a.clone());
        tm.record_baseline_snapshot(cell_b.cell_index, cell_b.clone());

        let mass_to_transfer = cell_a.mass_kg * 0.0001; // 0.01%

        // Create balanced mass transfer (conservation)
        let transfer_out = create_test_transaction(
            TransactionSource::ConvectionPlume,
            0x85283473fffffff_u64,
            Some(0x85283477fffffff_u64),
            0.0,
            -mass_to_transfer, // Remove from source
            "Mass transfer out",
        );

        let transfer_in = create_test_transaction(
            TransactionSource::ConvectionPlume,
            0x85283477fffffff_u64,
            None,
            0.0,
            mass_to_transfer, // Add to target
            "Mass transfer in",
        );

        tm.propose_transaction(transfer_out);
        tm.propose_transaction(transfer_in);

        let regulated = tm.validate_and_regulate_transactions(10000.0);

        // Both transactions should be allowed (within limits)
        assert_eq!(regulated.len(), 2);

        // Verify mass conservation
        let total_mass_delta: f64 = regulated.iter()
            .map(|t| t.mass_delta_kg)
            .sum();

        assert!((total_mass_delta.abs()) < 1e-10, "Mass conservation violated: {}", total_mass_delta);

        tm.commit_transactions(regulated);
    }

    #[test]
    fn test_energy_transfer_limits_per_year() {
        let mut tm = TransactionManager::new();
        tm.set_current_step(1);

        let cell_a = create_test_cell_snapshot(0x85283473fffffff_u64, 1e15, 1e20);
        tm.record_baseline_snapshot(cell_a.cell_index, cell_a.clone());

        // Test with different time steps
        let test_cases = vec![
            (1000.0, 0.005),   // 1000 years: 0.5% allowed
            (10000.0, 0.05),   // 10000 years: 5% allowed
            (100000.0, 0.5),   // 100000 years: 50% allowed
        ];

        for (years_per_step, expected_max_fraction) in test_cases {
            // Propose transaction at exactly the limit
            let energy_at_limit = cell_a.energy_joules * expected_max_fraction;

            let transaction = create_test_transaction(
                TransactionSource::ThermalConduction,
                0x85283473fffffff_u64,
                Some(0x85283477fffffff_u64),
                -energy_at_limit,
                0.0,
                "At limit transaction",
            );

            tm.propose_transaction(transaction);
            let regulated = tm.validate_and_regulate_transactions(years_per_step);

            // Should be allowed without scaling
            assert_eq!(regulated.len(), 1);
            assert!(!regulated[0].description.contains("SCALED"),
                "Transaction at limit should not be scaled for {} years", years_per_step);

            tm.commit_transactions(regulated);
        }
    }
}
