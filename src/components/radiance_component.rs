use crate::binary_pair::{BinaryPair, BinaryPairId, BinaryPairType};
use crate::binary_pair_listener::BinaryPairListener;
use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::constants::KM_TO_M;
use crate::material::material::MaterialPhases;
use crate::material::materials_loader::MaterialsLoader;
use crate::simulation::{Component, GeologicalCellData, Simulation, SimulationConfig};


/// Radiance Component - implements thermal radiance between cells
/// Uses Stefan-Boltzmann law: P = ε * σ * A * (T₁⁴ - T₂⁴)
/// Where ε = emissivity, σ = Stefan-Boltzmann constant, A = contact area
pub struct RadianceComponent {
    /// Stefan-Boltzmann constant (W/m²/K⁴)
    pub stefan_boltzmann_constant: f64,
    /// Default emissivity for unknown materials
    pub default_emissivity: f64,
    /// Time step scaling factor for stability
    pub time_step_factor: f64,
    /// Minimum temperature difference for radiance (K)
    pub min_temp_difference_k: f64,
}

impl RadianceComponent {
    pub fn new() -> Self {
        Self {
            stefan_boltzmann_constant: 5.670374419e-8, // W/m²/K⁴
            default_emissivity: 0.9,                   // Typical rock emissivity
            time_step_factor: 0.01,                    // Very conservative for T⁴ term
            min_temp_difference_k: 1.0,                // Skip small differences
        }
    }
    
    pub fn with_emissivity(emissivity: f64) -> Self {
        Self {
            stefan_boltzmann_constant: 5.670374419e-8,
            default_emissivity: emissivity,
            time_step_factor: 0.01,
            min_temp_difference_k: 1.0,
        }
    }
    
