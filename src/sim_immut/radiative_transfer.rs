use crate::sim_immut::binary_operations::{CellPair, BinaryOperationResult};
use crate::transaction_manager::{Transaction, CellLocation};
use crate::energy_mass::energy_mass::EnergyMass;

/// Stefan-Boltzmann constant (W⋅m⁻²⋅K⁻⁴)
const STEFAN_BOLTZMANN_CONSTANT: f64 = 5.670374419e-8;

/// Cosmic microwave background temperature (K)
const SPACE_TEMPERATURE_K: f64 = 2.7;

/// Configuration for radiative heat transfer
#[derive(Debug, Clone)]
pub struct RadiativeTransferConfig {
    /// Time step in years
    pub years_per_step: f64,
    /// Maximum energy transfer rate per step (fraction of total energy)
    pub max_transfer_rate: f64,
    /// Enable surface-to-space radiation
    pub enable_space_radiation: bool,
    /// Enable inter-layer radiation
    pub enable_inter_layer_radiation: bool,
    /// Enable intra-layer radiation
    pub enable_intra_layer_radiation: bool,
}

impl Default for RadiativeTransferConfig {
    fn default() -> Self {
        Self {
            years_per_step: 1000.0,
            max_transfer_rate: 0.01, // 1% max transfer per step
            enable_space_radiation: true,
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: true,
        }
    }
}

/// Radiative heat transfer operator using Stefan-Boltzmann law
pub struct RadiativeTransfer {
    config: RadiativeTransferConfig,
}

impl RadiativeTransfer {
    pub fn new(config: RadiativeTransferConfig) -> Self {
        Self { config }
    }

    /// Create the radiative transfer operation for binary operations manager
    pub fn create_operation(config: RadiativeTransferConfig) -> Box<dyn Fn(&CellPair) -> BinaryOperationResult + Send + Sync> {
        Box::new(move |pair: &CellPair| -> BinaryOperationResult {
            let radiative_transfer = RadiativeTransfer::new(config.clone());
            radiative_transfer.calculate_radiative_transfer(pair)
        })
    }

