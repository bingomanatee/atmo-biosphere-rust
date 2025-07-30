use crate::binary_pair::{BinaryPair, BinaryPairId, BinaryPairType};
use crate::cell_location::CellLocation;
use crate::collections::{Collection, CollectionsManager};
use crate::simulation::{GeologicalCellData, LayerConfig};
use crate::utils::h3_utils::H3Utils;
use h3o::{CellIndex, Resolution};
use std::collections::HashMap;

/// Builder for creating binary pairs between geological cells
/// Based on the deprecated binary pairing system for efficient geological operations
pub struct BinaryPairBuilder {
    // No fields needed - all methods are stateless
}

impl BinaryPairBuilder {
    pub fn new() -> Self {
        Self {
            // No fields to initialize
        }
    }
    
    /// Build all binary pairs for the simulation using layer configuration
    pub fn build_all_pairs(&mut self, coll_mgr: &mut CollectionsManager, layer_configs: &[LayerConfig]) -> Result<usize, String> {
        println!("   🔗 Creating binary pairs collection...");
        // Create binary pairs collection
        let binary_pairs_collection = crate::collections::Collection::<BinaryPairId, BinaryPair>::new();
        coll_mgr.add_collection("binary_pairs", binary_pairs_collection);

        println!("   🔗 Building vertical pairs (above/below relationships)...");
        // Build vertical pairs (above/below in same column) using layer config
        let vertical_pairs = self.build_vertical_pairs_from_config(layer_configs, coll_mgr)?;
        println!("   ✅ Created {} vertical pairs", vertical_pairs);

        println!("   🔗 Building horizontal pairs (neighbor relationships)...");
        // Build horizontal pairs (H3 neighbors at same depth) using layer config
        let horizontal_pairs = self.build_horizontal_pairs_from_config(layer_configs, coll_mgr)?;
        println!("   ✅ Created {} horizontal pairs", horizontal_pairs);

        let total_pairs = vertical_pairs + horizontal_pairs;
        println!("   🎯 Total binary pairs created: {}", total_pairs);
        Ok(total_pairs)
    }
    
    /// Build vertical pairs using layer configuration (more efficient)
    fn build_vertical_pairs_from_config(&mut self, layer_configs: &[LayerConfig], coll_mgr: &mut CollectionsManager) -> Result<usize, String> {
        let mut pairs_created = 0;

        // Get existing geological cells to only create pairs for cells that actually exist
        let geological_cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
            .ok_or("geological_cells collection not found")?;

        println!("     📊 Found {} existing geological cells", geological_cells.len());

        // Group cells by layer and H3 cell for efficient vertical pair creation
        let mut cells_by_layer_and_h3: std::collections::HashMap<(usize, h3o::CellIndex), Vec<usize>> = std::collections::HashMap::new();

        for entry in geological_cells.iter() {
            let cell_location = entry.key();
            let key = (cell_location.layer_set_index(), cell_location.h3_cell_index());
            cells_by_layer_and_h3.entry(key).or_insert_with(Vec::new).push(cell_location.depth_index());
        }

        println!("     📊 Processing {} unique (layer, H3) combinations", cells_by_layer_and_h3.len());

        let mut processed = 0;
        let total_combinations = cells_by_layer_and_h3.len();

        // For each (layer, H3 cell) combination, create vertical pairs
        for ((layer_index, h3_cell), mut depth_indices) in cells_by_layer_and_h3 {
            processed += 1;
            if processed % 100000 == 0 {
                println!("     🔄 Processed {}/{} combinations ({:.1}%)",
                         processed, total_combinations,
                         (processed as f64 / total_combinations as f64) * 100.0);
            }

            // Sort depth indices for consistent pair creation
            depth_indices.sort();

            // Create pairs between adjacent depth levels within this layer
            for i in 0..depth_indices.len().saturating_sub(1) {
                let upper_depth = depth_indices[i];
                let lower_depth = depth_indices[i + 1];

                let upper_location = CellLocation::new(layer_index, h3_cell, upper_depth);
                let lower_location = CellLocation::new(layer_index, h3_cell, lower_depth);

                let pair = BinaryPair::vertical(upper_location, lower_location);
                let pair_id = BinaryPairId::new(&pair);

                // Add pair directly to collection
                if let Some(binary_pairs) = coll_mgr.get_mut::<BinaryPairId, BinaryPair>("binary_pairs") {
                    binary_pairs.insert(pair_id, pair);
                    pairs_created += 1;
                }
            }
        }

        Ok(pairs_created)
    }
    
    /// Build horizontal pairs using layer configuration (H3 neighbors at same depth)
    fn build_horizontal_pairs_from_config(&mut self, _layer_configs: &[LayerConfig], coll_mgr: &mut CollectionsManager) -> Result<usize, String> {
        let mut pairs_created = 0;

        // Get existing geological cells to only create pairs for cells that actually exist
        let geological_cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
            .ok_or("geological_cells collection not found")?;

        // Group cells by (layer, depth) for efficient horizontal pair creation
        let mut cells_by_layer_depth: std::collections::HashMap<(usize, usize), Vec<h3o::CellIndex>> = std::collections::HashMap::new();

        for entry in geological_cells.iter() {
            let cell_location = entry.key();
            let key = (cell_location.layer_set_index(), cell_location.depth_index());
            cells_by_layer_depth.entry(key).or_insert_with(Vec::new).push(cell_location.h3_cell_index());
        }

        println!("     📊 Processing {} unique (layer, depth) combinations for horizontal pairs", cells_by_layer_depth.len());

        let mut processed = 0;
        let total_combinations = cells_by_layer_depth.len();

        // For each (layer, depth) combination, create horizontal pairs between H3 neighbors
        for ((layer_index, depth_index), h3_cells) in cells_by_layer_depth {
            processed += 1;
            let mut pairs_for_this_combination = 0;

            if processed % 1 == 0 {  // Print every combination since there are only 13
                println!("     🔄 Processing combination {}/{}: layer {}, depth {} with {} cells",
                         processed, total_combinations, layer_index, depth_index, h3_cells.len());
            }

            // Convert to HashSet for O(1) lookup instead of O(n) contains()
            let h3_cells_set: std::collections::HashSet<h3o::CellIndex> = h3_cells.iter().cloned().collect();

            // For each H3 cell at this layer/depth, find its neighbors
            for &h3_cell in &h3_cells {
                let neighbors = H3Utils::get_neighbors(h3_cell);

                for neighbor_h3 in neighbors {
                    // Only create pair if neighbor exists in our cell set and avoid duplicates
                    if h3_cells_set.contains(&neighbor_h3) && neighbor_h3 > h3_cell {
                        let cell_a = CellLocation::new(layer_index, h3_cell, depth_index);
                        let cell_b = CellLocation::new(layer_index, neighbor_h3, depth_index);

                        let pair = BinaryPair::horizontal(cell_a, cell_b);
                        let pair_id = BinaryPairId::new(&pair);

                        // Add pair directly to collection
                        if let Some(binary_pairs) = coll_mgr.get_mut::<BinaryPairId, BinaryPair>("binary_pairs") {
                            binary_pairs.insert(pair_id, pair);
                            pairs_created += 1;
                            pairs_for_this_combination += 1;
                        }
                    }
                }
            }

            println!("     ✅ Created {} horizontal pairs for layer {}, depth {}",
                     pairs_for_this_combination, layer_index, depth_index);
        }

        Ok(pairs_created)
    }
}