    /// Get emissivity for a material
    fn get_emissivity(&self, cell_data: &GeologicalCellData, location: &CellLocation) -> f64 {
        // Get material name based on layer
        let material_name = match location.layer_set_index() {
            0 => "granite",    // Crust
            1 => "basalt",     // Upper mantle
            2 => "peridotite", // Lower mantle
            _ => "iron",       // Core
        };
        
        // Get material emissivity from materials database
        if let Ok(_material) = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid) {
            // TODO: Add emissivity field to material properties
            // For now, use temperature-dependent emissivity
            let base_emissivity = self.default_emissivity;
            
            // Higher temperatures generally increase emissivity
            let temp_factor = 1.0 + (cell_data.temperature_k - 273.15) / 10000.0;
            (base_emissivity * temp_factor).min(1.0).max(0.1)
        } else {
            self.default_emissivity
        }
    }
    
    /// Calculate contact area between two cells
    fn calculate_contact_area(&self, pair: &BinaryPair, config: &SimulationConfig) -> f64 {
        match pair.pair_type {
            BinaryPairType::Horizontal => {
                // Lateral contact: area = height × contact_width
                // For H3 cells, contact width is approximately edge length
                let resolution = pair.cell_a.h3_cell_index().resolution();
                let edge_length_km = self.estimate_h3_edge_length(resolution, config.planet.radius_km);
                let height_km = self.get_cell_height_km(&pair.cell_a, config);
                
                edge_length_km * height_km * KM_TO_M * KM_TO_M // Convert km² to m²
            },
            BinaryPairType::Vertical => {
                // Vertical contact: area = cell_area (full face contact)
                use crate::utils::h3_utils::H3Utils;
                let resolution = pair.cell_a.h3_cell_index().resolution();
                let area_km2 = H3Utils::cell_area(resolution, config.planet.radius_km);
                area_km2 * 1_000_000.0 // Convert km² to m²
            }
        }
    }
    
    /// Estimate H3 edge length for lateral contact area
    fn estimate_h3_edge_length(&self, resolution: h3o::Resolution, planet_radius_km: f64) -> f64 {
        // Approximate edge length based on cell area: edge ≈ sqrt(area / 2.6)
        // (H3 cells are roughly hexagonal)
        use crate::utils::h3_utils::H3Utils;
        let area_km2 = H3Utils::cell_area(resolution, planet_radius_km);
        (area_km2 / 2.6).sqrt() // Hexagon area factor
    }
    
    /// Get cell height from layer configuration
    fn get_cell_height_km(&self, location: &CellLocation, config: &SimulationConfig) -> f64 {
        let layer_index = location.layer_set_index();
        if layer_index < config.layers.len() {
            config.layers[layer_index].height_per_step_km
        } else {
            10.0 // Fallback height in km
        }
    }
    
    /// Calculate radiant heat transfer between two cells
    fn calculate_radiance_transfer(&self, 
        cell_a_data: &GeologicalCellData, 
        cell_b_data: &GeologicalCellData,
        cell_a_location: &CellLocation,
        cell_b_location: &CellLocation,
        contact_area_m2: f64,
        time_step_years: f64) -> f64 {
        
        let temp_a = cell_a_data.temperature_k;
        let temp_b = cell_b_data.temperature_k;
        
        // Skip if temperature difference is too small
        if (temp_a - temp_b).abs() < self.min_temp_difference_k {
            return 0.0;
        }
        
        // Get emissivities for both cells
        let emissivity_a = self.get_emissivity(cell_a_data, cell_a_location);
        let emissivity_b = self.get_emissivity(cell_b_data, cell_b_location);
        let effective_emissivity = (emissivity_a + emissivity_b) / 2.0;
        
        // Stefan-Boltzmann law: P = ε * σ * A * (T₁⁴ - T₂⁴)
        let temp_a_4 = temp_a.powi(4);
        let temp_b_4 = temp_b.powi(4);
        let power_watts = effective_emissivity * self.stefan_boltzmann_constant * 
                         contact_area_m2 * (temp_a_4 - temp_b_4);
        
        // Convert to energy transfer over time step
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = time_step_years * seconds_per_year;
        let energy_joules = power_watts * time_step_seconds * self.time_step_factor;
        
        energy_joules
    }

    /// Calculate maximum energy available for transfer to achieve thermal leveling
    /// Never transfer more energy than needed to reach equilibrium with all neighbors
    fn calculate_max_energy_for_leveling(&self,
        source_data: &GeologicalCellData,
        source_location: &CellLocation,
        neighbors: &[(CellLocation, f64, f64)], // (neighbor, contact_area, potential_transfer)
        cells: &crate::collections::Collection<CellLocation, GeologicalCellData>,
        _config: &SimulationConfig) -> f64 {

        // Get source cell properties
        let source_temp = source_data.temperature_k;
        let source_mass = source_data.energy_mass.mass_kg();
        let source_specific_heat = self.get_specific_heat_capacity(source_location);

        // Calculate weighted average temperature of all neighbors
        let mut total_neighbor_thermal_mass = 0.0;
        let mut weighted_temp_sum = 0.0;

        for (neighbor_location, _contact_area, _transfer) in neighbors {
            if let Some(neighbor_data) = cells.get(neighbor_location) {
                let neighbor_mass = neighbor_data.energy_mass.mass_kg();
                let neighbor_specific_heat = self.get_specific_heat_capacity(neighbor_location);
                let neighbor_thermal_mass = neighbor_mass * neighbor_specific_heat;

                total_neighbor_thermal_mass += neighbor_thermal_mass;
                weighted_temp_sum += neighbor_data.temperature_k * neighbor_thermal_mass;
            }
        }

        if total_neighbor_thermal_mass == 0.0 {
            return 0.0; // No valid neighbors
        }

        let avg_neighbor_temp = weighted_temp_sum / total_neighbor_thermal_mass;

        // Only transfer energy if source is significantly hotter
        if source_temp <= avg_neighbor_temp + self.min_temp_difference_k {
            return 0.0;
        }

        // Calculate equilibrium temperature using thermal mass conservation
        let source_thermal_mass = source_mass * source_specific_heat;
        let total_thermal_mass = source_thermal_mass + total_neighbor_thermal_mass;

        let equilibrium_temp = (source_temp * source_thermal_mass + weighted_temp_sum) / total_thermal_mass;

        // Maximum energy to transfer = energy needed to cool source to equilibrium
        let max_energy = source_thermal_mass * (source_temp - equilibrium_temp);

        // Apply safety factor to prevent overshooting
        max_energy * 0.5 // Conservative: only transfer half of theoretical maximum
    }

    /// Get specific heat capacity for a cell based on its material
    fn get_specific_heat_capacity(&self, location: &CellLocation) -> f64 {
        let material_name = match location.layer_set_index() {
            0 => "granite",    // Crust
            1 => "basalt",     // Upper mantle
            2 => "peridotite", // Lower mantle
            _ => "iron",       // Core
        };

        if let Ok(material) = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid) {
            material.specific_heat_capacity_j_per_kg_k as f64
        } else {
            1000.0 // Fallback: typical rock specific heat J/kg/K
        }
    }

    /// Simplified radiance transfer calculation (no binary pair needed)
    fn calculate_radiance_transfer_simple(
        &self,
        source_data: &GeologicalCellData,
        target_data: &GeologicalCellData,
        contact_area_m2: f64,
        time_step_years: f64,
    ) -> f64 {
        // Stefan-Boltzmann constant (W/m²/K⁴)
        const STEFAN_BOLTZMANN: f64 = 5.670374419e-8;

        let source_temp = source_data.temperature_k;
        let target_temp = target_data.temperature_k;

        // Net radiant heat transfer: Q = ε * σ * A * (T₁⁴ - T₂⁴)
        let net_power = self.default_emissivity * STEFAN_BOLTZMANN * contact_area_m2 *
                       (source_temp.powi(4) - target_temp.powi(4));

        // Convert to energy over time step
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = time_step_years * seconds_per_year;

        net_power * time_step_seconds // Joules
    }

    /// Process energy transfers for a source cell and its neighbors
    fn process_cell_energy_transfers(
        &self,
        source_cell: CellLocation,
        neighbors_with_transfers: &[(CellLocation, f64, f64)],
        actor: &mut Actor,
        cell_map: &std::collections::HashMap<CellLocation, &GeologicalCellData>,
        _config: &SimulationConfig,
    ) {
        if let Some(source_data) = cell_map.get(&source_cell) {
            // Calculate maximum energy available for transfer
            let max_energy_available = self.calculate_max_energy_for_leveling_simple(
                source_data, &neighbors_with_transfers, cell_map
            );

            // Calculate total potential energy transfer to all neighbors
            let total_potential_transfer: f64 = neighbors_with_transfers.iter()
                .map(|(_, _, transfer)| transfer.abs()).sum();

            // Scale transfers if they exceed available energy
            let scaling_factor = if total_potential_transfer > max_energy_available {
                max_energy_available / total_potential_transfer
            } else {
                1.0
            };

            // Apply scaled energy transfers
            for &(neighbor_cell, _contact_area, potential_transfer) in neighbors_with_transfers {
                let actual_transfer = potential_transfer * scaling_factor;

                // Apply energy changes via actor
                actor.add("geological_cells", source_cell, "energy_joules", -actual_transfer);
                actor.add("geological_cells", neighbor_cell, "energy_joules", actual_transfer);
            }
        }
    }

    /// Calculate maximum energy available for leveling (simplified)
    fn calculate_max_energy_for_leveling_simple(
        &self,
        source_data: &GeologicalCellData,
        neighbors_with_transfers: &[(CellLocation, f64, f64)],
        cell_map: &std::collections::HashMap<CellLocation, &GeologicalCellData>,
    ) -> f64 {
        // Find the coolest neighbor temperature
        let min_neighbor_temp = neighbors_with_transfers.iter()
            .filter_map(|(neighbor_cell, _, _)| cell_map.get(neighbor_cell))
            .map(|data| data.temperature_k)
            .fold(f64::INFINITY, f64::min);

        if min_neighbor_temp == f64::INFINITY || source_data.temperature_k <= min_neighbor_temp {
            return 0.0; // No energy available for transfer
        }

        // Calculate energy to cool source to the coolest neighbor temperature
        let temp_difference = source_data.temperature_k - min_neighbor_temp;
        let specific_heat = 1000.0; // J/kg/K (simplified)
        let mass_kg = source_data.energy_mass.mass_kg();

        temp_difference * specific_heat * mass_kg
    }
}

