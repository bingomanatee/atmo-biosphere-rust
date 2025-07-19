use atmo_biosphere_rust::sim::transaction_manager::{TransactionManager, Transaction, TransactionSource, CellSnapshot};
use h3o::CellIndex;

fn main() {
    println!("🧪 Testing Transaction Management System");
    
    let mut transaction_manager = TransactionManager::new();
    transaction_manager.set_current_step(1);
    
    // Create test cell indices
    let cell_a = CellIndex::try_from(0x85283473fffffff_u64).unwrap();
    let cell_b = CellIndex::try_from(0x85283477fffffff_u64).unwrap();
    
    // Record baseline snapshots
    let baseline_a = CellSnapshot {
        cell_index: cell_a,
        mass_kg: 1e15,           // 1 petagram
        energy_joules: 1e20,     // 100 exajoules
        temperature_kelvin: 1500.0,
        pressure_pa: 1e9,        // 1 GPa
    };
    
    let baseline_b = CellSnapshot {
        cell_index: cell_b,
        mass_kg: 8e14,           // 0.8 petagram
        energy_joules: 5e19,     // 50 exajoules
        temperature_kelvin: 1200.0,
        pressure_pa: 5e8,        // 0.5 GPa
    };
    
    transaction_manager.record_baseline_snapshot(cell_a, baseline_a.clone());
    transaction_manager.record_baseline_snapshot(cell_b, baseline_b.clone());
    
    println!("\n📊 Baseline cell states recorded:");
    println!("   Cell A: {:.2e} kg, {:.2e} J, {:.0}K", 
        baseline_a.mass_kg, baseline_a.energy_joules, baseline_a.temperature_kelvin);
    println!("   Cell B: {:.2e} kg, {:.2e} J, {:.0}K", 
        baseline_b.mass_kg, baseline_b.energy_joules, baseline_b.temperature_kelvin);
    
    // Test 1: Reasonable transactions (should pass)
    println!("\n🧪 Test 1: Reasonable transactions");
    
    // Small thermal conduction (0.05% mass, 5% energy)
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::ThermalConduction,
        source_cell: cell_a,
        target_cell: Some(cell_b),
        energy_delta_joules: -baseline_a.energy_joules * 0.05,  // Remove 5% energy from A
        mass_delta_kg: -baseline_a.mass_kg * 0.0005,            // Remove 0.05% mass from A
        description: "Thermal conduction A->B".to_string(),
        step_id: 1,
    });
    
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::ThermalConduction,
        source_cell: cell_b,
        target_cell: Some(cell_a),
        energy_delta_joules: baseline_a.energy_joules * 0.05,   // Add 5% energy to B
        mass_delta_kg: baseline_a.mass_kg * 0.0005,             // Add 0.05% mass to B
        description: "Thermal conduction A->B (target)".to_string(),
        step_id: 1,
    });
    
    // Core radiance (energy only)
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::CoreRadiance,
        source_cell: cell_a,
        target_cell: None,
        energy_delta_joules: baseline_a.energy_joules * 0.02,   // Add 2% energy
        mass_delta_kg: 0.0,
        description: "Core radiance input".to_string(),
        step_id: 1,
    });
    
    let years_per_step = 10000.0; // 10,000 years per step
    let regulated_transactions = transaction_manager.validate_and_regulate_transactions(years_per_step);
    
    println!("✅ Reasonable transactions processed: {} transactions", regulated_transactions.len());
    transaction_manager.commit_transactions(regulated_transactions);
    
    // Test 2: Excessive transactions (should be scaled back)
    println!("\n🧪 Test 2: Excessive transactions (should be scaled)");
    
    // Excessive plume transfer (10% mass, 50% energy - way over limits)
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::ConvectionPlume,
        source_cell: cell_a,
        target_cell: Some(cell_b),
        energy_delta_joules: -baseline_a.energy_joules * 0.5,   // Remove 50% energy
        mass_delta_kg: -baseline_a.mass_kg * 0.1,               // Remove 10% mass
        description: "Excessive plume transport".to_string(),
        step_id: 2,
    });
    
    transaction_manager.set_current_step(2);
    transaction_manager.record_baseline_snapshot(cell_a, baseline_a.clone());
    
    let regulated_transactions = transaction_manager.validate_and_regulate_transactions(years_per_step);
    
    println!("⚖️  Excessive transactions regulated: {} transactions", regulated_transactions.len());
    for transaction in &regulated_transactions {
        if transaction.description.contains("SCALED") {
            println!("   Scaled: {}", transaction.description);
        }
    }
    transaction_manager.commit_transactions(regulated_transactions);
    
    // Test 3: Multiple competing transactions
    println!("\n🧪 Test 3: Multiple competing transactions");
    
    // Multiple components trying to modify the same cell
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::ThermalConduction,
        source_cell: cell_a,
        target_cell: Some(cell_b),
        energy_delta_joules: -baseline_a.energy_joules * 0.08,
        mass_delta_kg: -baseline_a.mass_kg * 0.0008,
        description: "Conduction transfer".to_string(),
        step_id: 3,
    });
    
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::ConvectionPlume,
        source_cell: cell_a,
        target_cell: Some(cell_b),
        energy_delta_joules: -baseline_a.energy_joules * 0.06,
        mass_delta_kg: -baseline_a.mass_kg * 0.0006,
        description: "Plume transport".to_string(),
        step_id: 3,
    });
    
    transaction_manager.propose_transaction(Transaction {
        source: TransactionSource::SurfaceCooling,
        source_cell: cell_a,
        target_cell: None,
        energy_delta_joules: -baseline_a.energy_joules * 0.04,
        mass_delta_kg: 0.0,
        description: "Surface cooling".to_string(),
        step_id: 3,
    });
    
    transaction_manager.set_current_step(3);
    transaction_manager.record_baseline_snapshot(cell_a, baseline_a.clone());
    
    let regulated_transactions = transaction_manager.validate_and_regulate_transactions(years_per_step);
    
    println!("🔄 Multiple competing transactions: {} transactions", regulated_transactions.len());
    transaction_manager.commit_transactions(regulated_transactions);
    
    // Generate final report
    println!("\n📊 Final Transaction Report:");
    let report = transaction_manager.generate_transaction_report(Some(3));
    println!("{}", report);
    
    let (pending, committed) = transaction_manager.get_transaction_stats();
    println!("📈 Final stats: {} pending, {} committed", pending, committed);
    
    println!("\n✅ Transaction management system test completed!");
}
