use h3o::Resolution;

fn main() {
    println!("H3 Resolution Information:");
    
    for res in 0..=5 {
        let resolution = Resolution::try_from(res).unwrap();
        let cell_count = resolution.cell_count();
        
        // Approximate cell edge length in km (H3 documentation values)
        let edge_length_km = match res {
            0 => 1107.712,
            1 => 418.676,
            2 => 158.244,
            3 => 59.810,  // ~60km edge length
            4 => 22.606,
            5 => 8.544,
            _ => 0.0,
        };
        
        // Approximate cell area in km²
        let area_km2 = edge_length_km * edge_length_km * 0.866; // Hexagon area approximation
        
        println!("Resolution {}: ~{:.1}km edge, ~{:.0}km² area, {} cells total", 
                 res, edge_length_km, area_km2, cell_count);
    }
    
    println!("\n🎯 Resolution 3 Analysis:");
    println!("   - Edge length: ~60km");
    println!("   - Cell area: ~3,100 km²");
    println!("   - Layer aspect ratios:");
    println!("     * 3km height : 60km width = 1:20 (very flat)");
    println!("     * 10km height : 60km width = 1:6 (flat)");
    println!("     * 20km height : 60km width = 1:3 (reasonable)");
    println!("   - Total depth: 15 cells × (3+10+20)km = 165km");
    println!("   - Much better aspect ratios than deep thin layers!");
}
