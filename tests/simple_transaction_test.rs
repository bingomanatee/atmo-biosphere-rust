use atmo_biosphere_rust::transaction_manager::{Transaction, CellLocation};
use h3o::CellIndex;

fn create_test_location(layer_set: usize, h3_index: u64, depth: usize) -> CellLocation {
    CellLocation {
        layer_set_index: layer_set,
        h3_cell_index: CellIndex::try_from(h3_index).unwrap(),
        depth_index: depth,
    }
}

#[test]
fn test_transaction_energy_conservation_logic() {
    // Test the transaction logic for energy conservation
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    let loc_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Initial energy values
    let initial_energy_a = 1000.0;
    let initial_energy_b = 500.0;
    let total_initial_energy = initial_energy_a + initial_energy_b;
    
    // Create paired transactions: transfer 100J from A to B
    let transfer_energy = 100.0;
    let transactions = vec![
        Transaction {
            source: "test".to_string(),
            source_cell: loc_a.clone(),
            target_cell: Some(loc_b.clone()),
            energy_delta_joules: -transfer_energy, // Remove from A
            mass_delta_kg: 0.0,
            description: "Transfer energy from A to B".to_string(),
            step_id: 1,
        },
        Transaction {
            source: "test".to_string(),
            source_cell: loc_b.clone(),
            target_cell: Some(loc_a.clone()),
            energy_delta_joules: transfer_energy, // Add to B
            mass_delta_kg: 0.0,
            description: "Transfer energy from A to B".to_string(),
            step_id: 1,
        },
    ];
    
    // Simulate transaction application
    let mut energy_a = initial_energy_a;
    let mut energy_b = initial_energy_b;
    
    for transaction in &transactions {
        if transaction.source_cell == loc_a {
            energy_a += transaction.energy_delta_joules;
        }
        if transaction.source_cell == loc_b {
            energy_b += transaction.energy_delta_joules;
        }
    }
    
    let total_final_energy = energy_a + energy_b;
    
    // Verify conservation
    assert!((total_final_energy - total_initial_energy).abs() < 1e-10, 
            "Energy not conserved: initial={}, final={}, diff={}", 
            total_initial_energy, total_final_energy, total_final_energy - total_initial_energy);
    
    // Verify specific changes
    assert!((energy_a - 900.0).abs() < 1e-10, 
            "Cell A energy incorrect: expected 900, got {}", energy_a);
    assert!((energy_b - 600.0).abs() < 1e-10,
            "Cell B energy incorrect: expected 600, got {}", energy_b);
}

#[test]
fn test_transaction_energy_loss_to_space() {
    // Test energy loss to space (no target cell)
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    
    let initial_energy = 1000.0;
    let energy_loss = 100.0;
    
    let transactions = vec![
        Transaction {
            source: "radiative_cooling".to_string(),
            source_cell: loc_a.clone(),
            target_cell: None, // Energy lost to space
            energy_delta_joules: -energy_loss,
            mass_delta_kg: 0.0,
            description: "Energy lost to space".to_string(),
            step_id: 1,
        },
    ];
    
    // Simulate transaction application
    let mut energy_a = initial_energy;
    
    for transaction in &transactions {
        if transaction.source_cell == loc_a {
            energy_a += transaction.energy_delta_joules;
        }
    }
    
    // Verify energy loss
    assert!((energy_a - 900.0).abs() < 1e-10, 
            "Cell A energy incorrect after space loss: expected 900, got {}", energy_a);
    
    // Total system energy should decrease by the amount lost to space
    let system_energy_loss = initial_energy - energy_a;
    assert!((system_energy_loss - energy_loss).abs() < 1e-10,
            "System energy loss incorrect: expected {}, got {}", energy_loss, system_energy_loss);
}

