use atmo_biosphere_rust::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::CellIndex;

fn main() {
    println!("🧪 Testing mass conservation between two cells");
    
    // Create two test cells
    let cell_index = CellIndex::try_from(0x85283473fffffff_u64).unwrap();
    
    let mut source_cell = EnergyMassCell::new(EnergyMassCellProps {
        cell_index,
        temperature_kelvin: 2000.0,  // Hot source cell
        pressure_pa: 1e9,            // 1 GPa pressure
        height_km: 10.0,
        top_km: 200.0,               // Deep layer
        material_name: "basalt".to_string(),
        planet_radius_km: 6371.0,
    });

    let mut target_cell = EnergyMassCell::new(EnergyMassCellProps {
        cell_index,
        temperature_kelvin: 1000.0,  // Cooler target cell
        pressure_pa: 5e8,            // 0.5 GPa pressure
        height_km: 10.0,
        top_km: 100.0,               // Shallower layer
        material_name: "basalt".to_string(),
        planet_radius_km: 6371.0,
    });

    // Record initial masses
    let initial_source_mass = source_cell.mass_kg();
    let initial_target_mass = target_cell.mass_kg();
    let initial_total_mass = initial_source_mass + initial_target_mass;

    println!("📊 Initial state:");
    println!("   Source cell: {:.2e} kg, {:.0}K", initial_source_mass, source_cell.temperature_kelvin());
    println!("   Target cell: {:.2e} kg, {:.0}K", initial_target_mass, target_cell.temperature_kelvin());
    println!("   Total mass: {:.2e} kg", initial_total_mass);

    // Simulate plume mass transfer (0.1% of source mass)
    let mass_transfer_fraction = 0.001;
    let mass_to_transport = initial_source_mass * mass_transfer_fraction;

    println!("\n🔄 Simulating plume transport:");
    println!("   Mass to transport: {:.2e} kg ({:.1}% of source)", 
        mass_to_transport, mass_transfer_fraction * 100.0);

    // Apply double-entry accounting
    println!("   Debit source: -{:.2e} kg", mass_to_transport);
    source_cell.add_mass_kg(-mass_to_transport);
    
    println!("   Credit target: +{:.2e} kg", mass_to_transport);
    target_cell.add_mass_kg(mass_to_transport);

    // Record final masses
    let final_source_mass = source_cell.mass_kg();
    let final_target_mass = target_cell.mass_kg();
    let final_total_mass = final_source_mass + final_target_mass;

    println!("\n📊 Final state:");
    println!("   Source cell: {:.2e} kg, {:.0}K", final_source_mass, source_cell.temperature_kelvin());
    println!("   Target cell: {:.2e} kg, {:.0}K", final_target_mass, target_cell.temperature_kelvin());
    println!("   Total mass: {:.2e} kg", final_total_mass);

    // Check mass conservation
    let mass_difference = final_total_mass - initial_total_mass;
    let mass_conservation_error = mass_difference.abs() / initial_total_mass;

    println!("\n🔍 Mass conservation analysis:");
    println!("   Initial total: {:.2e} kg", initial_total_mass);
    println!("   Final total:   {:.2e} kg", final_total_mass);
    println!("   Difference:    {:.2e} kg", mass_difference);
    println!("   Error:         {:.2e}% ({:.1e} relative)", 
        mass_conservation_error * 100.0, mass_conservation_error);

    // Check individual cell changes
    let source_change = final_source_mass - initial_source_mass;
    let target_change = final_target_mass - initial_target_mass;
    
    println!("\n🔍 Individual cell changes:");
    println!("   Source change: {:.2e} kg (expected: -{:.2e})", source_change, mass_to_transport);
    println!("   Target change: {:.2e} kg (expected: +{:.2e})", target_change, mass_to_transport);
    
    // Check if conservation is maintained
    if mass_conservation_error < 1e-10 {
        println!("\n✅ Mass conservation test PASSED!");
    } else {
        println!("\n❌ Mass conservation test FAILED!");
        println!("   Error: {:.2e}% (threshold: 1e-8%)", mass_conservation_error * 100.0);
    }

    // Check if individual changes match expectations
    let source_error = (source_change + mass_to_transport).abs() / mass_to_transport;
    let target_error = (target_change - mass_to_transport).abs() / mass_to_transport;
    
    if source_error < 1e-10 && target_error < 1e-10 {
        println!("✅ Individual cell changes are correct!");
    } else {
        println!("❌ Individual cell changes are incorrect!");
        println!("   Source error: {:.2e}%", source_error * 100.0);
        println!("   Target error: {:.2e}%", target_error * 100.0);
    }

    // Test edge case: what happens with very small mass transfers?
    println!("\n🧪 Testing edge case: very small mass transfer (1e-6%)");
    let tiny_fraction = 1e-8;
    let tiny_mass = initial_source_mass * tiny_fraction;
    
    let before_source = source_cell.mass_kg();
    let before_target = target_cell.mass_kg();
    
    source_cell.add_mass_kg(-tiny_mass);
    target_cell.add_mass_kg(tiny_mass);
    
    let after_source = source_cell.mass_kg();
    let after_target = target_cell.mass_kg();
    
    let tiny_source_change = after_source - before_source;
    let tiny_target_change = after_target - before_target;
    let tiny_total_change = (after_source + after_target) - (before_source + before_target);
    
    println!("   Tiny mass transfer: {:.2e} kg", tiny_mass);
    println!("   Source change: {:.2e} kg", tiny_source_change);
    println!("   Target change: {:.2e} kg", tiny_target_change);
    println!("   Total change: {:.2e} kg", tiny_total_change);
    
    if tiny_total_change.abs() < 1e-12 {
        println!("✅ Tiny mass transfer conserved!");
    } else {
        println!("❌ Tiny mass transfer NOT conserved!");
    }
}
