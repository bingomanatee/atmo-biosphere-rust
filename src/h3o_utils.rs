
use h3o::Resolution;

fn main() {
    // Iterate resolutions 0..=15
    for res in Resolution::range(Resolution::Zero, Resolution::Fifteen) {
        let res_num: u8 = res.into();
        let edge_km = res.edge_length_km();       // average edge length in km
        let area_km2 = res.area_km2();            // average hexagon area in km²
        let count = res.cell_count();             // total number of cells at this resolution

        println!(
            "Res {:2}: edge ≈ {:8.6} km, area ≈ {:10.6} km², count = {}",
            res_num, edge_km, area_km2, count
        );
    }
}