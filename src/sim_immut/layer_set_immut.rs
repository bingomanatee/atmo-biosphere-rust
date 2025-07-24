use crate::sim_immut::energy_mass_cell_immut::{EnergyMassCellImmut, EnergyMassCellImmutProps};
use crate::energy_mass::energy_mass::EnergyMass;
use h3o::{CellIndex, Resolution};
use std::collections::HashMap;

/// Immutable Layer Set - uses immutable energy/mass cells for better performance
#[derive(Debug, Clone)]
pub struct LayerSetImmut {
    pub layers: HashMap<CellIndex, ColumnImmut>,
    pub start_height_km: f64,
    pub resolution: Resolution,
}

/// Column of immutable cells
#[derive(Debug, Clone)]
pub struct ColumnImmut {
    pub cells: Vec<EnergyMassCellImmut>,
}

/// Parameters for creating an immutable layer set
#[derive(Debug, Clone)]
pub struct LayerSetParamsImmut {
    pub resolution: Resolution,
    pub start_height_km: f64,
    pub cell_height_km: f64,
    pub material_name: String,
    pub column_count: usize,
    pub planet_radius_km: f64,
    pub thermal_gradient_k_per_km: f64,
}

impl LayerSetImmut {
    /// Create a new immutable layer set
    pub fn new(params: LayerSetParamsImmut) -> Self {
        let mut layers = HashMap::new();
        
        // Generate H3 cells at the specified resolution
        let base_cells = h3o::CellIndex::base_cells();
        let mut h3_cells: Vec<CellIndex> = Vec::new();
        
        for base_cell in base_cells {
            let children: Vec<CellIndex> = base_cell.children(params.resolution).collect();
            h3_cells.extend(children);
        }
        
        // Create columns for each H3 cell
        for cel_id in h3_cells.iter().take(50) { // Limit for testing
            let mut cells = Vec::new();
            
            // Create cells in this column
            for index in 0..params.column_count {
                let top_km = params.start_height_km + index as f64 * params.cell_height_km;
                
                // Use default temperature - will be set properly in thermal gradient pass
                let temperature_kelvin = 300.0; // Default temperature, will be overridden
                
                let cell = EnergyMassCellImmut::new(EnergyMassCellImmutProps {
                    cell_index: *cel_id,
                    height_km: params.cell_height_km,
                    top_km,
                    material_name: params.material_name.clone(),
                    temperature_kelvin,
                    pressure_pa: 1e5, // Default 1 atmosphere, will be adjusted
                    planet_radius_km: params.planet_radius_km,
                });
                
                cells.push(cell);
            }
            
            layers.insert(*cel_id, ColumnImmut { cells });
        }
        
        LayerSetImmut {
            layers,
            start_height_km: params.start_height_km,
            resolution: params.resolution,
        }
    }
    
    /// Apply thermal gradient to all cells in this layer set (immutable pattern)
    pub fn with_thermal_gradient(&self, start_temperature_k: f64, gradient_k_per_km: f64) -> Self {
        let mut new_layers = HashMap::new();
        let mut cells_processed = 0;

        for (h3_cell, column) in &self.layers {
            let mut new_cells = Vec::new();

            for (_cell_idx, cell) in column.cells.iter().enumerate() {
                // Calculate depth within this layer set
                let depth_in_layer_km = cell.top_km - self.start_height_km + cell.height_km / 2.0;

                // Calculate temperature: start_temp + gradient * depth_in_layer
                let cell_temperature = start_temperature_k + gradient_k_per_km * depth_in_layer_km;

                // Create new cell with correct temperature (immutable pattern)
                let new_cell = cell.with_temperature(cell_temperature);
                new_cells.push(new_cell);
                cells_processed += 1;
            }
            
            new_layers.insert(*h3_cell, ColumnImmut { cells: new_cells });
        }

        // Thermal gradient applied to all cells in layer set

        LayerSetImmut {
            layers: new_layers,
            start_height_km: self.start_height_km,
            resolution: self.resolution,
        }
    }

