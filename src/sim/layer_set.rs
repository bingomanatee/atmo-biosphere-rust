use crate::energy_mass::energy_mass::EnergyMass;
use h3o::{CellIndex, Resolution};
use std::collections::HashMap;
use crate::material::MaterialPhases;
use crate::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use crate::utils::h3_utils::H3Utils;
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
}

#[derive(Clone)]
pub struct LayerSetParams {
    pub resolution: Resolution,
    pub start_height_km: f64,
    pub cell_height_km: f64,
    pub material_name: String,
    pub column_count: usize,
    pub planet_radius_km: f64,
}

impl LayerSet {
    pub fn new(params: &LayerSetParams) -> Self {
        // Use default thermal configuration for backward compatibility
        let default_thermal_config = crate::sim::simulation::ThermalGradientConfig::earth_like(288.15);
        Self::new_with_thermal_config(params, &default_thermal_config)
    }

    pub fn new_with_thermal_config(
        params: &LayerSetParams,
        thermal_config: &crate::sim::simulation::ThermalGradientConfig,
    ) -> Self {
        // Collect all cell IDs first to enable parallelization
        let cell_ids: Vec<CellIndex> = H3Utils::iter_cells_with_base(params.resolution)
            .map(|(cel_id, _)| cel_id)
            .collect();

        // Create columns in parallel
        let layers: HashMap<CellIndex, Column> = cell_ids
            .par_iter()
            .map(|&cel_id| {
                // Create cells within each column in parallel
                let cells: Vec<EnergyMassCell> = (0..params.column_count)
                    .into_par_iter()
                    .map(|index| {
                        let top_km = params.start_height_km + index as f64 * params.cell_height_km;
                        let cell_center_depth_km = top_km + params.cell_height_km / 2.0;

                        // Calculate temperature based on depth using quadratic thermal gradient
                        let temperature_kelvin = thermal_config.calculate_temperature_at_depth(cell_center_depth_km);

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

    /// Adjust pressures in all cells to account for accumulated mass from layers above
    pub fn adjust_pressures_for_accumulated_mass(&mut self, accumulated_mass_per_km2: f64) {
        // Standard gravity acceleration (m/s²)
        const GRAVITY_M_S2: f64 = 9.81;

        // Convert accumulated mass per km² to pressure (Pa)
        // Pressure = mass_per_m² * gravity
        // mass_per_km² = mass_per_m² * 1e6, so mass_per_m² = mass_per_km² / 1e6
        let base_pressure_from_above = (accumulated_mass_per_km2 / 1e6) * GRAVITY_M_S2;

        for column in self.layers.values_mut() {
            let mut column_accumulated_mass_per_km2 = accumulated_mass_per_km2;

            // Process cells from top to bottom within this column
            for (cell_index, cell) in column.cells.iter_mut().enumerate() {
                // Calculate pressure from all mass above this cell
                let pressure_from_above = (column_accumulated_mass_per_km2 / 1e6) * GRAVITY_M_S2;

                // Add atmospheric pressure at surface
                let atmospheric_pressure = 101325.0; // Pa
                let total_pressure = atmospheric_pressure + pressure_from_above;

                // Update cell pressure
                cell.set_pressure_pa(total_pressure);

                // Add this cell's mass to the accumulation for cells below
                let area_km2 = cell.area();
                let cell_mass_per_km2 = cell.mass_kg() / area_km2;
                column_accumulated_mass_per_km2 += cell_mass_per_km2;
            }
        }
    }
}