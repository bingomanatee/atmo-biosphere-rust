use crate::binary_pair::{BinaryPair, BinaryPairId, BinaryPairType};
use crate::cell_location::CellLocation;
use crate::collections::{Collection, CollectionsManager};
use crate::simulation::GeologicalCellData;
use crate::utils::h3_utils::H3Utils;
use h3o::CellIndex;
use std::collections::HashMap;

/// Builder for creating binary pairs between geological cells
/// Based on the deprecated binary pairing system for efficient geological operations
pub struct BinaryPairBuilder {
    /// H3 utilities for neighbor calculations
    h3_utils: H3Utils,
    /// Generated pairs (stored temporarily during building)
    generated_pairs: Vec<BinaryPair>,
}

impl BinaryPairBuilder {
    pub fn new() -> Self {
        Self {
            h3_utils: H3Utils::new(),
            generated_pairs: Vec::new(),
        }
    }
    
    /// Build all binary pairs for the simulation and add them to collections manager
    pub fn build_all_pairs(&mut self, coll_mgr: &mut CollectionsManager) -> Result<usize, String> {
        println!("🔗 Building binary pairs for geological cells...");
        
        // Get geological cells and clone them to avoid borrowing issues
        let geological_cells = coll_mgr
            .get::<CellLocation, GeologicalCellData>("geological_cells")
            .ok_or("geological_cells collection not found")?
            .clone();

        // Create binary pairs collection
        let binary_pairs_collection = crate::collections::Collection::<BinaryPairId, BinaryPair>::new();
        coll_mgr.add_collection("binary_pairs", binary_pairs_collection);

        // Clear any existing pairs
        self.generated_pairs.clear();

        // Build vertical pairs (above/below in same column)
        let vertical_pairs = self.build_vertical_pairs(&geological_cells)?;

        // Build horizontal pairs (H3 neighbors at same depth)
        let horizontal_pairs = self.build_horizontal_pairs(&geological_cells)?;

        let total_pairs = vertical_pairs + horizontal_pairs;

        // Add pairs to collection
        self.add_pairs_to_collection(coll_mgr)?;
        
        println!("✅ Built {} binary pairs total", total_pairs);
        println!("   - {} vertical pairs (above/below)", vertical_pairs);
        println!("   - {} horizontal pairs (H3 neighbors)", horizontal_pairs);
        
        Ok(total_pairs)
    }
    
    /// Build vertical pairs (cells above and below each other in same column)
    fn build_vertical_pairs(&mut self, geological_cells: &Collection<CellLocation, GeologicalCellData>) -> Result<usize, String> {
        let mut pairs_created = 0;
        let mut column_map: HashMap<(usize, CellIndex), Vec<CellLocation>> = HashMap::new();
        
        // Group cells by layer and H3 cell (column)
        for entry in geological_cells.iter() {
            let location = entry.key();
            let key = (location.layer_set_index(), location.h3_cell_index());
            column_map.entry(key).or_insert_with(Vec::new).push(*location);
        }
        
        // Create vertical pairs within each column
        for cells_in_column in column_map.values_mut() {
            // Sort by depth index
            cells_in_column.sort_by_key(|loc| loc.depth_index());
            
            // Create pairs between adjacent depth levels
            for i in 0..cells_in_column.len().saturating_sub(1) {
                let upper_cell = cells_in_column[i];
                let lower_cell = cells_in_column[i + 1];
                
                // Calculate distance and area for vertical pair
                let height_km = self.calculate_vertical_distance(&upper_cell, &lower_cell);
                let area_km2 = self.calculate_h3_area_km2(upper_cell.h3_cell_index());
                
                let pair = BinaryPair::vertical(upper_cell, lower_cell, height_km, area_km2);
                self.generated_pairs.push(pair);
                pairs_created += 1;
            }
        }
        
        Ok(pairs_created)
    }
    
    /// Build horizontal pairs (H3 neighbor cells at same depth)
    fn build_horizontal_pairs(&mut self, geological_cells: &Collection<CellLocation, GeologicalCellData>) -> Result<usize, String> {
        let mut pairs_created = 0;
        let mut depth_map: HashMap<(usize, usize), Vec<CellLocation>> = HashMap::new();
        
        // Group cells by layer and depth
        for entry in geological_cells.iter() {
            let location = entry.key();
            let key = (location.layer_set_index(), location.depth_index());
            depth_map.entry(key).or_insert_with(Vec::new).push(*location);
        }
        
        // Create horizontal pairs within each depth level
        for cells_at_depth in depth_map.values() {
            for i in 0..cells_at_depth.len() {
                let cell_a = cells_at_depth[i];
                
                // Find H3 neighbors for this cell
                let neighbors = H3Utils::get_neighbors(cell_a.h3_cell_index());
                
                for neighbor_h3 in neighbors {
                    // Look for a cell at the same depth with this H3 index
                    for j in (i + 1)..cells_at_depth.len() {
                        let cell_b = cells_at_depth[j];
                        
                        if cell_b.h3_cell_index() == neighbor_h3 {
                            // Found a neighbor pair
                            let distance_km = H3Utils::distance_km(
                                cell_a.h3_cell_index(),
                                cell_b.h3_cell_index(),
                            );
                            let contact_area_km2 = self.calculate_contact_area_km2(&cell_a, &cell_b);
                            
                            let pair = BinaryPair::horizontal(cell_a, cell_b, distance_km, contact_area_km2);
                            self.generated_pairs.push(pair);
                            pairs_created += 1;
                        }
                    }
                }
            }
        }
        
        Ok(pairs_created)
    }
    