    /// Final mass adjustment to fill area based on pressure and temperature (initialization only)
    pub fn with_final_mass_adjustment(&self) -> Self {
        let mut new_layers = HashMap::new();

        for (h3_cell, column) in &self.layers {
            let mut new_cells = Vec::new();

            for cell in &column.cells {
                // Recalculate mass to properly fill area based on final pressure and temperature
                let new_cell = cell.recalculate_mass_to_fill_area();
                new_cells.push(new_cell);
            }

            new_layers.insert(*h3_cell, ColumnImmut { cells: new_cells });
        }

        LayerSetImmut {
            layers: new_layers,
            start_height_km: self.start_height_km,
            resolution: self.resolution,
        }
    }
    
    /// Apply pressure adjustments to all cells (immutable pattern)
    pub fn with_pressure_adjustments(&self, accumulated_mass_per_km2: f64) -> Self {
        let mut new_layers = HashMap::new();
        
        for (h3_cell, column) in &self.layers {
            let mut new_cells = Vec::new();
            let mut running_mass_per_km2 = accumulated_mass_per_km2;
            
            for cell in &column.cells {
                // Calculate pressure from accumulated mass above
                // Convert mass per km² to mass per m² for correct pressure calculation
                let mass_per_m2 = running_mass_per_km2 / 1e6; // 1 km² = 1e6 m²
                let total_pressure = crate::constants::REFERENCE_PRESSURE_PA +
                    mass_per_m2 * crate::constants::GRAVITY_M_S2;
                
                // Estimate mass using geological pressure
                let area_km2 = cell.area();
                let estimated_mass_kg = self.estimate_cell_mass_at_pressure(cell, total_pressure);
                
                // Create new cell with updated mass and pressure (immutable pattern)
                let new_cell_with_mass = cell.with_mass(estimated_mass_kg);
                let new_cell_with_pressure = new_cell_with_mass.with_pressure(total_pressure);
                new_cells.push(new_cell_with_pressure);
                
                // Add this cell's mass per km² to running total for cells below
                running_mass_per_km2 += estimated_mass_kg / area_km2; // Keep in kg/km²
            }
            
            new_layers.insert(*h3_cell, ColumnImmut { cells: new_cells });
        }
        
        LayerSetImmut {
            layers: new_layers,
            start_height_km: self.start_height_km,
            resolution: self.resolution,
        }
    }
    
    /// Estimate cell mass at given pressure (helper method)
    fn estimate_cell_mass_at_pressure(&self, cell: &EnergyMassCellImmut, pressure_pa: f64) -> f64 {
        use crate::material::material::MassCalculationParams;
        use crate::energy_mass::energy_mass::EnergyMass;
        
        let volume_km3 = cell.volume_km3();
        let temperature_k = cell.temperature_kelvin();
        
        // If temperature is very low (clamped), use estimated temperature from depth
        let safe_temperature_k = if temperature_k <= 1.0 {
            // Estimate temperature from depth using simple gradient
            let depth_km = cell.top_km + cell.height_km / 2.0;
            288.15 + depth_km * 25.0 // Simple 25K/km gradient
        } else {
            temperature_k
        };
        
        // Get material properties for current phase
        let material = cell.material_properties();
        
        // Calculate mass from pressure, volume, and temperature
        material.calculate_mass_from_pressure_volume(MassCalculationParams {
            pressure_pa,
            volume_km3,
            temperature_k: safe_temperature_k,
        })
    }
    
