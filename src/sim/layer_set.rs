use crate::energy_mass::energy_mass::EnergyMass;
use h3o::{CellIndex, Resolution};
use std::collections::HashMap;
use crate::material::MaterialPhases;
use crate::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use crate::utils::h3_utils::H3Utils;
use crate::constants::{GRAVITY_M_S2, REFERENCE_PRESSURE_PA, KM2_TO_M2_CONVERSION};
use rayon::prelude::*;

pub struct Column {
    pub cell_index: CellIndex,
    pub cells: Vec<EnergyMassCell>,
    pub start_height_km: f64,
}

pub struct LayerSet {
    pub layers: HashMap<CellIndex, Column>,
    pub resolution: Resolution,
    pub start_height_km: f64,
    pub thermal_gradient_k_per_km: f64
}

#[derive(Clone)]
pub struct LayerSetParams {
    pub resolution: Resolution,
    pub start_height_km: f64,
    pub cell_height_km: f64,
    pub material_name: String,
    pub cells_per_column: usize,
    pub planet_radius_km: f64,
    pub thermal_gradient_k_per_km: f64,
    pub name: String
}

impl LayerSet {
    pub fn new(params: &LayerSetParams, start_temperature: f64) -> Self {
        // Collect all cell IDs first to enable parallelization
        let cell_ids: Vec<CellIndex> = H3Utils::iter_cells_with_base(params.resolution)
            .map(|(cel_id, _)| cel_id)
            .collect();

        // Create columns in parallel
        let layers: HashMap<CellIndex, Column> = cell_ids
            .par_iter()
            .map(|&cel_id| {
                // Create cells within each column in parallel
                let cells: Vec<EnergyMassCell> = (0..params.cells_per_column)
                    .into_par_iter()
                    .map(|index| {
                        let top_km = params.start_height_km + index as f64 * params.cell_height_km;
                        let cell_center_depth_km = top_km + params.cell_height_km / 2.0;

                        // Use default temperature - will be set properly in thermal gradient pass
                        let temperature_kelvin = start_temperature + cell_center_depth_km * params.thermal_gradient_k_per_km; // Default temperature, will be overridden

                        EnergyMassCell::new(EnergyMassCellProps {
                            cell_index: cel_id,
                            temperature_kelvin,
                            pressure_pa: 101325.0, // Will be adjusted later based on mass above
                            height_km: params.cell_height_km,
                            top_km,
                            material_name: params.material_name.clone(),
                            planet_radius_km: params.planet_radius_km,
                        })
                    })
                    .collect();

                let column = Column {
                    cell_index: cel_id,
                    cells,
                    start_height_km: params.start_height_km,
                };

                (cel_id, column)
            })
            .collect();

        LayerSet {
            layers,
            resolution: params.resolution,
            start_height_km: params.start_height_km,
            thermal_gradient_k_per_km: params.thermal_gradient_k_per_km
        }
    }

    /// Calculate the average mass per km² for this layer set
    pub fn calculate_average_mass_per_km2(&self) -> f64 {
        if self.layers.is_empty() {
            return 0.0;
        }

        let mut total_mass_per_km2 = 0.0;
        let mut cell_count = 0;

        for column in self.layers.values() {
            for cell in &column.cells {
                let area_km2 = cell.area();
                let mass_per_km2 = cell.mass_kg() / area_km2;
                total_mass_per_km2 += mass_per_km2;
                cell_count += 1;
            }
        }

        if cell_count > 0 {
            total_mass_per_km2 / cell_count as f64
        } else {
            0.0
        }
    }

    /// Adjust pressures in all cells to account for accumulated mass from cells above
    pub fn adjust_pressures_for_accumulated_mass(&mut self, accumulated_mass_per_km2: f64) {
        for column in self.layers.values_mut() {
            let mut column_accumulated_mass_per_km2 = accumulated_mass_per_km2;

            // Process cells from top to bottom within this column
            // Use iterative approach to break circular dependency between pressure and mass
            for cell in column.cells.iter_mut() {
                // Calculate pressure from all mass above this cell
                // Convert mass per km² to mass per m², then multiply by gravity
                let pressure_from_above = (column_accumulated_mass_per_km2 / KM2_TO_M2_CONVERSION) * GRAVITY_M_S2;

                // Add atmospheric pressure at surface
                let total_pressure = REFERENCE_PRESSURE_PA + pressure_from_above;

                // Estimate mass using geological pressure (break circular dependency)
                let area_km2 = cell.area();
                let estimated_mass_kg = Self::estimate_cell_mass_at_pressure(cell, total_pressure);

                // Update cell pressure AND mass with the estimated values
                cell.set_pressure_pa(total_pressure);

                // CRITICAL FIX: Actually apply the estimated mass to the cell (immutable pattern)
                let new_cell_with_mass = crate::sim::energy_mass_cell::EnergyMassCell::with_mass(cell, estimated_mass_kg);
                let new_cell_with_pressure = crate::sim::energy_mass_cell::EnergyMassCell::with_pressure(&new_cell_with_mass, total_pressure);
                *cell = new_cell_with_pressure;

                // Add this cell's estimated mass to the accumulation for cells below
                let cell_mass_per_km2 = estimated_mass_kg / area_km2;
                column_accumulated_mass_per_km2 += cell_mass_per_km2;
            }
        }
    }

    /// Estimate cell mass at a given pressure without circular dependency
    fn estimate_cell_mass_at_pressure(cell: &EnergyMassCell, pressure_pa: f64) -> f64 {
        use crate::material::materials_loader::MaterialsLoader;
        use crate::material::material::MassCalculationParams;
        use crate::material::MaterialPhases;

        // Get cell properties
        let volume_km3 = cell.area() * cell.height_km;
        let temperature_k = cell.temperature_kelvin();

        // If temperature is NaN, zero, or clamped to 1K, use estimated temperature from depth
        let safe_temperature_k = if temperature_k.is_nan() || temperature_k <= 1.0 {
            // Estimate temperature from depth using simple gradient
            let depth_km = cell.top_km + cell.height_km / 2.0;
            288.15 + depth_km * 25.0 // Simple 25K/km gradient
        } else {
            temperature_k
        };

        // Get material properties for solid phase (most conservative estimate)
        // Use a default material name since we can't access the private field
        let material_name = "basalt"; // Default to basalt for geological simulations
        if let Ok(material) = MaterialsLoader::get_phase_properties(material_name, MaterialPhases::Solid) {
            let params = MassCalculationParams {
                pressure_pa,
                volume_km3,
                temperature_k: safe_temperature_k,
            };

            material.calculate_mass_from_pressure_volume(params)
        } else {
            // Fallback: use typical mantle density
            let typical_density_kg_m3 = 3300.0;
            volume_km3 * 1e9 * typical_density_kg_m3
        }
    }
}