    /// Calculate radiative heat transfer between two cells using the algorithm from RADIATIVE_EXCHANGE.md
    fn calculate_radiative_transfer(&self, pair: &CellPair) -> BinaryOperationResult {
        let mut transactions = Vec::new();
        let mut total_energy_transferred = 0.0;

        // Determine if this operation should be processed based on configuration
        let should_process = match self.classify_pair_type(pair) {
            PairType::SurfaceToSpace => self.config.enable_space_radiation,
            PairType::InterLayer => self.config.enable_inter_layer_radiation,
            PairType::IntraLayer => self.config.enable_intra_layer_radiation,
        };

        if !should_process {
            return BinaryOperationResult {
                transactions,
                energy_transferred_joules: 0.0,
                operation_type: "RadiativeTransfer".to_string(),
            };
        }

        // Step 1: Get temperatures (cellA.temperatureInKelvin, cellB.temperatureInKelvin)
        let temp_a = pair.cell_a_data.temperature_kelvin();
        let temp_b = if pair.cell_b.layer_set_index == usize::MAX {
            SPACE_TEMPERATURE_K // Space temperature
        } else {
            pair.cell_b_data.temperature_kelvin()
        };

        // Step 2: Calculate emissivities (cellA.emissivity, cellB.emissivity)
        let emissivity_a = self.calculate_emissivity(&pair.cell_a_data);
        let emissivity_b = if pair.cell_b.layer_set_index == usize::MAX {
            1.0 // Space is a perfect black body
        } else {
            self.calculate_emissivity(&pair.cell_b_data)
        };

        // Step 3: averageTemperature = (cellA.temperatureInKelvin + cellB.temperatureInKelvin) ÷ 2
        let average_temperature = (temp_a + temp_b) / 2.0;

        // Step 4: effectiveEmissivity = CalculateEffectiveEmissivity(cellA.emissivity, cellB.emissivity)
        let effective_emissivity = self.calculate_effective_emissivity(emissivity_a, emissivity_b);

        // Step 5: radiativeConductivity = CalculateRadiativeConductivity(averageTemperature, effectiveEmissivity, cellCenterDistanceInMeters)
        let distance_meters = pair.distance_km * 1000.0;
        let radiative_conductivity = self.calculate_radiative_conductivity(average_temperature, effective_emissivity, distance_meters);

        // Step 6: temperatureDifference = cellB.temperatureInKelvin – cellA.temperatureInKelvin
        let temperature_difference = temp_b - temp_a;

        // Step 7: energyFluxRate = radiativeConductivity × temperatureDifference (watts per square meter)
        let energy_flux_rate = radiative_conductivity * temperature_difference;

        // Step 8: energyTransfer = energyFluxRate × cellFaceAreaInSquareMeters × timeStepInSeconds
        let area_m2 = pair.contact_area_km2 * 1e6; // Convert km² to m²
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = self.config.years_per_step * seconds_per_year;
        let energy_joules = energy_flux_rate * area_m2 * time_step_seconds;

        // Apply transfer rate limiting
        let max_energy_a = pair.cell_a_data.energy_joules() * self.config.max_transfer_rate;
        let limited_energy = if energy_joules > 0.0 {
            energy_joules.min(max_energy_a)
        } else {
            energy_joules.max(-max_energy_a)
        };

        // Only create transactions if energy transfer is significant
        if limited_energy.abs() > 1e6 { // Minimum 1 MJ threshold
            total_energy_transferred = limited_energy.abs();

            if limited_energy > 0.0 {
                // Heat flows from A to B
                transactions.push(Transaction {
                    source: "RadiativeTransfer".to_string(),
                    source_cell: pair.cell_a.clone(),
                    target_cell: if pair.cell_b.layer_set_index == usize::MAX {
                        None // Energy lost to space
                    } else {
                        Some(pair.cell_b.clone())
                    },
                    energy_delta_joules: -limited_energy,
                    mass_delta_kg: 0.0,
                    description: format!("Radiative transfer: {:.2e} J from A to B", limited_energy),
                    step_id: 0, // Will be set by transaction manager
                });

                if pair.cell_b.layer_set_index != usize::MAX {
                    transactions.push(Transaction {
                        source: "RadiativeTransfer".to_string(),
                        source_cell: pair.cell_b.clone(),
                        target_cell: None, // Energy added to cell_b
                        energy_delta_joules: limited_energy,
                        mass_delta_kg: 0.0,
                        description: format!("Radiative transfer: {:.2e} J from A to B", limited_energy),
                        step_id: 0,
                    });
                }
            } else {
                // Heat flows from B to A (only if B is not space)
                if pair.cell_b.layer_set_index != usize::MAX {
                    transactions.push(Transaction {
                        source: "RadiativeTransfer".to_string(),
                        source_cell: pair.cell_b.clone(),
                        target_cell: None, // Energy added to cell_b
                        energy_delta_joules: limited_energy,
                        mass_delta_kg: 0.0,
                        description: format!("Radiative transfer: {:.2e} J from B to A", limited_energy),
                        step_id: 0,
                    });

                    transactions.push(Transaction {
                        source: "RadiativeTransfer".to_string(),
                        source_cell: pair.cell_a.clone(),
                        target_cell: None, // Energy removed from cell_a
                        energy_delta_joules: -limited_energy,
                        mass_delta_kg: 0.0,
                        description: format!("Radiative transfer: {:.2e} J from B to A", -limited_energy),
                        step_id: 0,
                    });
                }
            }
        }

        BinaryOperationResult {
            transactions,
            energy_transferred_joules: total_energy_transferred,
            operation_type: "RadiativeTransfer".to_string(),
        }
    }

    /// Get material emissivity from static material properties (performance optimized)
    fn calculate_emissivity(&self, cell: &crate::sim_immut::energy_mass_cell_immut::EnergyMassCellImmut) -> f64 {
        let material = cell.material();

        // Use static emissivity from material properties if available
        let base_emissivity = material.emissivity
            .map(|e| e as f64)
            .unwrap_or_else(|| {
                // Fallback to hardcoded values for materials without emissivity data
                match cell.material_name.as_str() {
                    "basalt" => 0.95,      // High emissivity for dark volcanic rock
                    "granite" => 0.85,     // Moderate emissivity for light rock
                    "steel" | "iron" => 0.60,  // Lower emissivity for metals
                    "water" => 0.96,       // Very high emissivity for water
                    "air" => 0.80,         // Moderate emissivity for gases
                    _ => 0.85,             // Default moderate emissivity
                }
            });

        // Apply minimal temperature-dependent adjustment for extreme temperatures only
        let temperature = cell.temperature_kelvin();
        let temp_factor = if temperature > 2000.0 {
            // Only adjust for very high temperatures (molten/plasma states)
            1.0 + (temperature - 2000.0) * 0.00005 // Very slight increase
        } else {
            1.0
        };

        (base_emissivity * temp_factor).min(1.0)
    }

    /// Calculate effective emissivity for a pair of surfaces
    fn calculate_effective_emissivity(&self, emissivity_a: f64, emissivity_b: f64) -> f64 {
        // For two parallel surfaces: 1/ε_eff = 1/ε_a + 1/ε_b - 1
        // Simplified for most geological applications
        if emissivity_b >= 0.99 {
            // If one surface is nearly a black body (like space), use the other's emissivity
            emissivity_a
        } else {
            // Harmonic mean approximation for two finite surfaces
            2.0 * emissivity_a * emissivity_b / (emissivity_a + emissivity_b)
        }
    }

