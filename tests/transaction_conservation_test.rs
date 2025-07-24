use atmo_biosphere_rust::transaction_manager::{Transaction, CellLocation, TransactionManager};
use atmo_biosphere_rust::sim_immut::energy_mass_cell_immut::{EnergyMassCellImmut, EnergyMassCellImmutProps};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use atmo_biosphere_rust::material::MaterialPhases;
use std::collections::HashMap;
use h3o::CellIndex;

fn create_test_cell(energy: f64, mass: f64) -> EnergyMassCellImmut {
    let props = EnergyMassCellImmutProps {
        material_name: "basalt".to_string(),
        cell_index: CellIndex::try_from(0x85283473fffffff_u64).unwrap(),
        top_km: 0.0,
        height_km: 10.0,
        temperature_kelvin: 1000.0,
        pressure_pa: 1e5,
        planet_radius_km: 6371.0,
    };

    let mut cell = EnergyMassCellImmut::new(props);
    // Set the energy and mass to desired test values
    cell.set_energy_joules(energy);
    cell.set_mass_kg(mass);
    cell
}

fn create_test_location(layer_set: usize, h3_index: u64, depth: usize) -> CellLocation {
    CellLocation {
        layer_set_index: layer_set,
        h3_cell_index: CellIndex::try_from(h3_index).unwrap(),
        depth_index: depth,
    }
}

#[test]
fn test_energy_conservation_simple_transfer() {
    // Create test cells
    let cell_a = create_test_cell(1000.0, 100.0);
    let cell_b = create_test_cell(500.0, 50.0);
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    let loc_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Initial total energy and mass
    let initial_energy = cell_a.energy_joules() + cell_b.energy_joules();
    let initial_mass = cell_a.mass_kg() + cell_b.mass_kg();
    
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
    
    // Apply transactions manually (simulating transaction manager)
    let mut cells = HashMap::new();
    cells.insert(loc_a.clone(), cell_a);
    cells.insert(loc_b.clone(), cell_b);
    
    // Apply energy deltas
    let mut updated_cells = HashMap::new();
    for (location, cell) in cells {
        let mut new_energy = cell.energy_joules();
        let mut new_mass = cell.mass_kg();
        
        // Apply all transactions affecting this cell
        for transaction in &transactions {
            if transaction.source_cell == location {
                new_energy += transaction.energy_delta_joules;
                new_mass += transaction.mass_delta_kg;
            }
        }
        
        let mut updated_cell = cell.clone();
        updated_cell.set_energy_joules(new_energy);
        updated_cell.set_mass_kg(new_mass);
        
        updated_cells.insert(location, updated_cell);
    }
    
    // Calculate final total energy and mass
    let final_energy: f64 = updated_cells.values()
        .map(|cell| cell.energy_joules())
        .sum();
    let final_mass: f64 = updated_cells.values()
        .map(|cell| cell.mass_kg())
        .sum();
    
    // Verify conservation
    assert!((final_energy - initial_energy).abs() < 1e-10, 
            "Energy not conserved: initial={}, final={}, diff={}", 
            initial_energy, final_energy, final_energy - initial_energy);
    assert!((final_mass - initial_mass).abs() < 1e-10,
            "Mass not conserved: initial={}, final={}, diff={}", 
            initial_mass, final_mass, final_mass - initial_mass);
    
    // Verify specific cell changes
    let updated_a = &updated_cells[&loc_a];
    let updated_b = &updated_cells[&loc_b];
    
    assert!((updated_a.energy_joules() - 900.0).abs() < 1e-10, 
            "Cell A energy incorrect: expected 900, got {}", updated_a.energy_joules());
    assert!((updated_b.energy_joules() - 600.0).abs() < 1e-10,
            "Cell B energy incorrect: expected 600, got {}", updated_b.energy_joules());
}

