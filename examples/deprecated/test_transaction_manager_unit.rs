use atmo_biosphere_rust::sim::transaction_manager::{
    TransactionManager, Transaction, TransactionSource, CellSnapshot
};
use h3o::CellIndex;

fn create_test_cell_snapshot(cell_id: u64, mass_kg: f64, energy_joules: f64) -> CellSnapshot {
    CellSnapshot {
        cell_index: CellIndex::try_from(cell_id).unwrap(),
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
        source_cell: CellIndex::try_from(source_cell_id).unwrap(),
        target_cell: target_cell_id.map(|id| CellIndex::try_from(id).unwrap()),
        energy_delta_joules: energy_delta,
        mass_delta_kg: mass_delta,
        description: description.to_string(),
        step_id: 0,
    }
}

fn main() {
    println!("🧪 Running Transaction Manager Unit Tests");
    
    test_transaction_manager_creation();
    test_propose_and_validate_reasonable_transactions();
    test_excessive_transactions_get_scaled();
    test_multiple_competing_transactions();
    test_mass_conservation_validation();
    test_energy_transfer_limits_per_year();
    
    println!("\n✅ All Transaction Manager Unit Tests Passed!");
}

fn test_transaction_manager_creation() {
    println!("\n🔬 Test: Transaction Manager Creation");
    
    let tm = TransactionManager::new();
    let (pending, committed) = tm.get_transaction_stats();
    
    assert_eq!(pending, 0);
    assert_eq!(committed, 0);
    assert_eq!(tm.max_mass_transfer_rate_per_year, 0.001); // 0.1%
    assert_eq!(tm.max_energy_transfer_rate_per_year, 0.005); // 0.5%
    
    println!("   ✅ Transaction manager created with correct defaults");
}

fn test_propose_and_validate_reasonable_transactions() {
    println!("\n🔬 Test: Reasonable Transactions");
    
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
    
    println!("   ✅ Reasonable transactions processed without scaling");
}

fn test_excessive_transactions_get_scaled() {
    println!("\n🔬 Test: Excessive Transactions Get Scaled");
    
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
    
    println!("   ✅ Excessive transactions properly scaled down");
    
    tm.commit_transactions(regulated);
}

fn test_multiple_competing_transactions() {
    println!("\n🔬 Test: Multiple Competing Transactions");
    
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
    
    println!("   ✅ Multiple competing transactions properly regulated ({} scaled)", scaled_count);

    tm.commit_transactions(regulated);
}

fn test_mass_conservation_validation() {
    println!("\n🔬 Test: Mass Conservation Validation");
    
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
    
    println!("   ✅ Mass conservation properly validated (net change: {:.2e})", total_mass_delta);
    
    tm.commit_transactions(regulated);
}

fn test_energy_transfer_limits_per_year() {
    println!("\n🔬 Test: Energy Transfer Limits Per Year");
    
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
        
        println!("   ✅ {:.0} years/step: {:.1}% energy transfer allowed", years_per_step, expected_max_fraction * 100.0);
        
        tm.commit_transactions(regulated);
    }
}
