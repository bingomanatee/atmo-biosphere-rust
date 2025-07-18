pub mod sim;
pub mod component;
pub mod profiling;

#[cfg(test)]
mod test_pressure_calculation;

#[cfg(test)]
mod test_material_phase_params;

#[cfg(test)]
mod test_energy_banking;

#[cfg(test)]
mod test_pressure_phase_transitions;

#[cfg(test)]
mod test_layer_pressure_calculation;
mod h3o_utils;
pub mod energy_mass;
pub mod material;
mod utils;
mod constants;