    /// Calculate radiative conductivity using the algorithm from RADIATIVE_EXCHANGE.md
    fn calculate_radiative_conductivity(&self, average_temperature: f64, effective_emissivity: f64, distance_meters: f64) -> f64 {
        // From the algorithm:
        // radiativeConductivity = 4 × σ × effectiveEmissivity × averageTemperature³ ÷ cellCenterDistanceInMeters
        // Where σ is the Stefan-Boltzmann constant

        4.0 * STEFAN_BOLTZMANN_CONSTANT * effective_emissivity * average_temperature.powi(3) / distance_meters
    }

    /// Classify the type of cell pair for processing decisions
    fn classify_pair_type(&self, pair: &CellPair) -> PairType {
        if pair.cell_b.layer_set_index == usize::MAX {
            PairType::SurfaceToSpace
        } else if pair.cell_a.layer_set_index != pair.cell_b.layer_set_index {
            PairType::InterLayer
        } else {
            PairType::IntraLayer
        }
    }
}

/// Types of radiative transfer pairs
#[derive(Debug, Clone, PartialEq)]
enum PairType {
    SurfaceToSpace,
    InterLayer,
    IntraLayer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_immut::energy_mass_cell_immut::{EnergyMassCellImmut, EnergyMassCellImmutProps};
    use h3o::CellIndex;

    #[test]
    fn test_radiative_transfer_hot_to_cold() {
        println!("\n🧪 Testing Radiative Transfer: Hot to Cold");
        
        let cell_index = CellIndex::try_from(0x85283473fffffff_u64).unwrap();
        
        // Create hot cell (1000K)
        let hot_cell = EnergyMassCellImmut::new(EnergyMassCellImmutProps {
            cell_index,
            height_km: 10.0,
            top_km: 0.0,
            material_name: "basalt".to_string(),
            temperature_kelvin: 1000.0,
            pressure_pa: 1e5,
            planet_radius_km: 6371.0,
        });

        // Create cold cell (300K)
        let cold_cell = EnergyMassCellImmut::new(EnergyMassCellImmutProps {
            cell_index,
            height_km: 10.0,
            top_km: 10.0,
            material_name: "basalt".to_string(),
            temperature_kelvin: 300.0,
            pressure_pa: 1e5,
            planet_radius_km: 6371.0,
        });

        let pair = CellPair {
            cell_a: CellLocation::new(0, cell_index, 0),
            cell_b: CellLocation::new(1, cell_index, 0),
            cell_a_data: hot_cell,
            cell_b_data: cold_cell,
            distance_km: 1.0,
            contact_area_km2: 100.0,
        };

        let config = RadiativeTransferConfig::default();
        let radiative_transfer = RadiativeTransfer::new(config);
        let result = radiative_transfer.calculate_radiative_transfer(&pair);

        println!("   Energy transferred: {:.2e} J", result.energy_transferred_joules);
        println!("   Transactions created: {}", result.transactions.len());

        assert!(result.energy_transferred_joules > 0.0, "Should transfer energy from hot to cold");
        assert_eq!(result.transactions.len(), 2, "Should create two transactions for energy conservation");
    }

    #[test]
    fn test_surface_to_space_radiation() {
        println!("\n🧪 Testing Surface to Space Radiation");
        
        let cell_index = CellIndex::try_from(0x85283473fffffff_u64).unwrap();
        
        // Create surface cell (288K - Earth surface temperature)
        let surface_cell = EnergyMassCellImmut::new(EnergyMassCellImmutProps {
            cell_index,
            height_km: 1.0,
            top_km: 0.0,
            material_name: "basalt".to_string(),
            temperature_kelvin: 288.0,
            pressure_pa: 1e5,
            planet_radius_km: 6371.0,
        });

        // Create space cell (2.7K - cosmic background)
        let space_cell = EnergyMassCellImmut::new(EnergyMassCellImmutProps {
            cell_index,
            height_km: 1000.0,
            top_km: 1000.0,
            material_name: "space".to_string(),
            temperature_kelvin: SPACE_TEMPERATURE_K,
            pressure_pa: 0.0,
            planet_radius_km: 6371.0,
        });

        let pair = CellPair {
            cell_a: CellLocation::new(0, cell_index, 0),
            cell_b: CellLocation::new(usize::MAX, cell_index, 0), // Space
            cell_a_data: surface_cell,
            cell_b_data: space_cell,
            distance_km: 1000.0,
            contact_area_km2: 100.0,
        };

        let config = RadiativeTransferConfig::default();
        let radiative_transfer = RadiativeTransfer::new(config);
        let result = radiative_transfer.calculate_radiative_transfer(&pair);

        println!("   Energy lost to space: {:.2e} J", result.energy_transferred_joules);
        println!("   Transactions created: {}", result.transactions.len());

        assert!(result.energy_transferred_joules > 0.0, "Surface should radiate energy to space");
        assert_eq!(result.transactions.len(), 1, "Should create one transaction for energy loss to space");
    }
}
