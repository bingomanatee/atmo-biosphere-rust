use crate::sim::energy_mass_cell_immut::{EnergyMassCellImmut, EnergyMassCellImmutProps};
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
pub struct ImmutableLayerSetParams {
    pub resolution: Resolution,
    pub start_height_km: f64,
    pub cell_height_km: f64,
    pub material_name: String,
    pub column_count: usize,
    pub planet_radius_km: f64,
}

impl LayerSetImmut {
    /// Create a new immutable layer set
    pub fn new(params: ImmutableLayerSetParams) -> Self {
        let mut layers = HashMap::new();
        
        // Generate H3 cells at the specified resolution
        let base_cells = h3o::CellIndex::base_cells();
        let mut h3_cells: Vec<CellIndex> = Vec::new();
        
        for base_cell in base_cells {
            let children: Vec<CellIndex> = base_cell.children(params.resolution).collect();
            h3_cells.extend(children);
        }
        
        println!("Creating immutable layer set with {} H3 cells at resolution {:?}", 
               h3_cells.len(), params.resolution);
        
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
        
        for (h3_cell, column) in &self.layers {
            let mut new_cells = Vec::new();
            
            for cell in &column.cells {
                // Calculate depth within this layer set
                let depth_in_layer_km = cell.top_km - self.start_height_km + cell.height_km / 2.0;
                
                // Calculate temperature: start_temp + gradient * depth_in_layer
                let cell_temperature = start_temperature_k + gradient_k_per_km * depth_in_layer_km;
                
                // Create new cell with correct temperature (immutable pattern)
                let new_cell = cell.with_temperature(cell_temperature);
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
                let total_pressure = crate::constants::REFERENCE_PRESSURE_PA + 
                    running_mass_per_km2 * crate::constants::GRAVITY_M_S2;
                
                // Estimate mass using geological pressure
                let area_km2 = cell.area();
                let estimated_mass_kg = self.estimate_cell_mass_at_pressure(cell, total_pressure);
                
                // Create new cell with updated mass and pressure (immutable pattern)
                let new_cell_with_mass = cell.with_mass(estimated_mass_kg);
                let new_cell_with_pressure = new_cell_with_mass.with_pressure(total_pressure);
                new_cells.push(new_cell_with_pressure);
                
                // Add this cell's mass to running total for cells below
                running_mass_per_km2 += estimated_mass_kg / (area_km2 * 1e6); // Convert to kg/m²
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
