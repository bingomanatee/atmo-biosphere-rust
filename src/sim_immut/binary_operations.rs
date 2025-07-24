use crate::sim_immut::energy_mass_cell_immut::EnergyMassCellImmut;
use crate::sim_immut::layer_set_immut::{LayerSetImmut, ColumnImmut};
use crate::transaction_manager::{Transaction, CellLocation};
use h3o::CellIndex;
use rayon::prelude::*;
use std::collections::HashMap;

/// Represents a pair of neighboring cells for binary operations
#[derive(Debug, Clone)]
pub struct CellPair {
    pub cell_a: CellLocation,
    pub cell_b: CellLocation,
    pub cell_a_data: EnergyMassCellImmut,
    pub cell_b_data: EnergyMassCellImmut,
    pub distance_km: f64,
    pub contact_area_km2: f64,
}

/// Types of neighbor relationships
#[derive(Debug, Clone, PartialEq)]
pub enum NeighborType {
    /// Horizontal neighbors within the same layer
    Horizontal,
    /// Vertical neighbors between layers
    Vertical,
    /// Surface to space radiation
    SurfaceToSpace,
}

/// Result of a binary operation between two cells
#[derive(Debug, Clone)]
pub struct BinaryOperationResult {
    pub transactions: Vec<Transaction>,
    pub energy_transferred_joules: f64,
    pub operation_type: String,
}

/// Binary operation function type
pub type BinaryOperation = Box<dyn Fn(&CellPair) -> BinaryOperationResult + Send + Sync>;

/// Manager for binary operations between neighboring cells
pub struct BinaryOperationsManager {
    /// Pre-computed neighbor pairs for efficient parallel processing
    neighbor_pairs: Vec<CellPair>,
    /// Registered binary operations
    operations: Vec<(String, BinaryOperation)>,
}

impl BinaryOperationsManager {
    pub fn new() -> Self {
        Self {
            neighbor_pairs: Vec::new(),
            operations: Vec::new(),
        }
    }

    /// Register a binary operation
    pub fn register_operation<F>(&mut self, name: String, operation: F)
    where
        F: Fn(&CellPair) -> BinaryOperationResult + Send + Sync + 'static,
    {
        self.operations.push((name, Box::new(operation)));
    }

    /// Build neighbor pairs from layer sets (call this when layer structure changes)
    pub fn build_neighbor_pairs(&mut self, layer_sets: &[LayerSetImmut]) {
        self.neighbor_pairs.clear();
        
        // Build horizontal neighbors within layers
        self.build_horizontal_neighbors(layer_sets);
        
        // Build vertical neighbors between layers
        self.build_vertical_neighbors(layer_sets);
        
        // Build surface-to-space pairs
        self.build_surface_to_space_pairs(layer_sets);
        
        println!("🔗 Built {} neighbor pairs for binary operations", self.neighbor_pairs.len());
    }

