use crate::binary_pairing::{BinaryPairListener, BinaryPair, BinaryPairType};
use crate::transaction_manager_simple::SimpleTransactionManager;
// use crate::energy_mass::energy_mass::EnergyMass; // Unused

/// Core Heat Component using Binary Pair Listener pattern
/// Adds irregular heat input via Perlin noise and hotspots
#[derive(Debug, Clone)]
pub struct CoreHeatListener {
    /// Total Earth heat flow in TW
    earth_wattage_tw: f64,
    /// Number of major hotspots
    hotspot_count: usize,
    /// Perlin noise variation percentage
    perlin_variation: f64,
    /// Performance tracking
    total_energy_added: f64,
    total_pairs_processed: u64,
}

impl CoreHeatListener {
    /// Create new core heat listener
    pub fn new() -> Self {
        Self {
            earth_wattage_tw: 47.0, // 47 TW total Earth heat flow
            hotspot_count: 10,
            perlin_variation: 0.15, // ±15% variation
            total_energy_added: 0.0,
            total_pairs_processed: 0,
        }
    }
    
    /// Set Earth wattage
    pub fn with_earth_wattage(mut self, wattage_tw: f64) -> Self {
        self.earth_wattage_tw = wattage_tw;
        self
    }
    
    /// Set hotspot count
    pub fn with_hotspot_count(mut self, count: usize) -> Self {
        self.hotspot_count = count;
        self
    }
    
    /// Set Perlin noise variation
    pub fn with_perlin_variation(mut self, variation: f64) -> Self {
        self.perlin_variation = variation;
        self
    }
    
    /// Calculate base energy input per cell per step
    fn calculate_base_energy_input(&self, total_cells: usize, years_per_step: f64) -> f64 {
        let total_watts = self.earth_wattage_tw * 1e12; // Convert TW to W
        let watts_per_cell = total_watts / total_cells as f64;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let energy_per_step = watts_per_cell * years_per_step * seconds_per_year;
        energy_per_step
    }
    
    /// Generate Perlin noise variation for a cell
    fn generate_perlin_variation(&self, h3_cell: u64, cell_index: usize, step: i64) -> f64 {
        // Simple pseudo-Perlin noise using cell coordinates and time
        let x = (h3_cell & 0xFFFF) as f64 / 65535.0;
        let y = ((h3_cell >> 16) & 0xFFFF) as f64 / 65535.0;
        let z = cell_index as f64 / 10.0;
        let t = step as f64 / 1000.0; // Temporal component
        
        // Simple noise function (not true Perlin but similar effect)
        let noise = ((x * 12.9898 + y * 78.233 + z * 37.719 + t * 17.139).sin() * 43758.5453).fract();
        let centered_noise = (noise - 0.5) * 2.0; // Range: -1 to 1
        
        centered_noise * self.perlin_variation
    }
    
