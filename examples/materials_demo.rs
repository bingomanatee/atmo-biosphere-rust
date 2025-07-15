use atmo_biosphere_rust::material::{
    MaterialPhases, MaterialsLoader, get_phase_properties_by_name,
};

fn main() -> Result<(), String> {
    println!("=== Materials Loader Demo ===\n");

    // Example 1: Get all available material names
    println!("1. Available materials:");
    let material_names = MaterialsLoader::get_material_names()?;
    for name in &material_names {
        println!("   - {}", name);
    }
    println!();

    // Example 2: Get phase properties using the enum
    println!("2. Getting basalt solid properties using MaterialPhases enum:");
    match MaterialsLoader::get_phase_properties("basalt", MaterialPhases::Solid) {
        Ok(phase) => {
            println!("   Density: {} kg/m³", phase.density_kg_m3);
            println!(
                "   Thermal conductivity: {} W/(m·K)",
                phase.thermal_conductivity_w_m_k
            );
            println!(
                "   Specific heat capacity: {} J/(kg·K)",
                phase.specific_heat_capacity_j_per_kg_k
            );

            println!("   Melting temperature: {} K", phase.melt_temp);

            if let Some(bulk_modulus) = phase.bulk_modulus_pa {
                println!("   Bulk modulus: {} Pa", bulk_modulus);
            }
            // Show scaled fractional values
            if let Some(gas_interference) = phase.gas_interference_factor {
                println!(
                    "   Gas interference factor: {} (scaled by 1000, original: {:.3})",
                    gas_interference,
                    gas_interference as f64 / 1000.0
                );
            }
            if let Some(thermal_expansivity) = phase.thermal_expansivity {
                println!(
                    "   Thermal expansivity: {} (scaled by 1e9, original: {:.2e})",
                    thermal_expansivity,
                    thermal_expansivity as f64 / 1e9
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 3: Get phase properties using string names (convenience function)
    println!("3. Getting water liquid properties using string names:");
    match get_phase_properties_by_name("water", "liquid") {
        Ok(phase) => {
            println!("   Density: {} kg/m³", phase.density_kg_m3);
            println!(
                "   Thermal conductivity: {} W/(m·K)",
                phase.thermal_conductivity_w_m_k
            );
            println!(
                "   Specific heat capacity: {} J/(kg·K)",
                phase.specific_heat_capacity_j_per_kg_k
            );
            if let Some(dynamic_viscosity) = phase.dynamic_viscosity {
                println!(
                    "   Dynamic viscosity: {} Pa·s (original: {:.3e})",
                    dynamic_viscosity, dynamic_viscosity as f64
                );
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 4: Get available phases for a material (as enum variants)
    println!("4. Available phases for granite (as enum variants):");
    match MaterialsLoader::get_available_phases("granite") {
        Ok(phases) => {
            for phase in phases {
                println!("   - {:?} (string: '{}')", phase, phase.as_str());
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 4b: Get available phases as strings
    println!("4b. Available phases for granite (as strings):");
    match MaterialsLoader::get_available_phase_names("granite") {
        Ok(phase_names) => {
            for name in phase_names {
                println!("   - '{}'", name);
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 5: Get emission compounds
    println!("5. Emission compounds for steel:");
    match MaterialsLoader::get_emission_compounds("steel") {
        Ok(Some(compounds)) => {
            for (compound, concentration) in compounds {
                println!("   {}: {:.3}", compound, concentration);
            }
        }
        Ok(None) => println!("   No emission compounds data available"),
        Err(e) => println!("   Error: {}", e),
    }
    println!();

    // Example 6: Demonstrate enum conversion from strings
    println!("6. Demonstrating MaterialPhases enum conversion:");
    let phase_strings = ["solid", "liquid", "gas", "SOLID", "Liquid", "invalid"];
    for phase_str in &phase_strings {
        match MaterialPhases::from_str(phase_str) {
            Some(phase_enum) => {
                println!(
                    "   '{}' -> {:?} -> '{}'",
                    phase_str,
                    phase_enum,
                    phase_enum.as_str()
                );
            }
            None => {
                println!("   '{}' -> Invalid phase name", phase_str);
            }
        }
    }
    println!();

    // Example 7: Compare properties across phases using both enum and string methods
    println!("7. Comparing water properties across phases:");

    // Method 1: Using enum directly
    println!("   Method 1 - Using MaterialPhases enum:");
    for phase_enum in MaterialPhases::all_phases() {
        match MaterialsLoader::get_phase_properties("water", phase_enum) {
            Ok(phase) => {
                println!(
                    "     {:?} - Density: {} kg/m³, Thermal conductivity: {} W/(m·K)",
                    phase_enum, phase.density_kg_m3, phase.thermal_conductivity_w_m_k
                );
            }
            Err(e) => println!("     {:?}: Error - {}", phase_enum, e),
        }
    }

    // Method 2: Using string conversion
    println!("   Method 2 - Using string names:");
    let phases = ["solid", "liquid", "gas"];
    for phase_name in &phases {
        match get_phase_properties_by_name("water", phase_name) {
            Ok(phase) => {
                println!(
                    "     '{}' - Density: {} kg/m³, Thermal conductivity: {} W/(m·K)",
                    phase_name, phase.density_kg_m3, phase.thermal_conductivity_w_m_k
                );
            }
            Err(e) => println!("     '{}': Error - {}", phase_name, e),
        }
    }
    println!();

    // Example 8: Error handling - invalid material
    println!("8. Error handling example - invalid material:");
    match get_phase_properties_by_name("unobtainium", "solid") {
        Ok(_) => println!("   Unexpected success!"),
        Err(e) => println!("   Expected error: {}", e),
    }
    println!();

    // Example 9: Error handling - invalid phase
    println!("9. Error handling example - invalid phase:");
    match get_phase_properties_by_name("basalt", "plasma") {
        Ok(_) => println!("   Unexpected success!"),
        Err(e) => println!("   Expected error: {}", e),
    }
    println!();

    // Example 10: Show all valid phase names
    println!("10. All valid phase names:");
    for phase_name in MaterialPhases::all_phase_names() {
        println!("    - '{}'", phase_name);
    }

    println!("\n=== Demo completed successfully! ===");
    Ok(())
}
