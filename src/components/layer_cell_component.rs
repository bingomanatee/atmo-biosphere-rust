use crate::simulation::Component;
use crate::collections::Actor;
use crate::cell_location::CellLocation;
use crate::material::materials_loader::MaterialsLoader;
use crate::material::material::MaterialPhases;
use std::sync::Arc;

/// Layer Cell Component - initializes cells with proper geological properties
/// Based on the deprecated LayerSetImmut and EnergyMassCellImmut initialization
pub struct LayerCellComponent {
    pub surface_temp_k: f64,
    pub initialized: bool,
}

impl LayerCellComponent {
    pub fn new() -> Self {
        Self {
            surface_temp_k: 288.15, // 15°C surface temperature
            initialized: false,
        }
    }
    
    pub fn with_surface_temperature(surface_temp_k: f64) -> Self {
        Self {
            surface_temp_k,
            initialized: false,
        }
    }
    
    /// Calculate temperature at depth using geological gradient
    fn calculate_temperature_at_depth(&self, depth_km: f64, layer_index: usize) -> f64 {
        // Geological temperature gradients (K/km) based on layer
        let gradient_k_per_km = match layer_index {
            0 => 25.0,  // Crust: 25K/km (high gradient)
            1 => 15.0,  // Upper mantle: 15K/km (moderate)
            2 => 10.0,  // Lower mantle: 10K/km (low)
            _ => 5.0,   // Deep: 5K/km (very low)
        };
        
        self.surface_temp_k + (depth_km * gradient_k_per_km)
    }
    
    /// Calculate pressure at depth using geological pressure
    fn calculate_pressure_at_depth(&self, depth_km: f64) -> f64 {
        // Geological pressure: ~27 MPa per km depth (average rock density ~2700 kg/m³)
        let surface_pressure_pa = 101325.0; // 1 atmosphere
        let pressure_gradient_pa_per_km = 27_000_000.0; // 27 MPa/km
        
        surface_pressure_pa + (depth_km * pressure_gradient_pa_per_km)
    }
    
    /// Calculate realistic density based on material, temperature, and pressure
    fn calculate_density(&self, material_name: &str, temperature_k: f64, pressure_pa: f64) -> f64 {
        // Get material properties (assume solid phase for geological materials)
        if let Ok(material) = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid) {
            // Use material's reference density and adjust for temperature/pressure
            let base_density = material.density_kg_m3 as f64;

            // Temperature effect: density decreases with temperature (thermal expansion)
            let temp_factor = 1.0 - (temperature_k - 273.15) / 10000.0; // ~0.01% per K

            // Pressure effect: density increases with pressure (compression)
            // Realistic compression: ~0.1% per 100 MPa for rocks
            let pressure_factor = 1.0 + (pressure_pa - 101325.0) / 10_000_000_000.0; // ~0.1% per 100 MPa

            (base_density * temp_factor * pressure_factor).max(1000.0) // Minimum 1000 kg/m³
        } else {
            // Fallback to typical rock density
            2500.0
        }
    }
    
    /// Calculate energy from mass, temperature, and specific heat capacity
    fn calculate_energy(&self, mass_kg: f64, temperature_k: f64, material_name: &str) -> f64 {
        if let Ok(material) = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid) {
            // E = m * c * T
            mass_kg * material.specific_heat_capacity_j_per_kg_k as f64 * temperature_k
        } else {
            // Fallback: typical rock specific heat ~1000 J/kg/K
            mass_kg * 1000.0 * temperature_k
        }
    }
    
    /// Calculate cell volume in m³ from H3 cell area and height
    fn calculate_volume_m3(&self, location: &CellLocation, height_km: f64) -> f64 {
        // Get H3 cell area (this is a simplified calculation)
        // In reality, would use proper H3 area calculation
        let h3_cell = location.h3_cell_index();
        let resolution = h3_cell.resolution();
        
        // Approximate area based on H3 resolution (km²)
        let area_km2 = match resolution {
            h3o::Resolution::Zero => 4_250_000.0,
            h3o::Resolution::One => 607_000.0,
            h3o::Resolution::Two => 86_700.0,
            h3o::Resolution::Three => 12_400.0,
            h3o::Resolution::Four => 1_770.0,
            h3o::Resolution::Five => 253.0,
            h3o::Resolution::Six => 36.1,
            h3o::Resolution::Seven => 5.16,
            _ => 1.0, // Fallback
        };
        
        // Convert to m³: area_km² * height_km * 1e9 (km² to m², km to m)
        area_km2 * height_km * 1_000_000_000.0
    }
    
    /// Get material name for layer
    fn get_material_for_layer(&self, layer_index: usize) -> &'static str {
        match layer_index {
            0 => "granite",  // Crust
            1 => "basalt",   // Upper mantle
            2 => "peridotite", // Lower mantle
            _ => "iron",     // Core
        }
    }
}

