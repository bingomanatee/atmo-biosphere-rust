use h3o::CellIndex;

/// Three-dimensional cell identifier for geological simulations
/// 
/// This struct uniquely identifies a cell in the 3D geological simulation space:
/// - `layer_set_index`: Which geological layer (0=crust, 1=upper mantle, etc.)
/// - `h3_cell_index`: H3 geographical cell for horizontal positioning
/// - `depth_index`: Depth within the vertical column (0=top, 1=deeper, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellLocation {
    pub layer_set_index: usize,    // Which layer set (0=crust, 1=upper mantle, etc.)
    pub h3_cell_index: CellIndex,  // H3 geographical cell
    pub depth_index: usize,        // Depth within the column (0=top, 1=deeper, etc.)
}

impl CellLocation {
    /// Create a new CellLocation
    pub fn new(layer_set_index: usize, h3_cell_index: CellIndex, depth_index: usize) -> Self {
        Self {
            layer_set_index,
            h3_cell_index,
            depth_index,
        }
    }

    /// Get a human-readable description of this cell location
    pub fn description(&self) -> String {
        format!("Layer[{}]:H3[{}]:Depth[{}]",
            self.layer_set_index,
            self.h3_cell_index,
            self.depth_index)
    }

    /// Get the layer set index
    pub fn layer_set_index(&self) -> usize {
        self.layer_set_index
    }

    /// Get the H3 cell index
    pub fn h3_cell_index(&self) -> CellIndex {
        self.h3_cell_index
    }

    /// Get the depth index
    pub fn depth_index(&self) -> usize {
        self.depth_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_location_creation() {
        let h3_index = CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap();
        let location = CellLocation::new(0, h3_index, 5);
        
        assert_eq!(location.layer_set_index, 0);
        assert_eq!(location.h3_cell_index, h3_index);
        assert_eq!(location.depth_index, 5);
    }

    #[test]
    fn test_cell_location_description() {
        let h3_index = CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap();
        let location = CellLocation::new(1, h3_index, 3);
        
        let description = location.description();
        assert!(description.contains("Layer[1]"));
        assert!(description.contains("Depth[3]"));
    }

    #[test]
    fn test_cell_location_hash() {
        use std::collections::HashMap;
        
        let h3_index = CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap();
        let location1 = CellLocation::new(0, h3_index, 1);
        let location2 = CellLocation::new(0, h3_index, 1);
        let location3 = CellLocation::new(0, h3_index, 2);
        
        let mut map = HashMap::new();
        map.insert(location1.clone(), "value1");
        map.insert(location3.clone(), "value3");
        
        assert_eq!(map.get(&location2), Some(&"value1"));
        assert_eq!(map.get(&location3), Some(&"value3"));
        assert_eq!(map.len(), 2);
    }
}
