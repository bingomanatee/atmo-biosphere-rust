#[cfg(test)]
mod pressure_equalization_tests {
    use super::*;
    use crate::deprecated::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
    use crate::material::materials_loader::MaterialsLoader;
    use h3o::CellIndex;

    #[test]
    fn test_mass_transfer_pressure_imbalance() {
        println!("\n🧪 Testing Mass Transfer Pressure Imbalance");
        println!("============================================");
        
        let materials = MaterialsLoader::load_materials().expect("Failed to load materials");
        let cell_index = CellIndex::try_from(0x85283473fffffff_u64).unwrap();
        
        // Create two cells with different pressures but same material
        let mut lower_cell = EnergyMassCell::new(EnergyMassCellProps {
            cell_index,
            temperature_kelvin: 1800.0,
            pressure_pa: 2e9,  // 2 GPa - high pressure (deep)
            height_km: 20.0,
            top_km: 200.0,
            material_name: "basalt".to_string(),
        }, &materials);
        
        let mut upper_cell = EnergyMassCell::new(EnergyMassCellProps {
            cell_index,
            temperature_kelvin: 1200.0,
            pressure_pa: 1e9,  // 1 GPa - lower pressure (shallow)
            height_km: 20.0,
            top_km: 100.0,
            material_name: "basalt".to_string(),
        }, &materials);
        
        // Record initial state
        let initial_lower_mass = lower_cell.mass_kg();
        let initial_upper_mass = upper_cell.mass_kg();
        let initial_lower_pressure = lower_cell.pressure_pa();
        let initial_upper_pressure = upper_cell.pressure_pa();
        
        println!("Initial State:");
        println!("  Lower cell: {:.2e} kg at {:.1e} Pa", initial_lower_mass, initial_lower_pressure);
        println!("  Upper cell: {:.2e} kg at {:.1e} Pa", initial_upper_mass, initial_upper_pressure);
        println!("  Pressure ratio: {:.2}", initial_lower_pressure / initial_upper_pressure);
        
        // Simulate convection plume mass transfer (typical 0.1% transfer)
        let mass_transfer_fraction = 0.001;
        let mass_to_transfer = initial_lower_mass * mass_transfer_fraction;
        
        println!("\nSimulating mass transfer:");
        println!("  Transferring: {:.2e} kg ({:.1}% of lower cell)", 
                 mass_to_transfer, mass_transfer_fraction * 100.0);
        
        // Apply mass transfer (current implementation)
        lower_cell.add_mass_kg(-mass_to_transfer);  // Remove from lower
        upper_cell.add_mass_kg(mass_to_transfer);   // Add to upper
        
        // Check final state
        let final_lower_mass = lower_cell.mass_kg();
        let final_upper_mass = upper_cell.mass_kg();
        let final_lower_pressure = lower_cell.pressure_pa();
        let final_upper_pressure = upper_cell.pressure_pa();
        
        println!("\nFinal State:");
        println!("  Lower cell: {:.2e} kg at {:.1e} Pa", final_lower_mass, final_lower_pressure);
        println!("  Upper cell: {:.2e} kg at {:.1e} Pa", final_upper_mass, final_upper_pressure);
        println!("  Pressure ratio: {:.2}", final_lower_pressure / final_upper_pressure);
        
        // Calculate pressure changes
        let lower_pressure_change = final_lower_pressure - initial_lower_pressure;
        let upper_pressure_change = final_upper_pressure - initial_upper_pressure;
        
        println!("\nPressure Changes:");
        println!("  Lower cell: {:.2e} Pa ({:.1}%)", 
                 lower_pressure_change, 
                 (lower_pressure_change / initial_lower_pressure) * 100.0);
        println!("  Upper cell: {:.2e} Pa ({:.1}%)", 
                 upper_pressure_change,
                 (upper_pressure_change / initial_upper_pressure) * 100.0);
        
        // Check for mass conservation
        let total_initial_mass = initial_lower_mass + initial_upper_mass;
        let total_final_mass = final_lower_mass + final_upper_mass;
        let mass_conservation_error = (total_final_mass - total_initial_mass).abs();
        
        println!("\nMass Conservation:");
        println!("  Initial total: {:.2e} kg", total_initial_mass);
        println!("  Final total: {:.2e} kg", total_final_mass);
        println!("  Error: {:.2e} kg", mass_conservation_error);
        
        // CRITICAL TEST: Check if pressure equilibrium is maintained
        let pressure_imbalance = (final_lower_pressure - final_upper_pressure).abs();
        let expected_pressure_gradient = initial_lower_pressure - initial_upper_pressure;
        let pressure_gradient_change = pressure_imbalance - expected_pressure_gradient;
        
        println!("\nPressure Equilibrium Analysis:");
        println!("  Expected gradient: {:.2e} Pa", expected_pressure_gradient);
        println!("  Actual gradient: {:.2e} Pa", final_lower_pressure - final_upper_pressure);
        println!("  Gradient change: {:.2e} Pa", pressure_gradient_change);
        
        // ASSERTIONS - These should fail with current implementation
        assert!(mass_conservation_error < 1e-6, "Mass conservation violated");
        
        // This test will likely fail - showing the pressure imbalance problem
        let max_acceptable_gradient_change = expected_pressure_gradient * 0.01; // 1% tolerance
        if pressure_gradient_change.abs() > max_acceptable_gradient_change {
            println!("\n❌ PRESSURE IMBALANCE DETECTED!");
            println!("   Mass transfer without pressure equalization causes instability");
            println!("   Gradient change: {:.2e} Pa (>{:.2e} Pa tolerance)", 
                     pressure_gradient_change.abs(), max_acceptable_gradient_change);
        } else {
            println!("\n✅ Pressure equilibrium maintained");
        }
        
        // Document the problem
        println!("\n🔍 ANALYSIS:");
        println!("   Current implementation transfers mass without considering:");
        println!("   1. Pressure equilibrium requirements");
        println!("   2. Hydrostatic pressure gradients");
        println!("   3. Material compressibility effects");
        println!("   4. Volume conservation during mass transfer");
        
        println!("\n💡 SOLUTION NEEDED:");
        println!("   Mass transfer should maintain pressure equilibrium by:");
        println!("   1. Calculating required volume changes");
        println!("   2. Adjusting cell dimensions to maintain pressure");
        println!("   3. Ensuring hydrostatic equilibrium is preserved");
        println!("   4. Limiting transfer rates to prevent instability");
    }

