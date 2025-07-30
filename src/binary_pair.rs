use crate::cell_location::CellLocation;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Types of binary pair relationships between cells
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinaryPairType {
    /// Vertical pair: cell above and cell below in same column
    Vertical,
    /// Horizontal pair: neighboring H3 cells at same depth
    Horizontal,
}

/// Binary pair representing a relationship between two geological cells
/// Based on the deprecated binary pairing system for efficient geological operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryPair {
    /// First cell in the pair
    pub cell_a: CellLocation,
    /// Second cell in the pair
    pub cell_b: CellLocation,
    /// Type of relationship between the cells
    pub pair_type: BinaryPairType,
    /// Distance between cells (km) for calculations
    pub distance_km: f64,
    /// Contact area between cells (km²) for heat/mass transfer
    pub contact_area_km2: f64,
}

impl BinaryPair {
    /// Create a new binary pair
    pub fn new(
        cell_a: CellLocation,
        cell_b: CellLocation,
        pair_type: BinaryPairType,
        distance_km: f64,
        contact_area_km2: f64,
    ) -> Self {
        // Ensure consistent ordering: smaller cell always comes first
        let (cell_a, cell_b) = if cell_a < cell_b {
            (cell_a, cell_b)
        } else {
            (cell_b, cell_a)
        };
        
        Self {
            cell_a,
            cell_b,
            pair_type,
            distance_km,
            contact_area_km2,
        }
    }
    
    /// Create a vertical pair (above/below in same column)
    pub fn vertical(
        upper_cell: CellLocation,
        lower_cell: CellLocation,
        height_km: f64,
        area_km2: f64,
    ) -> Self {
        Self::new(
            upper_cell,
            lower_cell,
            BinaryPairType::Vertical,
            height_km,
            area_km2,
        )
    }
    
    /// Create a horizontal pair (H3 neighbors at same depth)
    pub fn horizontal(
        cell_a: CellLocation,
        cell_b: CellLocation,
        distance_km: f64,
        contact_area_km2: f64,
    ) -> Self {
        Self::new(
            cell_a,
            cell_b,
            BinaryPairType::Horizontal,
            distance_km,
            contact_area_km2,
        )
    }
    
    /// Get the other cell in the pair
    pub fn get_other_cell(&self, cell: &CellLocation) -> Option<CellLocation> {
        if &self.cell_a == cell {
            Some(self.cell_b)
        } else if &self.cell_b == cell {
            Some(self.cell_a)
        } else {
            None
        }
    }
    
    /// Check if this pair contains the given cell
    pub fn contains_cell(&self, cell: &CellLocation) -> bool {
        &self.cell_a == cell || &self.cell_b == cell
    }
    
    /// Get both cells as a tuple
    pub fn get_cells(&self) -> (CellLocation, CellLocation) {
        (self.cell_a, self.cell_b)
    }
    
    /// Calculate thermal conductance for this pair (W/K)
    /// Based on contact area, distance, and material properties
    pub fn thermal_conductance(&self, conductivity_w_m_k: f64) -> f64 {
        // Thermal conductance = k * A / d
        // Convert km to m: area_km2 * 1e6 = area_m2, distance_km * 1e3 = distance_m
        let area_m2 = self.contact_area_km2 * 1_000_000.0;
        let distance_m = self.distance_km * 1_000.0;
        
        if distance_m > 0.0 {
            conductivity_w_m_k * area_m2 / distance_m
        } else {
            0.0
        }
    }
    
    /// Calculate mass transfer coefficient for this pair
    /// Based on geological permeability and pressure gradients
    pub fn mass_transfer_coefficient(&self, permeability_m2: f64, viscosity_pa_s: f64) -> f64 {
        // Darcy's law: k * A / (μ * L)
        let area_m2 = self.contact_area_km2 * 1_000_000.0;
        let distance_m = self.distance_km * 1_000.0;
        
        if distance_m > 0.0 && viscosity_pa_s > 0.0 {
            permeability_m2 * area_m2 / (viscosity_pa_s * distance_m)
        } else {
            0.0
        }
    }
    
    /// Get a unique identifier for this pair (for use as collection key)
    pub fn get_id(&self) -> String {
        format!("{}_{}_{}_{}", 
                self.cell_a.layer_set_index(), self.cell_a.depth_index(),
                self.cell_b.layer_set_index(), self.cell_b.depth_index())
    }
}

impl Eq for BinaryPair {}

impl Hash for BinaryPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash based on both cells to ensure uniqueness
        self.cell_a.hash(state);
        self.cell_b.hash(state);
        self.pair_type.hash(state);
    }
}

/// Unique identifier for a binary pair (for use as collection key)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BinaryPairId {
    /// Unique string identifier
    pub id: String,
}

impl BinaryPairId {
    pub fn new(pair: &BinaryPair) -> Self {
        Self {
            id: pair.get_id(),
        }
    }
    
    pub fn from_cells(cell_a: &CellLocation, cell_b: &CellLocation) -> Self {
        let (cell_a, cell_b) = if cell_a < cell_b {
            (cell_a, cell_b)
        } else {
            (cell_b, cell_a)
        };
        
        Self {
            id: format!("{}_{}_{}_{}", 
                       cell_a.layer_set_index(), cell_a.depth_index(),
                       cell_b.layer_set_index(), cell_b.depth_index()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use h3o::LatLng;
    
    #[test]
    fn test_binary_pair_creation() {
        let cell_a = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 0);
        let cell_b = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 1);
        
        let pair = BinaryPair::vertical(cell_a, cell_b, 10.0, 100.0);
        
        assert_eq!(pair.pair_type, BinaryPairType::Vertical);
        assert_eq!(pair.distance_km, 10.0);
        assert_eq!(pair.contact_area_km2, 100.0);
        assert!(pair.contains_cell(&cell_a));
        assert!(pair.contains_cell(&cell_b));
    }
    
    #[test]
    fn test_binary_pair_ordering() {
        let cell_a = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 0);
        let cell_b = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 1);
        
        // Test that pairs are created with consistent ordering regardless of input order
        let pair1 = BinaryPair::vertical(cell_a, cell_b, 10.0, 100.0);
        let pair2 = BinaryPair::vertical(cell_b, cell_a, 10.0, 100.0);
        
        assert_eq!(pair1, pair2);
    }
    
    #[test]
    fn test_thermal_conductance() {
        let cell_a = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 0);
        let cell_b = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 1);
        
        let pair = BinaryPair::vertical(cell_a, cell_b, 10.0, 100.0); // 10km distance, 100km² area
        let conductivity = 3.0; // W/m/K (typical rock)
        
        let conductance = pair.thermal_conductance(conductivity);
        
        // Expected: 3.0 * (100 * 1e6) / (10 * 1e3) = 3.0 * 1e8 / 1e4 = 3e4 W/K
        assert!((conductance - 30_000.0).abs() < 1.0);
    }
    
    #[test]
    fn test_binary_pair_id() {
        let cell_a = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 0);
        let cell_b = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 1);
        
        let pair = BinaryPair::vertical(cell_a, cell_b, 10.0, 100.0);
        let id = BinaryPairId::new(&pair);
        
        assert!(!id.id.is_empty());
        
        // Test that IDs are consistent
        let id2 = BinaryPairId::from_cells(&cell_a, &cell_b);
        let id3 = BinaryPairId::from_cells(&cell_b, &cell_a); // Reversed order
        
        assert_eq!(id2, id3); // Should be same regardless of order
    }
}
