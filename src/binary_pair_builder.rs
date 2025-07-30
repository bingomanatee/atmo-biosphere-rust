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
        // Create binary pairs collection
        let binary_pairs_collection = crate::collections::Collection::<BinaryPairId, BinaryPair>::new();
        coll_mgr.add_collection("binary_pairs", binary_pairs_collection);

        // Build vertical pairs (above/below in same column) using layer config
        let vertical_pairs = self.build_vertical_pairs_from_config(layer_configs, coll_mgr)?;

        // Build horizontal pairs (H3 neighbors at same depth) using layer config
        let horizontal_pairs = self.build_horizontal_pairs_from_config(layer_configs, coll_mgr)?;

        let total_pairs = vertical_pairs + horizontal_pairs;
        Ok(total_pairs)
    }
    
    /// Build vertical pairs using layer configuration (more efficient)
    fn build_vertical_pairs_from_config(&mut self, layer_configs: &[LayerConfig], coll_mgr: &mut CollectionsManager) -> Result<usize, String> {
        let mut pairs_created = 0;

        for (layer_index, layer_config) in layer_configs.iter().enumerate() {
            let resolution = layer_config.resolution;
            let depth_steps = layer_config.depth_steps;
            let height_per_step_km = layer_config.height_per_step_km;

            // Get all H3 cells at this resolution using existing iterator
            let h3_cells_iter = H3Utils::iter_cells_with_base(resolution);

            // For each H3 cell, create vertical pairs between depth levels
            for (h3_cell, _base_cell) in h3_cells_iter {
                // Create pairs between adjacent depth levels within this layer
                for depth in 0..depth_steps.saturating_sub(1) {
                    let upper_location = CellLocation::new(layer_index, h3_cell, depth);
                    let lower_location = CellLocation::new(layer_index, h3_cell, depth + 1);

                    let pair = BinaryPair::vertical(upper_location, lower_location);
                    let pair_id = BinaryPairId::new(&pair);

                    // Add pair directly to collection
                    if let Some(binary_pairs) = coll_mgr.get_mut::<BinaryPairId, BinaryPair>("binary_pairs") {
                        binary_pairs.insert(pair_id, pair);
                        pairs_created += 1;
                    }
                }

                // Create pairs between this layer and the next layer (if exists)
                if layer_index + 1 < layer_configs.len() {
                    let current_bottom = CellLocation::new(layer_index, h3_cell, depth_steps - 1);
                    let next_top = CellLocation::new(layer_index + 1, h3_cell, 0);

                    let pair = BinaryPair::vertical(current_bottom, next_top);
                    let pair_id = BinaryPairId::new(&pair);

                    if let Some(binary_pairs) = coll_mgr.get_mut::<BinaryPairId, BinaryPair>("binary_pairs") {
                        binary_pairs.insert(pair_id, pair);
                        pairs_created += 1;
                    }
                }
            }
        }

        Ok(pairs_created)
    }
    
    /// Build horizontal pairs using layer configuration (more efficient)
    fn build_horizontal_pairs_from_config(&mut self, layer_configs: &[LayerConfig], coll_mgr: &mut CollectionsManager) -> Result<usize, String> {
        let mut pairs_created = 0;

        for (layer_index, layer_config) in layer_configs.iter().enumerate() {
            let resolution = layer_config.resolution;
            let depth_steps = layer_config.depth_steps;

            // Get all H3 cells at this resolution using existing iterator
            let h3_cells: Vec<CellIndex> = H3Utils::iter_cells_with_base(resolution)
                .map(|(h3_cell, _base_cell)| h3_cell)
                .collect();

            // For each depth level in this layer
            for depth in 0..depth_steps {
                // For each H3 cell, find its neighbors and create horizontal pairs
                for h3_cell in &h3_cells {
                    let cell_location = CellLocation::new(layer_index, *h3_cell, depth);

                    // Get H3 neighbors
                    let neighbors = H3Utils::get_neighbors(*h3_cell);

                    for neighbor_h3 in neighbors {
                        // Only create pair if neighbor exists in our cell set (to avoid duplicates)
                        if h3_cells.contains(&neighbor_h3) && neighbor_h3 > *h3_cell {
                            let neighbor_location = CellLocation::new(layer_index, neighbor_h3, depth);

                            let pair = BinaryPair::horizontal(cell_location, neighbor_location);
                            let pair_id = BinaryPairId::new(&pair);

                            // Add pair directly to collection
                            if let Some(binary_pairs) = coll_mgr.get_mut::<BinaryPairId, BinaryPair>("binary_pairs") {
                                binary_pairs.insert(pair_id, pair);
                                pairs_created += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(pairs_created)
    }
}
