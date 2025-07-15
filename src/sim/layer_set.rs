use crate::energy_mass::energy_mass::EnergyMass;
use h3o::{CellIndex, Resolution};
use std::collections::HashMap;
use crate::sim::energy_mass_cell::EnergyMassCell;
use crate::utils::h3_utils::H3Utils;

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
    pub column_count: usize
}

impl LayerSet {
    fn new(params: LayerSetParams) -> Self {
        let mut layers = HashMap::new();
        for (cel_id, _) in H3Utils:: iter_cells_with_base(params.resolution) {
            let cells: Vec<EnergyMassCell> = Vec::new();
            for index in 0..params.column_count {
                let _height = params.start_height_km + index as f64 * params.cell_height_km;
                
            }
            layers.insert(
                cel_id,
                Column {
                    cell_index: cel_id,
                    cells,
                    start_height_km: params.start_height_km,
                }
            );
        }
        LayerSet {
            layers,
            resolution: params.resolution,
            start_height_km: params.start_height_km,
        }
    }
}   