    /// Execute all registered operations on neighbor pairs in parallel
    pub fn execute_operations(&self) -> Vec<BinaryOperationResult> {
        if self.neighbor_pairs.is_empty() {
            return Vec::new();
        }

        // Execute operations in parallel across all neighbor pairs
        self.neighbor_pairs
            .par_iter()
            .flat_map(|pair| {
                self.operations
                    .iter()
                    .map(|(name, operation)| {
                        let mut result = operation(pair);
                        result.operation_type = name.clone();
                        result
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Build horizontal neighbor pairs within each layer
    fn build_horizontal_neighbors(&mut self, layer_sets: &[LayerSetImmut]) {
        for (layer_idx, layer_set) in layer_sets.iter().enumerate() {
            for (cell_index, column) in &layer_set.layers {
                // Get H3 neighbors for this cell
                let h3_neighbors = cell_index.grid_disk::<Vec<_>>(1);
                
                for neighbor_index in h3_neighbors {
                    if neighbor_index == *cell_index {
                        continue; // Skip self
                    }
                    
                    if let Some(neighbor_column) = layer_set.layers.get(&neighbor_index) {
                        // Create pairs for each depth level
                        for (depth_idx, cell) in column.cells.iter().enumerate() {
                            if let Some(neighbor_cell) = neighbor_column.cells.get(depth_idx) {
                                let pair = CellPair {
                                    cell_a: CellLocation::new(layer_idx, *cell_index, depth_idx),
                                    cell_b: CellLocation::new(layer_idx, neighbor_index, depth_idx),
                                    cell_a_data: cell.clone(),
                                    cell_b_data: neighbor_cell.clone(),
                                    distance_km: self.calculate_horizontal_distance(*cell_index, neighbor_index),
                                    contact_area_km2: self.calculate_contact_area(cell, neighbor_cell),
                                };
                                self.neighbor_pairs.push(pair);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Build vertical neighbor pairs between layers
    fn build_vertical_neighbors(&mut self, layer_sets: &[LayerSetImmut]) {
        for layer_idx in 0..layer_sets.len().saturating_sub(1) {
            let current_layer = &layer_sets[layer_idx];
            let next_layer = &layer_sets[layer_idx + 1];
            
            for (cell_index, current_column) in &current_layer.layers {
                if let Some(next_column) = next_layer.layers.get(cell_index) {
                    // Connect bottom of current layer to top of next layer
                    if let (Some(current_bottom), Some(next_top)) = 
                        (current_column.cells.last(), next_column.cells.first()) {
                        
                        let pair = CellPair {
                            cell_a: CellLocation::new(layer_idx, *cell_index, current_column.cells.len() - 1),
                            cell_b: CellLocation::new(layer_idx + 1, *cell_index, 0),
                            cell_a_data: current_bottom.clone(),
                            cell_b_data: next_top.clone(),
                            distance_km: (current_bottom.bottom_km - next_top.top_km).abs(),
                            contact_area_km2: current_bottom.area(),
                        };
                        self.neighbor_pairs.push(pair);
                    }
                }
            }
        }
    }

    /// Build surface-to-space pairs for radiative cooling
    fn build_surface_to_space_pairs(&mut self, layer_sets: &[LayerSetImmut]) {
        if let Some(surface_layer) = layer_sets.first() {
            for (cell_index, column) in &surface_layer.layers {
                if let Some(surface_cell) = column.cells.first() {
                    // Create a virtual "space" cell for radiation calculations
                    let space_cell = EnergyMassCellImmut {
                        cell_index: *cell_index,
                        energy_joules: 0.0,
                        mass_kg: 1.0, // Minimal mass for space
                        material_name: "space".to_string(),
                        material_phase: crate::material::MaterialPhases::Gas,
                        height_km: 1000.0, // Virtual space height
                        top_km: surface_cell.top_km + 1000.0,
                        bottom_km: surface_cell.top_km,
                        pressure_pa: 0.0, // Vacuum
                        phase_transition_energy_bank: 0.0,
                        planet_radius_km: surface_cell.planet_radius_km,
                        conductivity_w_m_k: 0.0,
                    };
                    
                    let pair = CellPair {
                        cell_a: CellLocation::new(0, *cell_index, 0),
                        cell_b: CellLocation::new(usize::MAX, *cell_index, 0), // Special layer index for space
                        cell_a_data: surface_cell.clone(),
                        cell_b_data: space_cell,
                        distance_km: 1000.0, // Distance to effective radiating space
                        contact_area_km2: surface_cell.area(),
                    };
                    self.neighbor_pairs.push(pair);
                }
            }
        }
    }

    /// Calculate horizontal distance between H3 cells
    fn calculate_horizontal_distance(&self, cell_a: CellIndex, cell_b: CellIndex) -> f64 {
        use h3o::LatLng;
        
        let latlng_a = LatLng::from(cell_a);
        let latlng_b = LatLng::from(cell_b);
        
        // Haversine distance calculation
        let lat1 = latlng_a.lat();
        let lon1 = latlng_a.lng();
        let lat2 = latlng_b.lat();
        let lon2 = latlng_b.lng();
        
        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;
        
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        
        6371.0 * c // Earth radius in km
    }

    /// Calculate contact area between two cells
    fn calculate_contact_area(&self, cell_a: &EnergyMassCellImmut, cell_b: &EnergyMassCellImmut) -> f64 {
        // Use the smaller of the two cell areas for contact
        cell_a.area().min(cell_b.area())
    }

    /// Get neighbor pairs for inspection/debugging
    pub fn get_neighbor_pairs(&self) -> &[CellPair] {
        &self.neighbor_pairs
    }

    /// Get statistics about neighbor pairs
    pub fn get_statistics(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        let horizontal_count = self.neighbor_pairs.iter()
            .filter(|pair| pair.cell_a.layer_set_index == pair.cell_b.layer_set_index)
            .count();

        let vertical_count = self.neighbor_pairs.iter()
            .filter(|pair| pair.cell_a.layer_set_index != pair.cell_b.layer_set_index && pair.cell_b.layer_set_index != usize::MAX)
            .count();

        let space_count = self.neighbor_pairs.iter()
            .filter(|pair| pair.cell_b.layer_set_index == usize::MAX)
            .count();
        
        stats.insert("horizontal_pairs".to_string(), horizontal_count);
        stats.insert("vertical_pairs".to_string(), vertical_count);
        stats.insert("surface_to_space_pairs".to_string(), space_count);
        stats.insert("total_pairs".to_string(), self.neighbor_pairs.len());
        
        stats
    }
}
