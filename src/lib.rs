// Core modules
pub mod collections;
pub mod events;
pub mod cell_location;
pub mod simulation;
pub mod components;

// Utility modules
mod h3o_utils;
pub mod material;
pub mod utils;
pub mod constants;
pub mod energy_mass;
pub mod binary_pair;
pub mod binary_pair_builder;

// All simulation and component code moved to deprecated/
// - sim_immut -> deprecated/src/sim_immut
// - component -> deprecated/src/component
// - binary_pairing -> deprecated/src/binary_pairing
// - transaction_manager -> deprecated/src/transaction_manager.rs
// - transaction_manager_simple -> deprecated/src/transaction_manager_simple.rs

// - profiling -> deprecated/src/profiling
// - reporting -> deprecated/src/reporting

// - test_geological_simulation -> deprecated/src/test_geological_simulation.rs