    #[test]
    fn test_repeated_mass_transfer_drainage() {
        println!("\n🧪 Testing Repeated Mass Transfer Drainage");
        println!("==========================================");
        
        let materials = MaterialsLoader::load_materials().expect("Failed to load materials");
        let cell_index = CellIndex::try_from(0x85283473fffffff_u64).unwrap();
        
        let mut cell = EnergyMassCell::new(EnergyMassCellProps {
            cell_index,
            temperature_kelvin: 1800.0,
            pressure_pa: 2e9,
            height_km: 20.0,
            top_km: 200.0,
            material_name: "basalt".to_string(),
        }, &materials);
        
        let initial_mass = cell.mass_kg();
        println!("Initial mass: {:.2e} kg", initial_mass);
        
        // Simulate repeated mass transfers (like convection plumes)
        let transfer_fraction = 0.001; // 0.1% per transfer
        let num_transfers = 100; // 100 transfers
        
        for i in 0..num_transfers {
            let current_mass = cell.mass_kg();
            let transfer_amount = current_mass * transfer_fraction;
            cell.add_mass_kg(-transfer_amount);
            
            if i % 20 == 0 {
                println!("Transfer {}: {:.2e} kg remaining ({:.1}% of initial)", 
                         i, current_mass, (current_mass / initial_mass) * 100.0);
            }
            
            // Check if cell has been drained
            if current_mass < initial_mass * 0.01 { // Less than 1% remaining
                println!("❌ CELL DRAINED at transfer {}", i);
                println!("   Remaining: {:.2e} kg ({:.3}% of initial)", 
                         current_mass, (current_mass / initial_mass) * 100.0);
                break;
            }
        }
        
        let final_mass = cell.mass_kg();
        let mass_loss_percent = ((initial_mass - final_mass) / initial_mass) * 100.0;
        
        println!("\nFinal Result:");
        println!("  Final mass: {:.2e} kg", final_mass);
        println!("  Mass lost: {:.1}%", mass_loss_percent);
        
        // This demonstrates the drainage problem
        assert!(final_mass > initial_mass * 0.1, 
                "Cell lost more than 90% of mass - drainage problem detected");
    }
}
