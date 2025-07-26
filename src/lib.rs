pub mod sim_immut;
pub mod transaction_manager;
pub mod transaction_manager_simple;
pub mod binary_pairing;
pub mod component;
pub mod events;

// Deprecated modules moved to deprecated_backup/

// #[cfg(test)]
// mod test_geological_simulation; // Temporarily disabled during atomic transaction refactor

mod h3o_utils;
pub mod energy_mass;
pub mod material;
pub mod utils;
pub mod constants;
pub mod profiling;
pub mod reporting;