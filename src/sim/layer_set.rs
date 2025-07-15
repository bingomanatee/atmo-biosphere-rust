use crate::energy_mass::energy_mass::EnergyMass;
use h3o::{CellIndex, Resolution};
use std::collections::HashMap;
use crate::material::MaterialPhases;
use crate::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
use crate::utils::h3_utils::H3Utils;
use rayon::prelude::*;

struct Column {
    cell_index: CellIndex,
    cells: Vec<EnergyMassCell>,
    start_height_km: f64,
}

struct LayerSet {
    layers: HashMap<CellIndex, Column>,
    resolution: Resolution,
    start_height_km: f64,
}

pub struct LayerSetParams {
    pub resolution: Resolution,
    pub start_height_km: f64,
    pub cell_height_km: f64,
    pub material_name: String,
    pub column_count: usize,
    pub planet_radius_km: f64,
}

impl LayerSet {
    fn new(params: LayerSetParams) -> Self {
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
                        let height = params.start_height_km + index as f64 * params.cell_height_km;
                        EnergyMassCell::new(EnergyMassCellProps {
                            cell_index: cel_id,
                            temperature_kelvin: 273.15,
                            pressure_pa: 101325.0,
                            height_km: params.cell_height_km,
                            top_km: height,
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
    }   