use crate::binary_pairing::{BinaryPairListener, BinaryPair, BinaryPairType};
use crate::transaction_manager_simple::SimpleTransactionManager;
use crate::energy_mass::energy_mass::EnergyMass;

/// Radiative Transfer Component using Binary Pair Listener pattern
#[derive(Debug, Clone)]
pub struct RadiativeTransferListener {
    /// Thermal conductivity for different materials
    thermal_conductivity: f64,
    /// Performance tracking
    total_energy_transferred: f64,
    total_pairs_processed: u64,
}

impl RadiativeTransferListener {
    /// Create new radiative transfer listener
    pub fn new() -> Self {
        Self {
            thermal_conductivity: 2.5, // W/m·K
            total_energy_transferred: 0.0,
            total_pairs_processed: 0,
        }
    }
    
    /// Create with custom thermal conductivity
    pub fn with_conductivity(mut self, conductivity: f64) -> Self {
        self.thermal_conductivity = conductivity;
        self
    }
    
    /// Calculate heat transfer between two cells
    fn calculate_heat_transfer(
        &self,
        temp1: f64,
        temp2: f64,
        distance: f64,
        contact_area: f64,
        time_step_years: f64,
    ) -> f64 {
        // Heat transfer equation: Q = k * A * (T1 - T2) / d * t
        let temp_difference = temp1 - temp2;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = time_step_years * seconds_per_year;
        
        self.thermal_conductivity * contact_area * temp_difference / distance * time_step_seconds
    }
    
    /// Get performance statistics
    pub fn get_performance_stats(&self) -> (f64, u64) {
        (self.total_energy_transferred, self.total_pairs_processed)
    }
}

impl Default for RadiativeTransferListener {
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryPairListener for RadiativeTransferListener {
    fn on_binary_pair(
        &mut self,
        pair: &BinaryPair,
        transaction_manager: &mut SimpleTransactionManager,
        _step: i64,
        _year: i64,
    ) {
        self.total_pairs_processed += 1;
        
        match pair.pair_type {
            BinaryPairType::HorizontalNeighbors | BinaryPairType::VerticalNeighbors => {
                if let Some(cell_b) = &pair.cell_b {
                    let temp_a = pair.cell_a.cell.get_temperature_kelvin();
                    let temp_b = cell_b.cell.get_temperature_kelvin();
                    
                    let heat_transfer = self.calculate_heat_transfer(
                        temp_a,
                        temp_b,
                        pair.distance_m,
                        pair.contact_area_m2,
                        1000.0, // 1000 years per step
                    );
                    
                    // Only process significant heat transfers
                    if heat_transfer.abs() > 1e15 {
                        // Energy flows from hot to cold
                        transaction_manager.add_energy_delta(
                            pair.cell_a.location,
                            -heat_transfer,
                            "radiative_transfer",
                        );
                        transaction_manager.add_energy_delta(
                            cell_b.location,
                            heat_transfer,
                            "radiative_transfer",
                        );
                        
                        self.total_energy_transferred += heat_transfer.abs();
                    }
                }
            }
            BinaryPairType::SurfaceToSpace => {
                // Handle surface radiation to space
                let surface_temp = pair.cell_a.cell.get_temperature_kelvin();
                let stefan_boltzmann = 5.670374419e-8; // W/m²·K⁴
                let emissivity = 0.95;
                let space_temp = 2.7_f64; // Cosmic background temperature
                
                let radiated_power = stefan_boltzmann * emissivity *
                    (surface_temp.powi(4) - space_temp.powi(4)); // W/m²
                
                let seconds_per_year = 365.25 * 24.0 * 3600.0;
                let energy_loss = radiated_power * pair.contact_area_m2 * 
                    1000.0 * seconds_per_year; // 1000 years per step
                
                if energy_loss > 1e15 {
                    transaction_manager.add_energy_delta(
                        pair.cell_a.location,
                        -energy_loss,
                        "surface_radiation",
                    );
                    
                    self.total_energy_transferred += energy_loss;
                }
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
            BinaryPairType::SurfaceToSpace,
        ]
    }
    
    fn component_key(&self) -> &'static str {
        "RadiativeTransferListener"
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
    fn test_radiative_transfer_listener() {
        println!("🌡️ Testing Radiative Transfer Listener");
        
        let mut listener = RadiativeTransferListener::new();
        let mut transaction_manager = SimpleTransactionManager::new();
        
        // Create test binary pair
        // Create test cells using proper constructor
        let energy_mass_a = crate::energy_mass::energy_mass::EnergyMass::new(1e20, 1e15);
        let energy_mass_b = crate::energy_mass::energy_mass::EnergyMass::new(5e19, 1e15);

        let cell_a = EnergyMassCellImmut::new(
            CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
            energy_mass_a,
            "basalt".to_string(),
            crate::material::MaterialPhases::Solid,
            0.0, 0.0, 10.0, 1e5, 0.0, 6371.0, 2.5
        );
        let cell_b = EnergyMassCellImmut::new(
            CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
            energy_mass_b,
            "basalt".to_string(),
            crate::material::MaterialPhases::Solid,
            10.0, 10.0, 20.0, 1e5, 0.0, 6371.0, 2.5
        );
        
        let pair = BinaryPair {
            pair_type: BinaryPairType::HorizontalNeighbors,
            cell_a: BinaryPairCell {
                location: CellLocation {
                    layer_set_index: 0,
                    h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
                    cell_index: 0,
                },
                cell: cell_a,
                depth_km: 0.0,
            },
            cell_b: Some(BinaryPairCell {
                location: CellLocation {
                    layer_set_index: 0,
                    h3_cell: CellIndex::try_from(0x8a1fb46622dffff_u64).unwrap(),
                    cell_index: 1,
                },
                cell: cell_b,
                depth_km: 0.0,
            }),
            distance_m: 60_000.0,
            contact_area_m2: 1e9,
        };
        
        // Process the pair
        listener.on_binary_pair(&pair, &mut transaction_manager, 0, 0);
        
        // Check that transactions were created
        let energy_deltas = transaction_manager.get_all_energy_deltas();
        assert!(energy_deltas.len() > 0, "Should have energy transactions");
        
        let (energy_transferred, pairs_processed) = listener.get_performance_stats();
        assert!(energy_transferred > 0.0, "Should have transferred energy");
        assert_eq!(pairs_processed, 1, "Should have processed one pair");
        
        println!("✅ Radiative transfer listener working");
        println!("   - Energy transferred: {:.2e} J", energy_transferred);
        println!("   - Pairs processed: {}", pairs_processed);
        println!("   - Transactions created: {}", energy_deltas.len());
    }
}
*/
