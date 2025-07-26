use crate::binary_pairing::{BinaryPairListener, BinaryPair, BinaryPairType};
use crate::transaction_manager_simple::SimpleTransactionManager;
use crate::energy_mass::energy_mass::EnergyMass;

/// Thread-safe Radiative Transfer Listener
#[derive(Debug, Clone)]
pub struct ThreadSafeRadiativeTransferListener {
    thermal_conductivity: f64,
    total_energy_transferred: f64,
    total_pairs_processed: u64,
}

impl ThreadSafeRadiativeTransferListener {
    pub fn new() -> Self {
        Self {
            thermal_conductivity: 2.5,
            total_energy_transferred: 0.0,
            total_pairs_processed: 0,
        }
    }
    
    fn calculate_heat_transfer(&self, temp1: f64, temp2: f64, distance: f64, contact_area: f64, time_step_years: f64) -> f64 {
        let temp_difference = temp1 - temp2;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let time_step_seconds = time_step_years * seconds_per_year;
        
        self.thermal_conductivity * contact_area * temp_difference / distance * time_step_seconds
    }
}

// Make it Send for thread safety
unsafe impl Send for ThreadSafeRadiativeTransferListener {}

impl BinaryPairListener for ThreadSafeRadiativeTransferListener {
    fn on_binary_pair(&mut self, pair: &BinaryPair, transaction_manager: &mut SimpleTransactionManager, _step: i64, _year: i64) {
        self.total_pairs_processed += 1;
        
        match pair.pair_type {
            BinaryPairType::HorizontalNeighbors | BinaryPairType::VerticalNeighbors => {
                if let Some(cell_b) = &pair.cell_b {
                    let temp_a = pair.cell_a.cell.temperature_kelvin();
                    let temp_b = cell_b.cell.temperature_kelvin();
                    
                    let heat_transfer = self.calculate_heat_transfer(
                        temp_a, temp_b, pair.distance_m, pair.contact_area_m2, 1000.0
                    );
                    
                    if heat_transfer.abs() > 1e15 {
                        transaction_manager.add_energy_delta(pair.cell_a.location, -heat_transfer, "radiative_transfer");
                        transaction_manager.add_energy_delta(cell_b.location, heat_transfer, "radiative_transfer");
                        self.total_energy_transferred += heat_transfer.abs();
                    }
                }
            }
            BinaryPairType::SurfaceToSpace => {
                let surface_temp = pair.cell_a.cell.temperature_kelvin();
                let stefan_boltzmann = 5.670374419e-8;
                let emissivity = 0.95;
                let space_temp = 2.7_f64;
                
                let radiated_power = stefan_boltzmann * emissivity * (surface_temp.powi(4) - space_temp.powi(4));
                let energy_loss = radiated_power * pair.contact_area_m2 * 1000.0 * 365.25 * 24.0 * 3600.0;
                
                if energy_loss > 1e15 {
                    transaction_manager.add_energy_delta(pair.cell_a.location, -energy_loss, "surface_radiation");
                    self.total_energy_transferred += energy_loss;
                }
            }
            BinaryPairType::Custom(_) => {}
        }
    }
    
    fn interested_pair_types(&self) -> Vec<BinaryPairType> {
        vec![BinaryPairType::HorizontalNeighbors, BinaryPairType::VerticalNeighbors, BinaryPairType::SurfaceToSpace]
    }
    
    fn component_key(&self) -> &'static str {
        "ThreadSafeRadiativeTransferListener"
    }
}

/// Thread-safe Core Heat Listener
#[derive(Debug, Clone)]
pub struct ThreadSafeCoreHeatListener {
    earth_wattage_tw: f64,
    hotspot_count: usize,
    perlin_variation: f64,
    total_energy_added: f64,
    total_pairs_processed: u64,
}

impl ThreadSafeCoreHeatListener {
    pub fn new() -> Self {
        Self {
            earth_wattage_tw: 47.0,
            hotspot_count: 10,
            perlin_variation: 0.15,
            total_energy_added: 0.0,
            total_pairs_processed: 0,
        }
    }
    
    fn calculate_base_energy_input(&self, total_cells: usize, years_per_step: f64) -> f64 {
        let total_watts = self.earth_wattage_tw * 1e12;
        let watts_per_cell = total_watts / total_cells as f64;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        watts_per_cell * years_per_step * seconds_per_year
    }
    