impl BinaryPairListener for RadianceComponent {
    fn on_binary_pair(
        &self,
        cell_a: CellLocation,
        cell_b: CellLocation,
        relationship: BinaryPairType,
        actor: &mut Actor,
        _step: u32,
        _year: f64,
    ) {
        // Get cell data from collections (we need access to CollectionsManager here)
        // For now, we'll implement this in the Component::step method
        // TODO: Refactor to pass CollectionsManager to BinaryPairListener

        // Calculate contact area based on relationship type
        let contact_area_m2 = match relationship {
            BinaryPairType::Vertical => 1000.0,   // Vertical contact area
            BinaryPairType::Horizontal => 500.0,  // Lateral contact area
        };

        // Note: We need cell data to calculate radiance transfer
        // This will be implemented when we integrate with the BinaryPairProcessor
        println!("🌟 RadianceComponent: Processing pair {:?} -> {:?} ({})",
                 cell_a, cell_b, match relationship {
                     BinaryPairType::Vertical => "vertical",
                     BinaryPairType::Horizontal => "horizontal",
                 });
    }

    fn interested_pair_types(&self) -> Vec<BinaryPairType> {
        // RadianceComponent is interested in all pair types for thermal transfer
        vec![BinaryPairType::Vertical, BinaryPairType::Horizontal]
    }