#[test]
fn test_energy_loss_to_space() {
    // Create test cell
    let cell_a = create_test_cell(1000.0, 100.0);
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    
    // Initial totals
    let initial_energy = cell_a.energy_joules();
    let initial_mass = cell_a.mass_kg();
    
    // Create transaction: lose 100J to space (no target cell)
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
    
    // Apply transactions
    let mut cells = HashMap::new();
    cells.insert(loc_a.clone(), cell_a);
    
    // Apply energy deltas
    let mut updated_cells = HashMap::new();
    for (location, cell) in cells {
        let mut new_energy = cell.energy_joules();
        let mut new_mass = cell.mass_kg();
        
        // Apply all transactions affecting this cell
        for transaction in &transactions {
            if transaction.source_cell == location {
                new_energy += transaction.energy_delta_joules;
                new_mass += transaction.mass_delta_kg;
            }
        }
        
        let mut updated_cell = cell.clone();
        updated_cell.set_energy_joules(new_energy);
        updated_cell.set_mass_kg(new_mass);
        
        updated_cells.insert(location, updated_cell);
    }
    
    // Calculate final totals
    let final_energy: f64 = updated_cells.values()
        .map(|cell| cell.energy_joules())
        .sum();
    let final_mass: f64 = updated_cells.values()
        .map(|cell| cell.mass_kg())
        .sum();
    
    // Verify energy loss and mass conservation
    assert!((final_energy - (initial_energy - energy_loss)).abs() < 1e-10, 
            "Energy loss to space incorrect: expected {}, got {}", 
            initial_energy - energy_loss, final_energy);
    assert!((final_mass - initial_mass).abs() < 1e-10,
            "Mass should be conserved in energy loss to space");
    
    // Verify specific cell change
    let updated_a = &updated_cells[&loc_a];
    assert!((updated_a.energy_joules() - 900.0).abs() < 1e-10, 
            "Cell A energy incorrect after space loss: expected 900, got {}", 
            updated_a.energy_joules());
}

#[test]
fn test_mass_conservation_transfer() {
    // Create test cells
    let cell_a = create_test_cell(1000.0, 100.0);
    let cell_b = create_test_cell(500.0, 50.0);
    let loc_a = create_test_location(0, 0x85283473fffffff, 0);
    let loc_b = create_test_location(0, 0x85283477fffffff, 1);
    
    // Initial totals
    let initial_energy = cell_a.energy_joules() + cell_b.energy_joules();
    let initial_mass = cell_a.mass_kg() + cell_b.mass_kg();
    
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
    
    // Apply transactions manually
    let mut cells = HashMap::new();
    cells.insert(loc_a.clone(), cell_a);
    cells.insert(loc_b.clone(), cell_b);
    
    // Apply mass deltas
    let mut updated_cells = HashMap::new();
    for (location, cell) in cells {
        let mut new_energy = cell.energy_joules();
        let mut new_mass = cell.mass_kg();
        
        // Apply all transactions affecting this cell
        for transaction in &transactions {
            if transaction.source_cell == location {
                new_energy += transaction.energy_delta_joules;
                new_mass += transaction.mass_delta_kg;
            }
        }
        
        let mut updated_cell = cell.clone();
        updated_cell.set_energy_joules(new_energy);
        updated_cell.set_mass_kg(new_mass);
        
        updated_cells.insert(location, updated_cell);
    }
    
    // Calculate final totals
    let final_energy: f64 = updated_cells.values()
        .map(|cell| cell.energy_joules())
        .sum();
    let final_mass: f64 = updated_cells.values()
        .map(|cell| cell.mass_kg())
        .sum();
    
    // Verify conservation
    assert!((final_energy - initial_energy).abs() < 1e-10, 
            "Energy not conserved in mass transfer");
    assert!((final_mass - initial_mass).abs() < 1e-10,
            "Mass not conserved: initial={}, final={}, diff={}", 
            initial_mass, final_mass, final_mass - initial_mass);
    
    // Verify specific cell changes
    let updated_a = &updated_cells[&loc_a];
    let updated_b = &updated_cells[&loc_b];
    
    assert!((updated_a.mass_kg() - 90.0).abs() < 1e-10, 
            "Cell A mass incorrect: expected 90, got {}", updated_a.mass_kg());
    assert!((updated_b.mass_kg() - 60.0).abs() < 1e-10,
            "Cell B mass incorrect: expected 60, got {}", updated_b.mass_kg());
}