    fn generate_perlin_variation(&self, h3_cell: u64, cell_index: usize, step: i64) -> f64 {
        let x = (h3_cell & 0xFFFF) as f64 / 65535.0;
        let y = ((h3_cell >> 16) & 0xFFFF) as f64 / 65535.0;
        let z = cell_index as f64 / 10.0;
        let t = step as f64 / 1000.0;
        
        let noise = ((x * 12.9898 + y * 78.233 + z * 37.719 + t * 17.139).sin() * 43758.5453).fract();
        let centered_noise = (noise - 0.5) * 2.0;
        centered_noise * self.perlin_variation
    }
    
    fn is_hotspot(&self, h3_cell: u64, cell_index: usize) -> bool {
        let hotspot_hash = (h3_cell.wrapping_mul(31) + cell_index as u64) % 1000;
        hotspot_hash < (self.hotspot_count * 1000 / 150) as u64
    }
}

unsafe impl Send for ThreadSafeCoreHeatListener {}

impl BinaryPairListener for ThreadSafeCoreHeatListener {
    fn on_binary_pair(&mut self, pair: &BinaryPair, transaction_manager: &mut SimpleTransactionManager, step: i64, _year: i64) {
        match pair.pair_type {
            BinaryPairType::HorizontalNeighbors | BinaryPairType::VerticalNeighbors => {
                self.total_pairs_processed += 1;
                
                if pair.cell_a.depth_km > 10.0 {
                    let h3_cell = u64::from(pair.cell_a.location.h3_cell);
                    let cell_index = pair.cell_a.location.cell_index;
                    
                    let base_energy = self.calculate_base_energy_input(1500, 1000.0);
                    let perlin_factor = 1.0 + self.generate_perlin_variation(h3_cell, cell_index, step);
                    let energy_input = base_energy * perlin_factor;
                    
                    transaction_manager.add_energy_delta(pair.cell_a.location, energy_input, "core_heat_perlin");
                    self.total_energy_added += energy_input;
                    
                    if self.is_hotspot(h3_cell, cell_index) {
                        let hotspot_energy = base_energy * 5.0;
                        transaction_manager.add_energy_delta(pair.cell_a.location, hotspot_energy, "core_heat_hotspot");
                        self.total_energy_added += hotspot_energy;
                    }
                }
                
                if let Some(cell_b) = &pair.cell_b {
                    if cell_b.depth_km > 10.0 {
                        let h3_cell = u64::from(cell_b.location.h3_cell);
                        let cell_index = cell_b.location.cell_index;
                        
                        let base_energy = self.calculate_base_energy_input(1500, 1000.0);
                        let perlin_factor = 1.0 + self.generate_perlin_variation(h3_cell, cell_index, step);
                        let energy_input = base_energy * perlin_factor;
                        
                        transaction_manager.add_energy_delta(cell_b.location, energy_input, "core_heat_perlin");
                        self.total_energy_added += energy_input;
                        
                        if self.is_hotspot(h3_cell, cell_index) {
                            let hotspot_energy = base_energy * 5.0;
                            transaction_manager.add_energy_delta(cell_b.location, hotspot_energy, "core_heat_hotspot");
                            self.total_energy_added += hotspot_energy;
                        }
                    }
                }
            }
            BinaryPairType::SurfaceToSpace => {}
            BinaryPairType::Custom(_) => {}
        }
    }
    
    fn interested_pair_types(&self) -> Vec<BinaryPairType> {
        vec![BinaryPairType::HorizontalNeighbors, BinaryPairType::VerticalNeighbors]
    }
    
    fn component_key(&self) -> &'static str {
        "ThreadSafeCoreHeatListener"
    }
}

/// Create thread-safe listeners for parallel processing
pub fn create_thread_safe_listeners() -> Vec<Box<dyn BinaryPairListener + Send>> {
    vec![
        Box::new(ThreadSafeRadiativeTransferListener::new()),
        Box::new(ThreadSafeCoreHeatListener::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_pairing::BinaryPairCell;
    use crate::transaction_manager::CellLocation;
    use crate::sim_immut::energy_mass_cell_immut::EnergyMassCellImmut;
    use crate::material::MaterialPhases;
    use h3o::CellIndex;
    
    #[test]
    fn test_thread_safe_listeners() {
        println!("🧵 Testing Thread-Safe Listeners");
        
        let listeners = create_thread_safe_listeners();
        assert_eq!(listeners.len(), 2);
        
        println!("✅ Created {} thread-safe listeners", listeners.len());
        
        // Test that they implement Send
        fn assert_send<T: Send>() {}
        assert_send::<ThreadSafeRadiativeTransferListener>();
        assert_send::<ThreadSafeCoreHeatListener>();
        
        println!("✅ All listeners are Send (thread-safe)");
    }
}
