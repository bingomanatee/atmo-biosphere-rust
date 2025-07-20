pub mod sim;
// pub mod immutable;  // TODO: Fix compilation issues
pub mod component;
pub mod events;

#[cfg(test)]
mod test_geological_simulation;

mod h3o_utils;
pub mod energy_mass;
pub mod material;
pub mod utils;
pub mod constants;
pub mod profiling;