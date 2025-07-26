use atmo_biosphere_rust::transaction_manager::{AtomicTransaction, AtomicOperation, TransactionManager};
use atmo_biosphere_rust::cell_location::CellLocation;
use h3o::CellIndex;

fn create_test_location(layer_set: usize, h3_index: u64, depth: usize) -> CellLocation {
    CellLocation::new(
        layer_set,
        CellIndex::try_from(h3_index).unwrap(),
        depth,
    )
}

#[test]
fn test_atomic_transfer_creation() {
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    let cell_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Test valid transfer creation
    let transfer = AtomicTransaction::transfer(
        "RadiativeTransfer".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        100.0, // 100J
        0.0,   // No mass
        "Heat transfer".to_string(),
    );
    
    assert!(transfer.is_ok());
    let transfer = transfer.unwrap();
    
    // Verify it's conservative (transfers don't create/destroy energy)
    assert!(transfer.is_conservative());
    
    // Verify affected cells
    let affected = transfer.affected_cells();
    assert_eq!(affected.len(), 2);
    assert!(affected.contains(&cell_a));
    assert!(affected.contains(&cell_b));
}

#[test]
fn test_atomic_transfer_validation() {
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    let cell_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Test invalid transfer (negative energy)
    let invalid_transfer = AtomicTransaction::transfer(
        "RadiativeTransfer".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        -100.0, // Negative energy - INVALID
        0.0,
        "Invalid transfer".to_string(),
    );
    
    assert!(invalid_transfer.is_err());
    assert!(invalid_transfer.unwrap_err().contains("positive"));
}

#[test]
fn test_atomic_injection_creation() {
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    
    // Test valid injection creation
    let injection = AtomicTransaction::inject(
        "CoreRadiance".to_string(),
        cell_a.clone(),
        50.0, // 50J injection
        0.0,  // No mass
        "Core radiance energy".to_string(),
    );
    
    assert!(injection.is_ok());
    let injection = injection.unwrap();
    
    // Verify it's NOT conservative (injections create energy)
    assert!(!injection.is_conservative());
    
    // Verify affected cells
    let affected = injection.affected_cells();
    assert_eq!(affected.len(), 1);
    assert!(affected.contains(&cell_a));
}

#[test]
fn test_atomic_extraction_creation() {
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    
    // Test valid extraction creation
    let extraction = AtomicTransaction::extract(
        "SpaceRadiation".to_string(),
        cell_a.clone(),
        25.0, // 25J extraction
        0.0,  // No mass
        "Energy radiated to space".to_string(),
    );
    
    assert!(extraction.is_ok());
    let extraction = extraction.unwrap();
    
    // Verify it's NOT conservative (extractions destroy energy)
    assert!(!extraction.is_conservative());
    
    // Verify affected cells
    let affected = extraction.affected_cells();
    assert_eq!(affected.len(), 1);
    assert!(affected.contains(&cell_a));
}

#[test]
fn test_transaction_manager_atomic_methods() {
    let mut tm = TransactionManager::new();
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    let cell_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Test atomic transfer proposal
    let result = tm.propose_transfer(
        "RadiativeTransfer".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        75.0,
        0.0,
        "Heat transfer".to_string(),
    );
    assert!(result.is_ok());
    
    // Test atomic injection proposal
    let result = tm.propose_injection(
        "CoreRadiance".to_string(),
        cell_a.clone(),
        25.0,
        0.0,
        "Energy injection".to_string(),
    );
    assert!(result.is_ok());
    
    // Test atomic extraction proposal
    let result = tm.propose_extraction(
        "SpaceRadiation".to_string(),
        cell_a.clone(),
        10.0,
        0.0,
        "Energy extraction".to_string(),
    );
    assert!(result.is_ok());
    
    let (pending, _) = tm.get_transaction_stats();
    assert_eq!(pending, 3);
}

#[test]
fn test_atomic_operation_types() {
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    let cell_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Test transfer operation
    let transfer = AtomicTransaction::transfer(
        "Test".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        100.0,
        5.0,
        "Transfer test".to_string(),
    ).unwrap();
    
    match &transfer.operation {
        AtomicOperation::Transfer { from_cell, to_cell, energy_joules, mass_kg } => {
            assert_eq!(*from_cell, cell_a);
            assert_eq!(*to_cell, cell_b);
            assert_eq!(*energy_joules, 100.0);
            assert_eq!(*mass_kg, 5.0);
        }
        _ => panic!("Expected Transfer operation"),
    }
    
    // Test injection operation
    let injection = AtomicTransaction::inject(
        "Test".to_string(),
        cell_a.clone(),
        50.0,
        2.0,
        "Injection test".to_string(),
    ).unwrap();
    
    match &injection.operation {
        AtomicOperation::Inject { into_cell, energy_joules, mass_kg } => {
            assert_eq!(*into_cell, cell_a);
            assert_eq!(*energy_joules, 50.0);
            assert_eq!(*mass_kg, 2.0);
        }
        _ => panic!("Expected Inject operation"),
    }
    
    // Test extraction operation
    let extraction = AtomicTransaction::extract(
        "Test".to_string(),
        cell_a.clone(),
        25.0,
        1.0,
        "Extraction test".to_string(),
    ).unwrap();
    
    match &extraction.operation {
        AtomicOperation::Extract { from_cell, energy_joules, mass_kg } => {
            assert_eq!(*from_cell, cell_a);
            assert_eq!(*energy_joules, 25.0);
            assert_eq!(*mass_kg, 1.0);
        }
        _ => panic!("Expected Extract operation"),
    }
}

#[test]
fn test_no_negative_values_allowed() {
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    let cell_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // All operations should reject negative values
    assert!(AtomicTransaction::transfer("Test".to_string(), cell_a.clone(), cell_b.clone(), -1.0, 0.0, "".to_string()).is_err());
    assert!(AtomicTransaction::transfer("Test".to_string(), cell_a.clone(), cell_b.clone(), 0.0, -1.0, "".to_string()).is_err());
    assert!(AtomicTransaction::inject("Test".to_string(), cell_a.clone(), -1.0, 0.0, "".to_string()).is_err());
    assert!(AtomicTransaction::inject("Test".to_string(), cell_a.clone(), 0.0, -1.0, "".to_string()).is_err());
    assert!(AtomicTransaction::extract("Test".to_string(), cell_a.clone(), -1.0, 0.0, "".to_string()).is_err());
    assert!(AtomicTransaction::extract("Test".to_string(), cell_a.clone(), 0.0, -1.0, "".to_string()).is_err());
}
