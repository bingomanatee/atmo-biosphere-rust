pub mod sim_immut;
pub mod transaction_manager;
pub mod component;
pub mod events;

// Deprecated modules - kept for component compatibility
#[path = "../deprecated/src/mod.rs"]
pub mod deprecated;

#[cfg(test)]
mod test_geological_simulation;

mod h3o_utils;
pub mod energy_mass;
pub mod material;
pub mod utils;
pub mod constants;
pub mod profiling;