impl Component for LayerCellComponent {
    fn name(&self) -> &'static str {
        "LayerCellComponent"
    }
    
    fn initialize(&mut self, sim: &mut crate::simulation::Simulation) {
        println!("🌍 Layer Cell Component initializing geological properties...");
        println!("   - Surface temperature: {:.1}K ({:.1}°C)", 
                 self.surface_temp_k, self.surface_temp_k - 273.15);
        
        let cells = sim.get_geological_cells();
        let total_cells = cells.len();
        println!("   - Total cells to initialize: {}", total_cells);
        
        // We'll set the initialized flag and do the actual work in the first step
        self.initialized = false;
        
        println!("✅ Layer Cell Component ready to initialize {} cells", total_cells);
    }
    
    fn step(&self, coll_mgr: &crate::collections::CollectionsManager, actor: &mut Actor, step: u32, _year: f64) {
        // Only initialize on the first step
        if step == 1 && !self.initialized {
            println!("🔧 LayerCellComponent: Initializing geological properties for all cells...");
            
            let cells = coll_mgr.get::<crate::cell_location::CellLocation, crate::simulation::GeologicalCellData>("geological_cells")
                .expect("geological_cells collection should exist");
            let mut cells_processed = 0;
            
            for entry in cells.iter() {
                let (location, _current_data) = (entry.key(), entry.value());
                
                // Calculate depth from surface using actual layer configuration
                let (depth_km, step_height_km) = match location.layer_set_index() {
                    0 => { // Continental Crust: 5km per step, 4 steps = 20km total
                        let step_height = 5.0;
                        let depth = location.depth_index() as f64 * step_height;
                        (depth, step_height)
                    },
                    1 => { // Upper Mantle: 25km per step, 6 steps = 150km total, starts at 20km
                        let step_height = 25.0;
                        let depth = 20.0 + (location.depth_index() as f64 * step_height);
                        (depth, step_height)
                    },
                    2 => { // Lower Mantle: 50km per step, 3 steps = 150km total, starts at 170km
                        let step_height = 50.0;
                        let depth = 170.0 + (location.depth_index() as f64 * step_height);
                        (depth, step_height)
                    },
                    _ => {
                        let step_height = 10.0;
                        let depth = location.depth_index() as f64 * step_height;
                        (depth, step_height)
                    }
                };
                
                // Calculate geological properties
                let temperature_k = self.calculate_temperature_at_depth(depth_km, location.layer_set_index());
                let pressure_pa = self.calculate_pressure_at_depth(depth_km);
                let material_name = self.get_material_for_layer(location.layer_set_index());
                let density_kg_m3 = self.calculate_density(material_name, temperature_k, pressure_pa);
                
                // Calculate volume and mass
                let volume_m3 = self.calculate_volume_m3(location, step_height_km);
                let mass_kg = density_kg_m3 * volume_m3;
                
                // Calculate energy from temperature and mass
                let energy_joules = self.calculate_energy(mass_kg, temperature_k, material_name);
                
                // Update cell properties using actor
                actor.replace("geological_cells", *location, "temperature_k", temperature_k);
                actor.replace("geological_cells", *location, "pressure_pa", pressure_pa);
                actor.replace("geological_cells", *location, "density_kg_m3", density_kg_m3);
                actor.replace("geological_cells", *location, "energy_joules", energy_joules);
                actor.replace("geological_cells", *location, "mass_kg", mass_kg);

                cells_processed += 1;

                // Debug output for first few cells
                if cells_processed <= 3 {
                    println!("    Cell {}: Layer[{}] Depth[{}] → Temp[{:.1}K] Pressure[{:.1}MPa] Density[{:.0}kg/m³] Depth[{:.1}km]",
                             cells_processed, location.layer_set_index(), location.depth_index(),
                             temperature_k, pressure_pa / 1e6, density_kg_m3, depth_km);
                }
                
                // Progress reporting for large numbers of cells
                if cells_processed % 100000 == 0 {
                    println!("    Processed {} cells...", cells_processed);
                }
            }
            
            println!("✅ LayerCellComponent: Initialized {} cells with geological properties", cells_processed);
            
            // Mark as initialized (note: this is a bit of a hack since we can't mutate self in step)
            // In a real implementation, we'd track this state differently
        }
    }
    
    fn complete(&mut self, sim: &crate::simulation::Simulation) {
        println!("🌍 Layer Cell Component completed");
        
        // Show some statistics
        let cells = sim.get_geological_cells();
        let mut temp_sum = 0.0;
        let mut pressure_sum = 0.0;
        let mut count = 0;
        
        for entry in cells.iter() {
            let data = entry.value();
            temp_sum += data.temperature_k;
            pressure_sum += data.pressure_pa;
            count += 1;
            
            if count >= 1000 { break; } // Sample first 1000 cells
        }
        
        if count > 0 {
            println!("   - Average temperature (sample): {:.1}K ({:.1}°C)", 
                     temp_sum / count as f64, (temp_sum / count as f64) - 273.15);
            println!("   - Average pressure (sample): {:.1} MPa", 
                     (pressure_sum / count as f64) / 1_000_000.0);
        }
    }
}