#[test]
fn test_transaction_mass_conservation_logic() {
    // Test the transaction logic for mass conservation
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    let loc_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Initial mass values
    let initial_mass_a = 100.0;
    let initial_mass_b = 50.0;
    let total_initial_mass = initial_mass_a + initial_mass_b;
    
    // Create paired transactions: transfer 10kg from A to B
    let transfer_mass = 10.0;
    let transactions = vec![
        Transaction {
            source: "test".to_string(),
            source_cell: loc_a.clone(),
            target_cell: Some(loc_b.clone()),
            energy_delta_joules: 0.0,
            mass_delta_kg: -transfer_mass, // Remove from A
            description: "Transfer mass from A to B".to_string(),
            step_id: 1,
        },
        Transaction {
            source: "test".to_string(),
            source_cell: loc_b.clone(),
            target_cell: Some(loc_a.clone()),
            energy_delta_joules: 0.0,
            mass_delta_kg: transfer_mass, // Add to B
            description: "Transfer mass from A to B".to_string(),
            step_id: 1,
        },
    ];
    
    // Simulate transaction application
    let mut mass_a = initial_mass_a;
    let mut mass_b = initial_mass_b;
    
    for transaction in &transactions {
        if transaction.source_cell == loc_a {
            mass_a += transaction.mass_delta_kg;
        }
        if transaction.source_cell == loc_b {
            mass_b += transaction.mass_delta_kg;
        }
    }
    
    let total_final_mass = mass_a + mass_b;
    
    // Verify conservation
    assert!((total_final_mass - total_initial_mass).abs() < 1e-10, 
            "Mass not conserved: initial={}, final={}, diff={}", 
            total_initial_mass, total_final_mass, total_final_mass - total_initial_mass);
    
    // Verify specific changes
    assert!((mass_a - 90.0).abs() < 1e-10, 
            "Cell A mass incorrect: expected 90, got {}", mass_a);
    assert!((mass_b - 60.0).abs() < 1e-10,
            "Cell B mass incorrect: expected 60, got {}", mass_b);
}

#[test]
fn test_radiative_transfer_transaction_pairing() {
    // Test that radiative transfer creates properly paired transactions
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    let loc_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Simulate what radiative transfer should create
    let energy_transfer = 50.0;
    
    // This is what the radiative transfer SHOULD create for proper conservation
    let correct_transactions = vec![
        // Remove energy from source cell
        Transaction {
            source: "RadiativeTransfer".to_string(),
            source_cell: loc_a.clone(),
            target_cell: Some(loc_b.clone()),
            energy_delta_joules: -energy_transfer,
            mass_delta_kg: 0.0,
            description: "Radiative transfer from A to B".to_string(),
            step_id: 1,
        },
        // Add energy to target cell
        Transaction {
            source: "RadiativeTransfer".to_string(),
            source_cell: loc_b.clone(),
            target_cell: Some(loc_a.clone()),
            energy_delta_joules: energy_transfer,
            mass_delta_kg: 0.0,
            description: "Radiative transfer from A to B".to_string(),
            step_id: 1,
        },
    ];
    
    // Verify that the sum of all energy deltas is zero (conservation)
    let total_energy_delta: f64 = correct_transactions.iter()
        .map(|t| t.energy_delta_joules)
        .sum();
    
    assert!((total_energy_delta).abs() < 1e-10,
            "Transaction energy deltas don't sum to zero: total={}", total_energy_delta);
    
    // Verify that the sum of all mass deltas is zero (conservation)
    let total_mass_delta: f64 = correct_transactions.iter()
        .map(|t| t.mass_delta_kg)
        .sum();
    
    assert!((total_mass_delta).abs() < 1e-10,
            "Transaction mass deltas don't sum to zero: total={}", total_mass_delta);
}

#[test]
fn test_space_radiation_transaction() {
    // Test that space radiation properly loses energy from the system
    let loc_surface = create_test_location(0, 0x85283473fffffff, 0);
    
    let energy_lost_to_space = 75.0;
    
    // Space radiation transaction (energy lost to space)
    let space_transaction = Transaction {
        source: "RadiativeTransfer".to_string(),
        source_cell: loc_surface.clone(),
        target_cell: None, // Energy lost to space (2.7K background)
        energy_delta_joules: -energy_lost_to_space,
        mass_delta_kg: 0.0,
        description: "Energy radiated to space".to_string(),
        step_id: 1,
    };
    
    // For space radiation, the energy delta should be negative (energy leaving system)
    assert!(space_transaction.energy_delta_joules < 0.0,
            "Space radiation should have negative energy delta");
    
    // Target cell should be None (energy goes to space, not another cell)
    assert!(space_transaction.target_cell.is_none(),
            "Space radiation should have no target cell");
    
    // Mass should be conserved (no mass lost to space in radiative transfer)
    assert!((space_transaction.mass_delta_kg).abs() < 1e-10,
            "Space radiation should not transfer mass");
}
