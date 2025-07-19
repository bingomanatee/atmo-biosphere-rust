use atmo_biosphere_rust::material::{MaterialsLoader, MaterialPhases, get_phase_properties_by_name};

fn main() -> Result<(), String> {
    println!("=== Simple Materials Test ===\n");

    // Test 1: Load materials and show available ones
    println!("1. Available materials:");
    let materials = MaterialsLoader::get_material_names()?;
    for material in &materials {
        println!("   - {}", material);
    }
    println!();

    // Test 2: Get phase properties using enum
    println!("2. Getting basalt solid properties using MaterialPhases::Solid:");
    let basalt_solid = MaterialsLoader::get_phase_properties("basalt", MaterialPhases::Solid)?;
    println!("   Density: {} kg/m³", basalt_solid.density_kg_m3);
    println!("   Thermal conductivity: {} W/(m·K)", basalt_solid.thermal_conductivity_w_m_k);
    println!();

    // Test 3: Get phase properties using string (converted to enum internally)
    println!("3. Getting water liquid properties using string 'liquid':");
    let water_liquid = get_phase_properties_by_name("water", "liquid")?;
    println!("   Density: {} kg/m³", water_liquid.density_kg_m3);
    println!("   Specific heat capacity: {} J/(kg·K)", water_liquid.specific_heat_capacity_j_per_kg_k);
    println!();

    // Test 4: Show enum conversion works with different cases
    println!("4. Testing case-insensitive enum conversion:");
    let test_cases = ["solid", "LIQUID", "Gas"];
    for case in &test_cases {
        match MaterialPhases::from_str(case) {
            Some(phase) => println!("   '{}' -> {:?}", case, phase),
            None => println!("   '{}' -> Invalid", case),
        }
    }
    println!();

    // Test 5: Get available phases for a material
    println!("5. Available phases for steel:");
    let steel_phases = MaterialsLoader::get_available_phases("steel")?;
    for phase in steel_phases {
        println!("   - {:?} ('{}')", phase, phase.as_str());
    }
    println!();

    println!("=== All tests completed successfully! ===");
    Ok(())
}
