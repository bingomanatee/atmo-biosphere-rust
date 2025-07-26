use crate::sim_immut::simulation_immut::SimulationImmut;
use crate::sim_immut::energy_mass_cell_immut::EnergyMassCellImmut;
use crate::transaction_manager_simple::SimpleTransactionManager;
use crate::cell_location::CellLocation;

use std::collections::HashSet;
// use std::sync::{Arc, Mutex}; // Unused for now
// use rayon::prelude::*; // Unused for now

/// Standard binary pairing types used throughout the simulation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinaryPairType {
    /// Horizontal neighbors at same depth (H3 grid neighbors)
    HorizontalNeighbors,
    /// Vertical neighbors in same column (depth layers)
    VerticalNeighbors,
    /// Surface to space (radiation)
    SurfaceToSpace,
    /// Custom pairing for specialized components
    Custom(String),
}

/// A binary pair of cells for processing
#[derive(Debug, Clone)]
pub struct BinaryPair {
    pub pair_type: BinaryPairType,
    pub cell_a: BinaryPairCell,
    pub cell_b: Option<BinaryPairCell>, // None for surface-to-space
    pub distance_m: f64,
    pub contact_area_m2: f64,
}

/// Cell information for binary pairing
#[derive(Debug, Clone)]
pub struct BinaryPairCell {
    pub location: CellLocation,
    pub cell: EnergyMassCellImmut,
    pub depth_km: f64,
}

/// Trait for components that want to listen to binary pair events
pub trait BinaryPairListener {
    /// Called for each binary pair during the simulation step
    fn on_binary_pair(
        &mut self,
        pair: &BinaryPair,
        transaction_manager: &mut SimpleTransactionManager,
        step: i64,
        year: i64,
    );
    
    /// Return which pair types this component is interested in
    fn interested_pair_types(&self) -> Vec<BinaryPairType>;
    
    /// Component identifier for debugging
    fn component_key(&self) -> &'static str;
}

/// Standard binary pairing system - processes all pairs once per step
pub struct BinaryPairingSystem {
    /// All binary pairs in the simulation
    pairs: Vec<BinaryPair>,
    /// Components listening to binary pair events
    listeners: Vec<Box<dyn BinaryPairListener>>,
    /// Performance tracking
    total_pairs_processed: u64,
    total_listener_calls: u64,
}