    /// Check if this cell is a hotspot
    fn is_hotspot(&self, h3_cell: u64, cell_index: usize) -> bool {
        // Deterministic hotspot placement based on cell coordinates
        let hotspot_hash = (h3_cell.wrapping_mul(31) + cell_index as u64) % 1000;
        hotspot_hash < (self.hotspot_count * 1000 / 150) as u64 // ~10 hotspots per 150 cells
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (f64, u64) {
        (self.total_energy_added, self.total_pairs_processed)
    }
}

impl Default for CoreHeatListener {
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryPairListener for CoreHeatListener {
    fn on_binary_pair(
        &mut self,
        pair: &BinaryPair,
        transaction_manager: &mut SimpleTransactionManager,
        step: i64,
        _year: i64,
    ) {
        // Core heat only affects deep cells (not surface radiation)
        match pair.pair_type {
            BinaryPairType::HorizontalNeighbors | BinaryPairType::VerticalNeighbors => {
                self.total_pairs_processed += 1;
                
                // Only add heat to deeper cells (avoid surface interference)
                if pair.cell_a.depth_km > 10.0 {
                    let h3_cell = u64::from(pair.cell_a.location.h3_cell_index);
                    let cell_index = pair.cell_a.location.depth_index;
                    
                    // Calculate base energy input (assuming 1500 total cells, 1000 years per step)
                    let base_energy = self.calculate_base_energy_input(1500, 1000.0);
                    
                    // Apply Perlin noise variation
                    let perlin_factor = 1.0 + self.generate_perlin_variation(h3_cell, cell_index, step);
                    let energy_input = base_energy * perlin_factor;
                    
                    // Add base energy with Perlin variation
                    transaction_manager.add_energy_delta(
                        pair.cell_a.location.clone(),
                        energy_input,
                        "core_heat_perlin",
                    );
                    self.total_energy_added += energy_input;
                    
                    // Add hotspot energy if this is a hotspot cell
                    if self.is_hotspot(h3_cell, cell_index) {
                        let hotspot_energy = base_energy * 5.0; // 5x concentrated energy
                        transaction_manager.add_energy_delta(
                            pair.cell_a.location.clone(),
                            hotspot_energy,
                            "core_heat_hotspot",
                        );
                        self.total_energy_added += hotspot_energy;
                    }
                }
                
                // Also process cell_b if it exists and is deep enough
                if let Some(cell_b) = &pair.cell_b {
                    if cell_b.depth_km > 10.0 {
                        let h3_cell = u64::from(cell_b.location.h3_cell_index);
                        let cell_index = cell_b.location.depth_index;
                        
                        let base_energy = self.calculate_base_energy_input(1500, 1000.0);
                        let perlin_factor = 1.0 + self.generate_perlin_variation(h3_cell, cell_index, step);
                        let energy_input = base_energy * perlin_factor;
                        
                        transaction_manager.add_energy_delta(
                            cell_b.location.clone(),
                            energy_input,
                            "core_heat_perlin",
                        );
                        self.total_energy_added += energy_input;
                        
                        if self.is_hotspot(h3_cell, cell_index) {
                            let hotspot_energy = base_energy * 5.0;
                            transaction_manager.add_energy_delta(
                                cell_b.location.clone(),
                                hotspot_energy,
                                "core_heat_hotspot",
                            );
                            self.total_energy_added += hotspot_energy;
                        }
                    }
                }
            }
            BinaryPairType::SurfaceToSpace => {
                // Core heat doesn't affect surface-to-space radiation
            }
            BinaryPairType::Custom(_) => {
                // Handle custom pair types if needed
            }
        }
    }
    
    fn interested_pair_types(&self) -> Vec<BinaryPairType> {
        vec![
            BinaryPairType::HorizontalNeighbors,
            BinaryPairType::VerticalNeighbors,
            // Note: Not interested in SurfaceToSpace
        ]
    }
    
    fn component_key(&self) -> &'static str {
        "CoreHeatListener"
    }
}

// TODO: Fix tests after stabilization
/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_pairing::{BinaryPairCell, CellLocation};
    use crate::sim_immut::energy_mass_cell_immut::EnergyMassCellImmut;
    use crate::energy_mass::energy_mass::EnergyMass;
    use h3o::CellIndex;
    
    #[test]
    #[ignore] // TODO: Fix test after stabilization
    fn test_core_heat_listener() {
        println!("🔥 Testing Core Heat Listener");
        
        let mut listener = CoreHeatListener::new()
            .with_earth_wattage(47.0)
            .with_hotspot_count(10)
            .with_perlin_variation(0.15);
        
        let mut transaction_manager = SimpleTransactionManager::new();
        
        // Create test binary pair for deep cell
        let energy_mass = EnergyMass::new(1e20, 1e15);
        let deep_cell = EnergyMassCellImmut::new(
            CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
            energy_mass,
            "peridotite".to_string(),
            crate::material::MaterialPhases::Solid,
            50.0, 50.0, 60.0, 1e6, 0.0, 6371.0, 3.0
        );
        
        let pair = BinaryPair {
            pair_type: BinaryPairType::VerticalNeighbors,
            cell_a: BinaryPairCell {
                location: CellLocation {
                    layer_set_index: 2, // Deep layer
                    h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
                    cell_index: 3,
                },
                cell: deep_cell.clone(),
                depth_km: 50.0, // Deep enough for core heat
            },
            cell_b: Some(BinaryPairCell {
                location: CellLocation {
                    layer_set_index: 2,
                    h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
                    cell_index: 4,
                },
                cell: deep_cell.clone(),
                depth_km: 60.0,
            }),
            distance_m: 10_000.0,
            contact_area_m2: 3.6e9,
        };
        
        // Process the pair
        listener.on_binary_pair(&pair, &mut transaction_manager, 100, 100000);
        
        // Check that energy was added
        let energy_deltas = transaction_manager.get_all_energy_deltas();
        assert!(energy_deltas.len() > 0, "Should have energy additions");
        
        let (energy_added, pairs_processed) = listener.get_performance_stats();
        assert!(energy_added > 0.0, "Should have added energy");
        assert_eq!(pairs_processed, 1, "Should have processed one pair");
        
        println!("✅ Core heat listener working");
        println!("   - Energy added: {:.2e} J", energy_added);
        println!("   - Pairs processed: {}", pairs_processed);
        println!("   - Transactions created: {}", energy_deltas.len());
        
        // Test Perlin noise variation
        let h3_cell_u64 = u64::from(CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap());
        let variation1 = listener.generate_perlin_variation(h3_cell_u64, 0, 0);
        let variation2 = listener.generate_perlin_variation(h3_cell_u64, 0, 100);
        assert_ne!(variation1, variation2, "Perlin noise should vary with time");
        assert!(variation1.abs() <= 0.15, "Variation should be within ±15%");
        
        println!("   - Perlin variation test: {:.3} vs {:.3}", variation1, variation2);
    }
}
*/