    /// Add generated pairs to the collections manager
    fn add_pairs_to_collection(&self, coll_mgr: &mut CollectionsManager) -> Result<(), String> {
        let binary_pairs = coll_mgr
            .get_mut::<BinaryPairId, BinaryPair>("binary_pairs")
            .ok_or("binary_pairs collection not found")?;

        for pair in &self.generated_pairs {
            let pair_id = BinaryPairId::new(pair);
            binary_pairs.insert(pair_id, pair.clone());
        }

        Ok(())
    }
    
    /// Calculate vertical distance between two cells in same column
    fn calculate_vertical_distance(&self, upper_cell: &CellLocation, lower_cell: &CellLocation) -> f64 {
        // For now, use a simple calculation based on depth difference
        // In reality, this would use the layer configuration
        let depth_diff = (lower_cell.depth_index() as f64 - upper_cell.depth_index() as f64).abs();
        
        // Assume average cell height of 10km (this should come from layer config)
        depth_diff * 10.0
    }
    
    /// Calculate H3 cell area in km²
    fn calculate_h3_area_km2(&self, h3_cell: CellIndex) -> f64 {
        // Approximate area based on H3 resolution
        let resolution = h3_cell.resolution();
        match resolution {
            h3o::Resolution::Zero => 4_250_000.0,
            h3o::Resolution::One => 607_000.0,
            h3o::Resolution::Two => 86_700.0,
            h3o::Resolution::Three => 12_400.0,
            h3o::Resolution::Four => 1_770.0,
            h3o::Resolution::Five => 253.0,
            h3o::Resolution::Six => 36.1,
            h3o::Resolution::Seven => 5.16,
            _ => 1.0,
        }
    }
    
    /// Calculate contact area between two neighboring cells
    fn calculate_contact_area_km2(&self, cell_a: &CellLocation, _cell_b: &CellLocation) -> f64 {
        // For horizontal neighbors, contact area is roughly the edge length times height
        let edge_length_km = self.calculate_h3_edge_length_km(cell_a.h3_cell_index());
        let height_km = 10.0; // Average cell height (should come from layer config)
        
        edge_length_km * height_km
    }
    
    /// Calculate H3 cell edge length in km
    fn calculate_h3_edge_length_km(&self, h3_cell: CellIndex) -> f64 {
        // Approximate edge length based on H3 resolution
        let resolution = h3_cell.resolution();
        match resolution {
            h3o::Resolution::Zero => 1107.0,
            h3o::Resolution::One => 418.7,
            h3o::Resolution::Two => 158.2,
            h3o::Resolution::Three => 59.8,
            h3o::Resolution::Four => 22.6,
            h3o::Resolution::Five => 8.5,
            h3o::Resolution::Six => 3.2,
            h3o::Resolution::Seven => 1.2,
            _ => 1.0,
        }
    }
}

impl Default for BinaryPairBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy_mass::EnergyMass;
    use h3o::LatLng;
    
    #[test]
    fn test_binary_pair_builder_creation() {
        let builder = BinaryPairBuilder::new();
        
        // Test that builder can calculate areas and distances
        let h3_cell = LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Three);
        let area = builder.calculate_h3_area_km2(h3_cell);
        let edge_length = builder.calculate_h3_edge_length_km(h3_cell);
        
        assert!(area > 0.0);
        assert!(edge_length > 0.0);
        println!("H3 Resolution Three: area = {:.1} km², edge = {:.1} km", area, edge_length);
    }
    
    #[test]
    fn test_vertical_distance_calculation() {
        let builder = BinaryPairBuilder::new();
        
        let h3_cell = LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two);
        let upper_cell = CellLocation::new(0, h3_cell, 0);
        let lower_cell = CellLocation::new(0, h3_cell, 2);
        
        let distance = builder.calculate_vertical_distance(&upper_cell, &lower_cell);
        
        // Should be 2 depth levels * 10km = 20km
        assert_eq!(distance, 20.0);
    }
    
    #[test]
    fn test_contact_area_calculation() {
        let builder = BinaryPairBuilder::new();
        
        let h3_cell_a = LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Three);
        let h3_cell_b = LatLng::new(0.1, 0.0).unwrap().to_cell(h3o::Resolution::Three);
        
        let cell_a = CellLocation::new(0, h3_cell_a, 0);
        let cell_b = CellLocation::new(0, h3_cell_b, 0);
        
        let contact_area = builder.calculate_contact_area_km2(&cell_a, &cell_b);
        
        assert!(contact_area > 0.0);
        println!("Contact area: {:.1} km²", contact_area);
    }
}
