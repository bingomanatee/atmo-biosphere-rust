use h3o::Resolution;
use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::constants::{KM2_TO_M2, KM_TO_M};
use crate::material::material::MaterialPhases;
use crate::material::materials_loader::MaterialsLoader;
use crate::simulation::{Component, GeologicalCellData, Simulation, SimulationConfig};
use crate::utils::h3_utils::H3Utils;

/// Layer Cell Component - initializes cells with proper geological properties
/// Based on the deprecated LayerSetImmut and EnergyMassCellImmut initialization
pub struct LayerCellComponent {
    pub initialized: bool,
}

impl LayerCellComponent {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
    
    /// Calculate temperature at depth by iterating through layer history
    fn calculate_temperature_at_depth(&self, location: &CellLocation, config: &SimulationConfig) -> f64 {
        let mut temperature_k = config.planet.surface_temperature_k;
        let current_layer = location.layer_set_index();
        let current_depth = location.depth_index();

        // Add temperature increase from all layers above current layer
        for layer_idx in 0..current_layer {
            if layer_idx < config.layers.len() {
                let layer_config = &config.layers[layer_idx];
                let layer_thickness_km = layer_config.height_per_step_km * layer_config.depth_steps as f64;
                let temp_increase = layer_thickness_km * layer_config.temperature_gradient_k_per_km;
                temperature_k += temp_increase;
            }
        }

        // Add temperature increase from current layer up to current depth
        if current_layer < config.layers.len() {
            let current_layer_config = &config.layers[current_layer];
            let depth_in_current_layer_km = current_layer_config.height_per_step_km * current_depth as f64;
            let temp_increase = depth_in_current_layer_km * current_layer_config.temperature_gradient_k_per_km;
            temperature_k += temp_increase;
        }

        temperature_k
    }
    
    /// Calculate pressure at depth based on gravity and estimated mass of overlying material
    fn calculate_pressure_at_depth(&self, location: &CellLocation, config: &SimulationConfig) -> f64 {
        let surface_pressure_pa = 101325.0; // Should come from config.planet.surface_pressure_pa
        let gravity_m_s2 = config.planet.surface_gravity_m_s_s;
        let current_layer = location.layer_set_index();
        let current_depth = location.depth_index();

        // Get H3 cell area for this resolution and planet
        let h3_cell = location.h3_cell_index();
        let resolution = h3_cell.resolution();
        let area_m2 = self.get_h3_cell_area_m2(resolution, config);

        let mut total_mass_above_kg = 0.0;

        // Calculate mass from all layers above current layer
        for layer_idx in 0..current_layer {
            if layer_idx < config.layers.len() {
                let layer_config = &config.layers[layer_idx];
                let material_name = self.get_material_for_layer(layer_idx);
                let estimated_density = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid)
                    .map(|material| material.density_kg_m3 as f64)
                    .unwrap_or_else(|_| panic!("Material '{}' not found in materials database", material_name));

                // Mass = area × height × density for entire layer
                let layer_height_m = layer_config.height_per_step_km * layer_config.depth_steps as f64 * 1000.0;
                let layer_volume_m3 = area_m2 * layer_height_m;
                let layer_mass_kg = layer_volume_m3 * estimated_density;

                total_mass_above_kg += layer_mass_kg;
            }
        }

        // Calculate mass from cells above in current layer
        if current_layer < config.layers.len() {
            let current_layer_config = &config.layers[current_layer];
            let material_name = self.get_material_for_layer(current_layer);
            let estimated_density = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid)
                .map(|material| material.density_kg_m3 as f64)
                .unwrap_or_else(|_| panic!("Material '{}' not found in materials database", material_name));

            // Mass from cells above in same layer
            let cells_above_height_m = current_layer_config.height_per_step_km * current_depth as f64 * 1000.0;
            let volume_above_m3 = area_m2 * cells_above_height_m;
            let mass_above_kg = volume_above_m3 * estimated_density;

            total_mass_above_kg += mass_above_kg;
        }

        // Calculate pressure: P = P_surface + (mass * g) / area
        let pressure_from_mass = (total_mass_above_kg * gravity_m_s2) / area_m2;

        surface_pressure_pa + pressure_from_mass
    }

    /// Get cell volume in m³ - consolidates all volume calculation logic
    fn get_volume_m3(&self, location: &CellLocation, config: &SimulationConfig) -> f64 {
        // Get H3 cell area based on resolution and planetary radius
        let resolution = location.h3_cell_index().resolution();
        let area_km2 = H3Utils::cell_area(resolution, config.planet.radius_km);
        let area_m2 = area_km2 * KM2_TO_M2;

        // Get cell height from layer configuration
        let height_km = self.get_cell_height_km(location, config);
        let height_m = height_km * KM_TO_M;

        // Volume = area × height
        area_m2 * height_m
    }

    /// Get H3 cell area in m² using resolution and planetary radius
    fn get_h3_cell_area_m2(&self, resolution: Resolution, config: &SimulationConfig) -> f64 {
        let area_km2 = H3Utils::cell_area(resolution, config.planet.radius_km);
        area_km2 * KM2_TO_M2 // Convert km² to m²
    }

    /// Get the height of a cell in km based on its layer configuration
    fn get_cell_height_km(&self, location: &CellLocation, config: &SimulationConfig) -> f64 {
        let layer_index = location.layer_set_index();
        if layer_index < config.layers.len() {
            config.layers[layer_index].height_per_step_km
        } else {
            10.0 // Fallback height in km
        }
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
    
    fn initialize(&mut self, _coll_mgr: &mut CollectionsManager, _config: &SimulationConfig) {
        // LayerCellComponent initialization is now handled in the first step
        // to avoid borrowing conflicts with the simulation collections
        self.initialized = false; // Will be set to true after first step initialization
    }

    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, _year: f64, config: &SimulationConfig) {
        // Initialize cells on the first step to avoid borrowing conflicts
        if step == 1 && !self.initialized {
            let cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
                .expect("geological_cells collection should exist");

            for entry in cells.iter() {
                let (location, _current_data) = (entry.key(), entry.value());

                // Calculate initial geological properties based on depth and layer config
                let temperature_k = self.calculate_temperature_at_depth(location, config);
                let pressure_pa = self.calculate_pressure_at_depth(location, config);
                let material_name = self.get_material_for_layer(location.layer_set_index());
                let density_kg_m3 = self.calculate_density(material_name, temperature_k, pressure_pa);

                // Calculate volume and mass
                let volume_m3 = self.get_volume_m3(location, config);
                let mass_kg = density_kg_m3 * volume_m3;

                // Calculate energy from temperature and mass
                let energy_joules = self.calculate_energy(mass_kg, temperature_k, material_name);

                // Set initial cell properties using actor (field-by-field)
                actor.replace("geological_cells", *location, "temperature_k", temperature_k);
                actor.replace("geological_cells", *location, "pressure_pa", pressure_pa);
                actor.replace("geological_cells", *location, "density_kg_m3", density_kg_m3);
                // Set EnergyMass fields individually since Actor only supports f64
                actor.replace("geological_cells", *location, "energy_joules", energy_joules);
                actor.replace("geological_cells", *location, "mass_kg", mass_kg);
            }

            // Note: We can't set self.initialized = true here because self is not mutable
            // This is a limitation of the current component architecture
        }
    }
    
    fn complete(&mut self, _sim: &Simulation, _config: &SimulationConfig) {
        // Component cleanup - no output needed
    }
}
