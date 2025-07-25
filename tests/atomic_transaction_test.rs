use atmo_biosphere_rust::transaction_manager::{AtomicTransaction, TransactionManager, CellLocation};
use h3o::CellIndex;

#[test]
fn test_atomic_transaction_creation() {
    // Test that atomic transactions can be created and are valid
    let cell_a = CellLocation::new(0, CellIndex::try_from(0x85283473fffffff_u64).unwrap(), 0);
    let cell_b = CellLocation::new(0, CellIndex::try_from(0x85283477fffffff_u64).unwrap(), 0);
    
    // Test transfer transaction
    let transfer = AtomicTransaction::transfer(
        "TestComponent".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        1000.0, // 1000 J
        0.1,    // 0.1 kg
        "Test energy transfer".to_string(),
    );
    
    assert!(transfer.is_ok(), "Transfer transaction should be valid");
    
    // Test inject transaction
    let inject = AtomicTransaction::inject(
        "TestComponent".to_string(),
        cell_a.clone(),
        500.0, // 500 J
        0.05,  // 0.05 kg
        "Test energy injection".to_string(),
    );
    
    assert!(inject.is_ok(), "Inject transaction should be valid");
    
    // Test extract transaction
    let extract = AtomicTransaction::extract(
        "TestComponent".to_string(),
        cell_a.clone(),
        200.0, // 200 J
        0.02,  // 0.02 kg
        "Test energy extraction".to_string(),
    );
    
    assert!(extract.is_ok(), "Extract transaction should be valid");
}

#[test]
fn test_atomic_transaction_validation() {
    let cell_a = CellLocation::new(0, CellIndex::try_from(0x85283473fffffff_u64).unwrap(), 0);
    
    // Test that negative energy is rejected
    let invalid_transfer = AtomicTransaction::transfer(
        "TestComponent".to_string(),
        cell_a.clone(),
        cell_a.clone(),
        -1000.0, // Negative energy should be rejected
        0.1,
        "Invalid negative energy".to_string(),
    );
    
    assert!(invalid_transfer.is_err(), "Negative energy should be rejected");
    
    // Test that negative mass is rejected
    let invalid_mass = AtomicTransaction::inject(
        "TestComponent".to_string(),
        cell_a.clone(),
        1000.0,
        -0.1, // Negative mass should be rejected
        "Invalid negative mass".to_string(),
    );
    
    assert!(invalid_mass.is_err(), "Negative mass should be rejected");
}

#[test]
fn test_transaction_manager_atomic_operations() {
    let mut tm = TransactionManager::new();
    let cell_a = CellLocation::new(0, CellIndex::try_from(0x85283473fffffff_u64).unwrap(), 0);
    let cell_b = CellLocation::new(0, CellIndex::try_from(0x85283477fffffff_u64).unwrap(), 0);
    
    // Test proposing atomic transactions
    let result = tm.propose_transfer(
        "TestComponent".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        1000.0,
        0.1,
        "Test transfer via manager".to_string(),
    );
    
    assert!(result.is_ok(), "Transaction manager should accept valid transfer");
    
    let result = tm.propose_injection(
        "TestComponent".to_string(),
        cell_a.clone(),
        500.0,
        0.05,
        "Test injection via manager".to_string(),
    );
    
    assert!(result.is_ok(), "Transaction manager should accept valid injection");
    
    // Check that transactions are pending
    let (pending_count, _) = tm.get_transaction_stats();
    assert_eq!(pending_count, 2, "Should have 2 pending transactions");
    
    // Test validation and regulation
    let validated = tm.validate_and_regulate_transactions(1.0); // 1 year per step
    assert_eq!(validated.len(), 2, "Should validate 2 transactions");
    
    println!("✅ Atomic transaction system working correctly!");
}