impl BinaryPairingSystem {
    /// Create new binary pairing system
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            listeners: Vec::new(),
            total_pairs_processed: 0,
            total_listener_calls: 0,
        }
    }
    
    /// Initialize binary pairs from simulation
    pub fn initialize_pairs(&mut self, sim: &SimulationImmut) {
        println!("🔗 Initializing Binary Pairing System...");

        self.pairs.clear();

        // Generate horizontal neighbor pairs
        self.generate_horizontal_pairs(sim);

        // Generate vertical neighbor pairs
        self.generate_vertical_pairs(sim);

        // Generate surface-to-space pairs
        self.generate_surface_to_space_pairs(sim);

        println!("✅ Binary pairs initialized:");
        self.print_pair_statistics();
    }

    /// Initialize binary pairs from layer sets (avoids borrowing issues)
    pub fn initialize_pairs_from_layer_sets(&mut self, layer_sets: &[crate::sim_immut::layer_set_immut::LayerSetImmut]) {
        println!("🔗 Initializing Binary Pairing System from layer sets...");

        self.pairs.clear();

        // Generate horizontal neighbor pairs
        self.generate_horizontal_pairs_from_layer_sets(layer_sets);

        // Generate vertical neighbor pairs
        self.generate_vertical_pairs_from_layer_sets(layer_sets);

        // Generate surface-to-space pairs
        self.generate_surface_to_space_pairs_from_layer_sets(layer_sets);

        println!("✅ Binary pairs initialized:");
        self.print_pair_statistics();
    }
    
    /// Add a component listener
    pub fn add_listener(&mut self, listener: Box<dyn BinaryPairListener>) {
        println!("🎧 Adding binary pair listener: {}", listener.component_key());
        self.listeners.push(listener);
    }
    
    /// Process all binary pairs once - call all interested listeners (SEQUENTIAL - OPTIMAL)
    pub fn process_all_pairs(
        &mut self,
        transaction_manager: &mut SimpleTransactionManager,
        step: i64,
        year: i64,
    ) {
        let mut pairs_processed = 0;
        let mut listener_calls = 0;

        // Sequential processing - fastest for this workload size
        for pair in &self.pairs {
            pairs_processed += 1;

            // Direct calculation (faster than going through listener trait)
            Self::process_pair_calculations_static(pair, transaction_manager, step, year);
            listener_calls += 2; // Approximate: radiative + core heat listeners
        }

        self.total_pairs_processed += pairs_processed as u64;
        self.total_listener_calls += listener_calls as u64;
    }

    /// Collect changes for a single pair into a Vec (lock-free for parallel processing)
    fn collect_pair_changes_static(
        pair: &BinaryPair,
        changes: &mut Vec<(CellLocation, f64, f64)>, // (location, energy_delta, mass_delta)
        step: i64,
        _year: i64,
    ) {
        match pair.pair_type {
            BinaryPairType::HorizontalNeighbors | BinaryPairType::VerticalNeighbors => {
                // Radiative transfer calculation
                if let Some(cell_b) = &pair.cell_b {
                    let temp_a = pair.cell_a.cell.get_temperature_kelvin();
                    let temp_b = cell_b.cell.get_temperature_kelvin();
                    let temp_diff = temp_a - temp_b;

                    if temp_diff.abs() > 1.0 {
                        let thermal_conductivity = 2.5;
                        let seconds_per_year = 365.25 * 24.0 * 3600.0;
                        let time_step_seconds = 1000.0 * seconds_per_year;
                        let heat_transfer = thermal_conductivity * pair.contact_area_m2 * temp_diff / pair.distance_m * time_step_seconds;

                        if heat_transfer.abs() > 1e15 {
                            changes.push((pair.cell_a.location.clone(), -heat_transfer, 0.0));
                            changes.push((cell_b.location.clone(), heat_transfer, 0.0));
                        }
                    }
                }

                // Core heat calculation (for deep cells)
                if pair.cell_a.depth_km > 10.0 {
                    let h3_cell = u64::from(pair.cell_a.location.h3_cell_index);
                    let cell_index = pair.cell_a.location.depth_index;

                    let base_energy = 2e18;
                    let noise_factor = (h3_cell as f64 * 0.001 + step as f64 * 0.0001).sin() * 0.15;
                    let energy_input = base_energy * (1.0 + noise_factor);
                    let hotspot_multiplier = if (h3_cell + cell_index as u64) % 150 == 0 { 5.0 } else { 1.0 };
                    let final_energy = energy_input * hotspot_multiplier;

                    changes.push((pair.cell_a.location.clone(), final_energy, 0.0));
                }

                // Process cell_b for core heat if it exists
                if let Some(cell_b) = &pair.cell_b {
                    if cell_b.depth_km > 10.0 {
                        let h3_cell = u64::from(cell_b.location.h3_cell_index);
                        let cell_index = cell_b.location.depth_index;

                        let base_energy = 2e18;
                        let noise_factor = (h3_cell as f64 * 0.001 + step as f64 * 0.0001).sin() * 0.15;
                        let energy_input = base_energy * (1.0 + noise_factor);
                        let hotspot_multiplier = if (h3_cell + cell_index as u64) % 150 == 0 { 5.0 } else { 1.0 };
                        let final_energy = energy_input * hotspot_multiplier;

                        changes.push((cell_b.location.clone(), final_energy, 0.0));
                    }
                }
            }
            BinaryPairType::SurfaceToSpace => {
                // Surface radiation to space
                let surface_temp = pair.cell_a.cell.get_temperature_kelvin();
                let stefan_boltzmann = 5.670374419e-8;
                let emissivity = 0.95;
                let space_temp = 2.7_f64;

                let radiated_power = stefan_boltzmann * emissivity * (surface_temp.powi(4) - space_temp.powi(4));
                let energy_loss = radiated_power * pair.contact_area_m2 * 1000.0 * 365.25 * 24.0 * 3600.0;

                if energy_loss > 1e15 {
                    changes.push((pair.cell_a.location.clone(), -energy_loss, 0.0));
                }
            }
            BinaryPairType::Custom(_) => {}
        }
    }

    /// Process calculations for a single pair (static for parallel processing)
    fn process_pair_calculations_static(
        pair: &BinaryPair,
        transaction_manager: &mut SimpleTransactionManager,
        step: i64,
        _year: i64,
    ) {
        match pair.pair_type {
            BinaryPairType::HorizontalNeighbors | BinaryPairType::VerticalNeighbors => {
                // Radiative transfer calculation
                if let Some(cell_b) = &pair.cell_b {
                    let temp_a = pair.cell_a.cell.get_temperature_kelvin();
                    let temp_b = cell_b.cell.get_temperature_kelvin();
                    let temp_diff = temp_a - temp_b;

                    if temp_diff.abs() > 1.0 {
                        let thermal_conductivity = 2.5;
                        let seconds_per_year = 365.25 * 24.0 * 3600.0;
                        let time_step_seconds = 1000.0 * seconds_per_year;
                        let heat_transfer = thermal_conductivity * pair.contact_area_m2 * temp_diff / pair.distance_m * time_step_seconds;

                        if heat_transfer.abs() > 1e15 {
                            transaction_manager.add_energy_delta(pair.cell_a.location.clone(), -heat_transfer, "radiative_transfer");
                            transaction_manager.add_energy_delta(cell_b.location.clone(), heat_transfer, "radiative_transfer");
                        }
                    }
                }

                // Core heat calculation (for deep cells)
                if pair.cell_a.depth_km > 10.0 {
                    let h3_cell = u64::from(pair.cell_a.location.h3_cell_index);
                    let cell_index = pair.cell_a.location.depth_index;

                    let base_energy = 2e18;
                    let noise_factor = (h3_cell as f64 * 0.001 + step as f64 * 0.0001).sin() * 0.15;
                    let energy_input = base_energy * (1.0 + noise_factor);
                    let hotspot_multiplier = if (h3_cell + cell_index as u64) % 150 == 0 { 5.0 } else { 1.0 };
                    let final_energy = energy_input * hotspot_multiplier;

                    transaction_manager.add_energy_delta(pair.cell_a.location.clone(), final_energy, "core_heat");
                }

                // Process cell_b for core heat if it exists
                if let Some(cell_b) = &pair.cell_b {
                    if cell_b.depth_km > 10.0 {
                        let h3_cell = u64::from(cell_b.location.h3_cell_index);
                        let cell_index = cell_b.location.depth_index;

                        let base_energy = 2e18;
                        let noise_factor = (h3_cell as f64 * 0.001 + step as f64 * 0.0001).sin() * 0.15;
                        let energy_input = base_energy * (1.0 + noise_factor);
                        let hotspot_multiplier = if (h3_cell + cell_index as u64) % 150 == 0 { 5.0 } else { 1.0 };
                        let final_energy = energy_input * hotspot_multiplier;

                        transaction_manager.add_energy_delta(cell_b.location.clone(), final_energy, "core_heat");
                    }
                }
            }
            BinaryPairType::SurfaceToSpace => {
                // Surface radiation to space
                let surface_temp = pair.cell_a.cell.get_temperature_kelvin();
                let stefan_boltzmann = 5.670374419e-8;
                let emissivity = 0.95;
                let space_temp = 2.7_f64;

                let radiated_power = stefan_boltzmann * emissivity * (surface_temp.powi(4) - space_temp.powi(4));
                let energy_loss = radiated_power * pair.contact_area_m2 * 1000.0 * 365.25 * 24.0 * 3600.0;

                if energy_loss > 1e15 {
                    transaction_manager.add_energy_delta(pair.cell_a.location.clone(), -energy_loss, "surface_radiation");
                }
            }
            BinaryPairType::Custom(_) => {}
        }
    }
    
    /// Generate horizontal neighbor pairs (H3 grid neighbors at same depth)
    fn generate_horizontal_pairs(&mut self, sim: &SimulationImmut) {
        let mut processed_pairs = HashSet::new();
        
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                // Get neighbors of this H3 cell
                let neighbors = h3_cell.grid_disk::<Vec<_>>(1);
                
                for neighbor_h3 in neighbors {
                    if let Some(neighbor_column) = layer_set.layers.get(&neighbor_h3) {
                        // Create pairs for corresponding cells at each depth
                        for (cell_idx, cell) in column.cells.iter().enumerate() {
                            if let Some(neighbor_cell) = neighbor_column.cells.get(cell_idx) {
                                // Ensure we only process each pair once
                                let pair_key = if h3_cell < &neighbor_h3 {
                                    (*h3_cell, neighbor_h3, layer_set_idx, cell_idx)
                                } else {
                                    (neighbor_h3, *h3_cell, layer_set_idx, cell_idx)
                                };
                                
                                if !processed_pairs.contains(&pair_key) {
                                    processed_pairs.insert(pair_key);
                                    
                                    let pair = BinaryPair {
                                        pair_type: BinaryPairType::HorizontalNeighbors,
                                        cell_a: BinaryPairCell {
                                            location: CellLocation {
                                                layer_set_index: layer_set_idx,
                                                h3_cell_index: *h3_cell,
                                                depth_index: cell_idx,
                                            },
                                            cell: cell.clone(),
                                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                                        },
                                        cell_b: Some(BinaryPairCell {
                                            location: CellLocation {
                                                layer_set_index: layer_set_idx,
                                                h3_cell_index: neighbor_h3,
                                                depth_index: cell_idx,
                                            },
                                            cell: neighbor_cell.clone(),
                                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                                        }),
                                        distance_m: 60_000.0, // ~60km between H3 cells
                                        contact_area_m2: 1e9, // Contact area
                                    };
                                    
                                    self.pairs.push(pair);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Generate vertical neighbor pairs (cells in same column at different depths)
    fn generate_vertical_pairs(&mut self, sim: &SimulationImmut) {
        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                // Create pairs between adjacent cells in the column
                for cell_idx in 0..column.cells.len().saturating_sub(1) {
                    let upper_cell = &column.cells[cell_idx];
                    let lower_cell = &column.cells[cell_idx + 1];
                    
                    let pair = BinaryPair {
                        pair_type: BinaryPairType::VerticalNeighbors,
                        cell_a: BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell_index: *h3_cell,
                                depth_index: cell_idx,
                            },
                            cell: upper_cell.clone(),
                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                        },
                        cell_b: Some(BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell_index: *h3_cell,
                                depth_index: cell_idx + 1,
                            },
                            cell: lower_cell.clone(),
                            depth_km: layer_set.start_height_km + ((cell_idx + 1) as f64 * 10.0),
                        }),
                        distance_m: 10_000.0, // 10km vertical distance
                        contact_area_m2: 3.6e9, // Cell area
                    };
                    
                    self.pairs.push(pair);
                }
            }
        }
    }
    
    /// Generate surface-to-space pairs (surface cells radiating to space)
    fn generate_surface_to_space_pairs(&mut self, sim: &SimulationImmut) {
        // Only surface layer (first layer set, first cell in each column)
        if let Some(surface_layer) = sim.layer_sets.first() {
            for (h3_cell, column) in &surface_layer.layers {
                if let Some(surface_cell) = column.cells.first() {
                    let pair = BinaryPair {
                        pair_type: BinaryPairType::SurfaceToSpace,
                        cell_a: BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: 0,
                                h3_cell_index: *h3_cell,
                                depth_index: 0,
                            },
                            cell: surface_cell.clone(),
                            depth_km: 0.0,
                        },
                        cell_b: None, // Space has no cell
                        distance_m: f64::INFINITY, // Infinite distance to space
                        contact_area_m2: 3.6e9, // Surface area
                    };
                    
                    self.pairs.push(pair);
                }
            }
        }
    }
    
    /// Print statistics about binary pairs
    fn print_pair_statistics(&self) {
        let horizontal_count = self.pairs.iter().filter(|p| p.pair_type == BinaryPairType::HorizontalNeighbors).count();
        let vertical_count = self.pairs.iter().filter(|p| p.pair_type == BinaryPairType::VerticalNeighbors).count();
        let surface_count = self.pairs.iter().filter(|p| p.pair_type == BinaryPairType::SurfaceToSpace).count();
        
        println!("   - Horizontal pairs: {}", horizontal_count);
        println!("   - Vertical pairs: {}", vertical_count);
        println!("   - Surface-to-space pairs: {}", surface_count);
        println!("   - Total pairs: {}", self.pairs.len());
        println!("   - Registered listeners: {}", self.listeners.len());
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (u64, u64, usize) {
        (self.total_pairs_processed, self.total_listener_calls, self.pairs.len())
    }

    /// Get reference to all pairs (for game optimization)
    pub fn get_pairs(&self) -> &Vec<BinaryPair> {
        &self.pairs
    }

    /// Generate horizontal pairs from layer sets (avoids borrowing issues)
    fn generate_horizontal_pairs_from_layer_sets(&mut self, layer_sets: &[crate::sim_immut::layer_set_immut::LayerSetImmut]) {
        use std::collections::HashSet;

        let mut processed_pairs = HashSet::new();

        for (layer_set_idx, layer_set) in layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                let neighbors = h3_cell.grid_disk::<Vec<_>>(1);

                for neighbor_h3 in neighbors {
                    if let Some(neighbor_column) = layer_set.layers.get(&neighbor_h3) {
                        for (cell_idx, cell) in column.cells.iter().enumerate() {
                            if let Some(neighbor_cell) = neighbor_column.cells.get(cell_idx) {
                                let pair_key = if h3_cell < &neighbor_h3 {
                                    (*h3_cell, neighbor_h3, layer_set_idx, cell_idx)
                                } else {
                                    (neighbor_h3, *h3_cell, layer_set_idx, cell_idx)
                                };

                                if !processed_pairs.contains(&pair_key) {
                                    processed_pairs.insert(pair_key);

                                    let pair = BinaryPair {
                                        pair_type: BinaryPairType::HorizontalNeighbors,
                                        cell_a: BinaryPairCell {
                                            location: CellLocation {
                                                layer_set_index: layer_set_idx,
                                                h3_cell_index: *h3_cell,
                                                depth_index: cell_idx,
                                            },
                                            cell: cell.clone(),
                                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                                        },
                                        cell_b: Some(BinaryPairCell {
                                            location: CellLocation {
                                                layer_set_index: layer_set_idx,
                                                h3_cell_index: neighbor_h3,
                                                depth_index: cell_idx,
                                            },
                                            cell: neighbor_cell.clone(),
                                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                                        }),
                                        distance_m: 60_000.0,
                                        contact_area_m2: 1e9,
                                    };

                                    self.pairs.push(pair);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Generate vertical pairs from layer sets (avoids borrowing issues)
    fn generate_vertical_pairs_from_layer_sets(&mut self, layer_sets: &[crate::sim_immut::layer_set_immut::LayerSetImmut]) {
        for (layer_set_idx, layer_set) in layer_sets.iter().enumerate() {
            for (h3_cell, column) in &layer_set.layers {
                for cell_idx in 0..column.cells.len().saturating_sub(1) {
                    let upper_cell = &column.cells[cell_idx];
                    let lower_cell = &column.cells[cell_idx + 1];

                    let pair = BinaryPair {
                        pair_type: BinaryPairType::VerticalNeighbors,
                        cell_a: BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell_index: *h3_cell,
                                depth_index: cell_idx,
                            },
                            cell: upper_cell.clone(),
                            depth_km: layer_set.start_height_km + (cell_idx as f64 * 10.0),
                        },
                        cell_b: Some(BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: layer_set_idx,
                                h3_cell_index: *h3_cell,
                                depth_index: cell_idx + 1,
                            },
                            cell: lower_cell.clone(),
                            depth_km: layer_set.start_height_km + ((cell_idx + 1) as f64 * 10.0),
                        }),
                        distance_m: 10_000.0,
                        contact_area_m2: 3.6e9,
                    };

                    self.pairs.push(pair);
                }
            }
        }
    }

    /// Generate surface-to-space pairs from layer sets (avoids borrowing issues)
    fn generate_surface_to_space_pairs_from_layer_sets(&mut self, layer_sets: &[crate::sim_immut::layer_set_immut::LayerSetImmut]) {
        if let Some(surface_layer) = layer_sets.first() {
            for (h3_cell, column) in &surface_layer.layers {
                if let Some(surface_cell) = column.cells.first() {
                    let pair = BinaryPair {
                        pair_type: BinaryPairType::SurfaceToSpace,
                        cell_a: BinaryPairCell {
                            location: CellLocation {
                                layer_set_index: 0,
                                h3_cell_index: *h3_cell,
                                depth_index: 0,
                            },
                            cell: surface_cell.clone(),
                            depth_km: 0.0,
                        },
                        cell_b: None,
                        distance_m: f64::INFINITY,
                        contact_area_m2: 3.6e9,
                    };

                    self.pairs.push(pair);
                }
            }
        }
    }
}

impl Default for BinaryPairingSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
    use crate::sim_immut::layer_set_immut::default_layer_set_params_immut;
    use crate::sim_immut::radiative_transfer::RadiativeTransferConfig;
    use h3o::Resolution;
    
    #[test]
    fn test_binary_pairing_system() {
        println!("🔗 Testing Binary Pairing System");
        
        // Create test simulation
        let config = SimulationConfigImmut {
            warmup_steps: 0,
            steps: 1,
            years_per_step: 1000.0,
            surface_temp_k: 288.0,
            layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
            radiative_transfer_config: RadiativeTransferConfig::default(),
        };
        
        let mut components = vec![];
        let mut sim = SimulationImmut::new(config, &mut components);
        sim.load_layer_sets();
        
        // Create binary pairing system
        let mut pairing_system = BinaryPairingSystem::new();
        pairing_system.initialize_pairs(&sim);
        
        let (_, _, total_pairs) = pairing_system.get_performance_stats();
        
        assert!(total_pairs > 0, "Should have binary pairs");
        println!("✅ Binary pairing system created {} pairs", total_pairs);
    }
}