    /// Get total mass per km² for this layer set
    pub fn total_mass_per_km2(&self) -> f64 {
        if let Some((_, column)) = self.layers.iter().next() {
            let mut total_mass_kg = 0.0;
            let mut total_area_km2 = 0.0;
            
            for cell in &column.cells {
                total_mass_kg += cell.mass_kg();
                total_area_km2 += cell.area();
            }
            
            if total_area_km2 > 0.0 {
                total_mass_kg / total_area_km2
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl ColumnImmut {
    /// Get cell at specific depth index
    pub fn get_cell(&self, depth_index: usize) -> Option<&EnergyMassCellImmut> {
        self.cells.get(depth_index)
    }
    
    /// Get mutable cell at specific depth index
    pub fn get_cell_mut(&mut self, depth_index: usize) -> Option<&mut EnergyMassCellImmut> {
        self.cells.get_mut(depth_index)
    }
}


/// Helper function to create default immutable layer set parameters
/// Correct layer structure as discussed: 5+5+5 cells, good aspect ratios, 165km depth
/// Designed for Resolution 3 (~60km cells) with artificial deep radiance
pub fn default_layer_set_params_immut(resolution: h3o::Resolution, planet_radius_km: f64) -> Vec<LayerSetParamsImmut> {
    vec![
        // Surface Layer (0-15km): Surface detail for plate interactions
        LayerSetParamsImmut {
            resolution,
            start_height_km: 0.0,
            cell_height_km: 3.0,  // 3km cells - aspect ratio 3:60 = 1:20
            material_name: "basalt".to_string(),
            column_count: 5,      // 5 cells = 15km (surface detail)
            planet_radius_km,
            thermal_gradient_k_per_km: 25.0, // High gradient in crust
        },
        // Mid Layer (15-65km): Heat transport
        LayerSetParamsImmut {
            resolution,
            start_height_km: 15.0,
            cell_height_km: 10.0, // 10km cells - aspect ratio 10:60 = 1:6
            material_name: "granite".to_string(),
            column_count: 5,      // 5 cells = 50km (heat transport)
            planet_radius_km,
            thermal_gradient_k_per_km: 15.0, // Moderate gradient
        },
        // Deep Layer (65-165km): Background + artificial boundary
        LayerSetParamsImmut {
            resolution,
            start_height_km: 65.0,
            cell_height_km: 20.0, // 20km cells - aspect ratio 20:60 = 1:3
            material_name: "basalt".to_string(),
            column_count: 5,      // 5 cells = 100km (deep background + artificial radiance)
            planet_radius_km,
            thermal_gradient_k_per_km: 10.0, // Low gradient, artificial radiance at bottom
        },
    ]
}

/// Variable resolution layer parameters - efficient with focus on lateral heat diffusion
/// Surface detail for plates, coarse everywhere else for efficiency
pub fn variable_resolution_layer_params_immut(resolution: h3o::Resolution, planet_radius_km: f64) -> Vec<LayerSetParamsImmut> {
    vec![
        // Surface Layer (0-20km): Moderate detail for plate interactions
        LayerSetParamsImmut {
            resolution,
            start_height_km: 0.0,
            cell_height_km: 5.0,  // 5km cells - enough detail for plates, not excessive
            material_name: "basalt".to_string(),
            column_count: 4,      // 4 cells = 20km (crust + upper lithosphere)
            planet_radius_km,
            thermal_gradient_k_per_km: 25.0, // High gradient in crust
        },
        // Upper Mantle (20-80km): Efficient background for lateral heat diffusion
        LayerSetParamsImmut {
            resolution,
            start_height_km: 20.0,
            cell_height_km: 15.0, // 15km cells - efficient for background thermal
            material_name: "granite".to_string(),
            column_count: 4,      // 4 cells = 60km (lithospheric mantle)
            planet_radius_km,
            thermal_gradient_k_per_km: 15.0, // Moderate gradient
        },
        // Deep Mantle (80-180km): Coarse for plume sources and deep background
        LayerSetParamsImmut {
            resolution,
            start_height_km: 80.0,
            cell_height_km: 25.0, // 25km cells - coarse and efficient
            material_name: "basalt".to_string(),
            column_count: 4,      // 4 cells = 100km (deep mantle/plume source)
            planet_radius_km,
            thermal_gradient_k_per_km: 10.0, // Low gradient in deep mantle
        },
    ]
}

/// Helper function to create shallow, detailed layer parameters for full simulation runs
/// Optimized for ~20 cells per column focusing on lateral energy diffusion and surface processes
pub fn coarse_layer_set_params_immut(resolution: h3o::Resolution, planet_radius_km: f64) -> Vec<LayerSetParamsImmut> {
    vec![
        // Surface/Crust (0-40km): Basalt - detailed surface and crustal processes
        LayerSetParamsImmut {
            resolution,
            start_height_km: 0.0,
            cell_height_km: 2.0,  // 2km cells for detailed surface resolution
            material_name: "basalt".to_string(),
            column_count: 12,     // 12 cells = 24km total depth (detailed crust)
            planet_radius_km,
            thermal_gradient_k_per_km: 25.0, // High gradient in crust
        },
        // Upper Mantle (24-64km): Granite - moderate resolution for lithosphere
        LayerSetParamsImmut {
            resolution,
            start_height_km: 24.0,
            cell_height_km: 5.0,  // 5km cells for lithosphere processes
            material_name: "granite".to_string(),
            column_count: 8,      // 8 cells = 40km total depth
            planet_radius_km,
            thermal_gradient_k_per_km: 15.0, // Moderate gradient
        },
    ]
}
