pub mod material;
pub mod materials_loader;
pub mod material_utils;

pub use material::{Material, MaterialPhase, MaterialPhases};
pub use materials_loader::{MaterialsLoader, get_phase_properties_by_name};
pub use material_utils::MaterialUtils;