    fn component_name(&self) -> &'static str {
        "RadianceComponent"
    }
}

impl Component for RadianceComponent {
    fn name(&self) -> &'static str {
        "RadianceComponent"
    }
    
    fn initialize(&mut self, _coll_mgr: &mut CollectionsManager, _config: &SimulationConfig) {
        println!("🌟 RadianceComponent: Initializing thermal radiance calculations...");
        // No initialization needed - radiance is calculated dynamically
    }
    
    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, year: f64, config: &SimulationConfig) {
        // Get binary pairs and geological cells
        let pairs = match coll_mgr.get::<BinaryPairId, BinaryPair>("binary_pairs") {
            Some(pairs) => pairs,
            None => {
                // No binary pairs available - skip radiance calculations
                println!("🌟 RadianceComponent: No binary pairs found, skipping radiance calculations");
                return;
            }
        };

        let cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
            .expect("geological_cells collection should exist");

        let time_step_years = config.years_per_step as f64;
        let mut transfers_calculated = 0;

        // Only print occasionally to reduce console noise
        if step % 1000 == 0 {
            println!("🌟 RadianceComponent: Processing {} binary pairs for thermal radiance", pairs.len());
        }

        // Process each binary pair for radiance transfer using the efficient listener pattern
        for entry in pairs.iter() {
            let pair = entry.value();
            let (cell_a, cell_b) = pair.get_cells();

            // Only process pairs we're interested in
            if self.interested_pair_types().contains(&pair.pair_type) {
                // Get cell data for both cells
                if let (Some(cell_a_data), Some(cell_b_data)) = (cells.get(&cell_a), cells.get(&cell_b)) {
                    // Calculate contact area based on relationship type
                    let contact_area_m2 = match pair.pair_type {
                        BinaryPairType::Vertical => 1000.0,   // Vertical contact area
                        BinaryPairType::Horizontal => 500.0,  // Lateral contact area
                    };

                    // Calculate radiance transfer using Stefan-Boltzmann law
                    let energy_transfer = self.calculate_radiance_transfer_simple(
                        &*cell_a_data, &*cell_b_data, contact_area_m2, time_step_years
                    );

                    // Only apply significant transfers
                    if energy_transfer.abs() > 1e6 {
                        if energy_transfer > 0.0 {
                            // Cell A is hotter, transfers energy to Cell B
                            actor.add("geological_cells", cell_a, "energy_joules", -energy_transfer);
                            actor.add("geological_cells", cell_b, "energy_joules", energy_transfer);
                        } else {
                            // Cell B is hotter, transfers energy to Cell A
                            actor.add("geological_cells", cell_b, "energy_joules", energy_transfer);
                            actor.add("geological_cells", cell_a, "energy_joules", -energy_transfer);
                        }
                        transfers_calculated += 1;
                    }
                }
            }
        }

        if transfers_calculated > 0 && step % 1000 == 0 {
            println!("🌟 RadianceComponent: Calculated {} energy transfers at step {}", transfers_calculated, step);
        }
    }
    
    fn complete(&mut self, _sim: &Simulation, _config: &SimulationConfig) {
        // Component cleanup - no output needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy_mass::EnergyMass;
    use h3o::Resolution;
    
    #[test]
    fn test_radiance_component_creation() {
        let component = RadianceComponent::new();
        assert_eq!(component.stefan_boltzmann_constant, 5.670374419e-8);
        assert_eq!(component.default_emissivity, 0.9);
    }
    
    #[test]
    fn test_emissivity_calculation() {
        let component = RadianceComponent::new();
        
        // Test emissivity calculation logic
        let cell_data = GeologicalCellData {
            energy_mass: EnergyMass::new(1000.0, 1000.0),
            temperature_k: 1500.0, // High temperature
            pressure_pa: 1e8,
            density_kg_m3: 3000.0,
            up_id: None,
            down_id: None,
        };
        
        let location = CellLocation::new(
            0, // layer_set_index
            h3o::CellIndex::try_from(0x85283473fffffff_u64).unwrap(),
            0, // depth_index
        );
        
        let emissivity = component.get_emissivity(&cell_data, &location);
        assert!(emissivity >= 0.1 && emissivity <= 1.0);
    }
}
