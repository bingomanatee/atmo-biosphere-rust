use atmo_biosphere_rust::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::{CellIndex, Resolution};

fn main() {
    println!("=== Heat Transfer Conductivity Demo ===\n");

    // Create two adjacent cells with different temperatures
    let cell_index1 = CellIndex::base_cells().next().unwrap()
        .children(Resolution::Three).next().unwrap();
    let cell_index2 = CellIndex::base_cells().next().unwrap()
        .children(Resolution::Three).nth(1).unwrap();

    let mut hot_cell = EnergyMassCell::new(EnergyMassCellProps {
        cell_index: cell_index1,
        temperature_kelvin: 2000.0, // Hot magma
        pressure_pa: 5e8,           // High pressure (5 GPa)
        height_km: 10.0,
        top_km: 0.0,
        material_name: "basalt".to_string(),
        planet_radius_km: 6371.0,
    });

    let mut cold_cell = EnergyMassCell::new(EnergyMassCellProps {
        cell_index: cell_index2,
        temperature_kelvin: 1000.0, // Cooler rock
        pressure_pa: 1e8,           // Lower pressure (1 GPa)
        height_km: 10.0,
        top_km: 0.0,
        material_name: "basalt".to_string(),
        planet_radius_km: 6371.0,
    });

    println!("Initial conditions:");
    println!("Hot cell: {:.1} K, {:.2e} Pa", hot_cell.temperature_kelvin(), hot_cell.pressure_pa());
    println!("Cold cell: {:.1} K, {:.2e} Pa", cold_cell.temperature_kelvin(), cold_cell.pressure_pa());

    // Calculate individual conductivities
    let hot_conductivity = hot_cell.get_conductivity_w_m_k();
    let cold_conductivity = cold_cell.get_conductivity_w_m_k();
    
    println!("\nConductivities:");
    println!("Hot cell conductivity: {:.3e} W/(m·K)", hot_conductivity);
    println!("Cold cell conductivity: {:.3e} W/(m·K)", cold_conductivity);

    // Calculate conductance coefficient between cells
    let conductance_coeff = hot_cell.calculate_conductance_coefficient(&mut cold_cell, true);
    println!("Conductance coefficient: {:.3e} J/K", conductance_coeff);

    // Simulate heat transfer
    println!("\n=== Heat Transfer Simulation ===");
    
    for step in 1..=5 {
        println!("\nStep {}:", step);
        
        // Calculate energy transfer
        hot_cell.transmit_energy(&mut cold_cell, true);
        
        let hot_pending = hot_cell.pending_energy_delta();
        let cold_pending = cold_cell.pending_energy_delta();
        
        println!("Pending energy transfers:");
        println!("  Hot cell: {:.3e} J", hot_pending);
        println!("  Cold cell: {:.3e} J", cold_pending);
        println!("  Energy conservation: {:.3e} J", hot_pending + cold_pending);
        
        // Commit the energy changes
        hot_cell.commit();
        cold_cell.commit();
        
        println!("After commit:");
        println!("  Hot cell: {:.1} K", hot_cell.temperature_kelvin());
        println!("  Cold cell: {:.1} K", cold_cell.temperature_kelvin());
        println!("  Temperature difference: {:.1} K", 
                 hot_cell.temperature_kelvin() - cold_cell.temperature_kelvin());
    }

    println!("\n=== Pressure Effect on Conductivity ===");
    
    // Demonstrate how pressure affects conductivity
    let mut test_cell = EnergyMassCell::new(EnergyMassCellProps {
        cell_index: cell_index1,
        temperature_kelvin: 1500.0,
        pressure_pa: 1e5, // Start at atmospheric pressure
        height_km: 10.0,
        top_km: 0.0,
        material_name: "basalt".to_string(),
        planet_radius_km: 6371.0,
    });

    let pressures = vec![1e5, 1e6, 1e7, 1e8, 1e9]; // 1 atm to 10 GPa
    
    println!("Pressure vs Conductivity:");
    for pressure in pressures {
        test_cell.set_pressure_pa(pressure);
        let conductivity = test_cell.get_conductivity_w_m_k();
        println!("  {:.1e} Pa: {:.3e} W/(m·K)", pressure, conductivity);
    }

    println!("\n=== Demo Complete ===");
}
