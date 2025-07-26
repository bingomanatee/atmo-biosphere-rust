use atmo_biosphere_rust::transaction_manager::{Transaction, TransactionType, TransactionManager};
use atmo_biosphere_rust::cell_location::CellLocation;
use h3o::CellIndex;

fn create_test_location(layer_set: usize, h3_index: u64, depth: usize) -> CellLocation {
    CellLocation::new(
        layer_set,
        CellIndex::try_from(h3_index).unwrap(),
        depth,
    )
}

fn main() {
    println!("⚛️  Atomic Transaction System Demo");
    println!("==================================");
    println!("Demonstrating safe, atomic transactions that prevent energy creation");

    let mut transaction_manager = TransactionManager::new();
    
    // Create test cell locations
    let cell_a = create_test_location(0, 0x85283473fffffff, 0);
    let cell_b = create_test_location(0, 0x85283477fffffff, 1);
    let cell_c = create_test_location(1, 0x85283473fffffff, 0);

    println!("\n🔄 Creating Atomic Transactions:");
    println!("================================");

    // 1. Paired Transfer (the safe default for energy redistribution)
    println!("\n1️⃣ Paired Transfer (Radiative Transfer):");
    let radiative_transfer = Transaction::paired_transfer(
        "RadiativeTransfer".to_string(),
        cell_a.clone(),
        cell_b.clone(),
        100.0, // 100J from A to B
        0.0,   // No mass transfer
        "Heat transfer from hot cell to cool cell".to_string(),
    );
    
    println!("   Source: {:?}", cell_a.description());
    println!("   Target: {:?}", cell_b.description());
    println!("   Energy: 100J from A to B");
    println!("   Conservation: ✅ Guaranteed (paired transfer)");
    println!("   Overdraft Protection: ✅ Validated before application");
    
    transaction_manager.propose_transaction(radiative_transfer);

    // 2. Absolute Change (for system energy sources like core radiance)
    println!("\n2️⃣ Absolute Change (Core Radiance):");
    let core_radiance = Transaction::absolute_change(
        "CoreRadiance".to_string(),
        cell_c.clone(),
        50.0, // Add 50J to deep cell
        0.0,  // No mass change
        "Core radiance energy injection".to_string(),
    );
    
    println!("   Target: {:?}", cell_c.description());
    println!("   Energy: +50J (system energy input)");
    println!("   Conservation: ⚠️  Creates energy (intended for core radiance)");
    println!("   Use case: Heat sources like core radiance, solar input");
    
    transaction_manager.propose_transaction(core_radiance);

    // 3. Space Radiation (energy loss to space)
    println!("\n3️⃣ Absolute Change (Space Radiation):");
    let space_radiation = Transaction::absolute_change(
        "SpaceRadiation".to_string(),
        cell_a.clone(),
        -25.0, // Remove 25J (radiated to space)
        0.0,   // No mass change
        "Energy radiated to 2.7K space background".to_string(),
    );
    
    println!("   Source: {:?}", cell_a.description());
    println!("   Energy: -25J (system energy loss)");
    println!("   Conservation: ⚠️  Destroys energy (intended for space cooling)");
    println!("   Use case: Radiative cooling to space");
    
    transaction_manager.propose_transaction(space_radiation);

    // Analyze the proposed transactions
    println!("\n📊 Transaction Analysis:");
    println!("========================");
    
    let (pending_count, _) = transaction_manager.get_transaction_stats();
    println!("Pending transactions: {}", pending_count);
    
    // Count transaction types
    let mut paired_transfers = 0;
    let mut absolute_changes = 0;
    let mut total_energy_change = 0.0;
    
    // Note: This is a demo - in practice we'd need to access pending transactions
    // through a public method or implement this analysis in the transaction manager
    
    println!("\nTransaction Types:");
    println!("  🔄 Paired Transfers: {} (energy conserving)", 1);
    println!("  ⚡ Absolute Changes: {} (energy sources/sinks)", 2);
    
    println!("\nNet System Energy Change:");
    println!("  Radiative Transfer: ±0J (redistributes existing energy)");
    println!("  Core Radiance: +50J (adds energy to system)");
    println!("  Space Radiation: -25J (removes energy from system)");
    println!("  Net Change: +25J (system gains energy overall)");

    println!("\n✅ Atomic Transaction Benefits:");
    println!("==============================");
    println!("1. 🔒 No Unsafe Mode: All transactions are atomic and validated");
    println!("2. ⚖️  Guaranteed Conservation: Paired transfers can't create energy");
    println!("3. 🛡️  Overdraft Protection: Source cells validated before transfer");
    println!("4. 🎯 Clear Intent: Transaction type shows conservation properties");
    println!("5. 🔍 Easy Debugging: Single transaction represents complete operation");
    
    println!("\n🌍 Geological Simulation Usage:");
    println!("===============================");
    println!("• Radiative Transfer → Paired transfers (energy neutral)");
    println!("• Thermal Conduction → Paired transfers (energy neutral)");
    println!("• Convection Plumes → Paired transfers (energy + mass neutral)");
    println!("• Core Radiance → Absolute changes (energy input)");
    println!("• Solar Radiation → Absolute changes (energy input)");
    println!("• Space Cooling → Absolute changes (energy output)");
    println!("• Volcanic Outgassing → Absolute changes (mass input)");

    println!("\n🎉 Atomic Transaction System Ready!");
    println!("===================================");
    println!("The transaction system now enforces conservation by default.");
    println!("No unsafe modes, no energy creation bugs, no overdraft issues.");
    println!("All energy transfers are atomic and validated! 🌍⚛️");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paired_transfer_conservation() {
        let cell_a = create_test_location(0, 0x85283473fffffff, 0);
        let cell_b = create_test_location(0, 0x85283477fffffff, 1);
        
        let transaction = Transaction::paired_transfer(
            "Test".to_string(),
            cell_a.clone(),
            cell_b.clone(),
            100.0,
            5.0,
            "Test transfer".to_string(),
        );
        
        assert!(transaction.is_conservative());
        
        let affected_cells = transaction.affected_cells();
        assert_eq!(affected_cells.len(), 2);
        assert!(affected_cells.contains(&cell_a));
        assert!(affected_cells.contains(&cell_b));
    }

    #[test]
    fn test_absolute_change_non_conservative() {
        let cell_a = create_test_location(0, 0x85283473fffffff, 0);
        
        let transaction = Transaction::absolute_change(
            "CoreRadiance".to_string(),
            cell_a.clone(),
            50.0,
            0.0,
            "Energy injection".to_string(),
        );
        
        assert!(!transaction.is_conservative());
        
        let affected_cells = transaction.affected_cells();
        assert_eq!(affected_cells.len(), 1);
        assert!(affected_cells.contains(&cell_a));
    }

    #[test]
    fn test_transaction_manager_convenience_methods() {
        let mut tm = TransactionManager::new();
        let cell_a = create_test_location(0, 0x85283473fffffff, 0);
        let cell_b = create_test_location(0, 0x85283477fffffff, 1);
        
        // Test paired transfer convenience method
        tm.propose_paired_transfer(
            "RadiativeTransfer".to_string(),
            cell_a.clone(),
            cell_b.clone(),
            75.0,
            0.0,
            "Heat transfer".to_string(),
        );
        
        // Test absolute change convenience method
        tm.propose_absolute_change(
            "CoreRadiance".to_string(),
            cell_a.clone(),
            25.0,
            0.0,
            "Energy injection".to_string(),
        );
        
        let (pending, _) = tm.get_transaction_stats();
        assert_eq!(pending, 2);
    }
}
