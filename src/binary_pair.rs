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

/// Binary pair representing a simple relationship between two geological cells
/// Just two location IDs for use by other components
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryPair {
    /// First cell in the pair
    pub cell_a: CellLocation,
    /// Second cell in the pair
    pub cell_b: CellLocation,
    /// Type of relationship between the cells
    pub pair_type: BinaryPairType,
}

impl BinaryPair {
    /// Create a new binary pair
    pub fn new(
        cell_a: CellLocation,
        cell_b: CellLocation,
        pair_type: BinaryPairType,
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
        }
    }
    
    /// Create a vertical pair (above/below in same column)
    pub fn vertical(
        upper_cell: CellLocation,
        lower_cell: CellLocation,
    ) -> Self {
        Self::new(
            upper_cell,
            lower_cell,
            BinaryPairType::Vertical,
        )
    }

    /// Create a horizontal pair (H3 neighbors at same depth)
    pub fn horizontal(
        cell_a: CellLocation,
        cell_b: CellLocation,
    ) -> Self {
        Self::new(
            cell_a,
            cell_b,
            BinaryPairType::Horizontal,
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

        let pair = BinaryPair::vertical(cell_a, cell_b);

        assert_eq!(pair.pair_type, BinaryPairType::Vertical);
        assert!(pair.contains_cell(&cell_a));
        assert!(pair.contains_cell(&cell_b));
    }

    #[test]
    fn test_binary_pair_ordering() {
        let cell_a = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 0);
        let cell_b = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 1);

        // Test that pairs are created with consistent ordering regardless of input order
        let pair1 = BinaryPair::vertical(cell_a, cell_b);
        let pair2 = BinaryPair::vertical(cell_b, cell_a);

        assert_eq!(pair1, pair2);
    }

    #[test]
    fn test_binary_pair_id() {
        let cell_a = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 0);
        let cell_b = CellLocation::new(0, LatLng::new(0.0, 0.0).unwrap().to_cell(h3o::Resolution::Two), 1);

        let pair = BinaryPair::vertical(cell_a, cell_b);
        let id = BinaryPairId::new(&pair);

        assert!(!id.id.is_empty());

        // Test that IDs are consistent
        let id2 = BinaryPairId::from_cells(&cell_a, &cell_b);
        let id3 = BinaryPairId::from_cells(&cell_b, &cell_a); // Reversed order

        assert_eq!(id2, id3); // Should be same regardless of order
    }
}
