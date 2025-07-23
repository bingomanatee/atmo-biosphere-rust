use std::sync::Arc;
use crate::material::material::MaterialPhase;

pub trait EnergyMass {
    fn energy_joules(&self) -> f64;
    fn mass_kg(&self) -> f64;
    fn volume_km3(&self) -> f64;
    fn material(&self) -> Arc<MaterialPhase>;
    fn temperature_kelvin(&self) -> f64;
    fn pressure_pa(&self) -> f64;

    fn set_pressure_pa(&mut self, pressure_pa: f64);
    fn set_energy_joules(&mut self, energy_joules: f64);
    fn set_temperature_kelvin(&mut self, temperature_kelvin: f64);

    fn add_energy_joules(&mut self, energy_joules: f64);
    fn remove_energy_joules(&mut self, energy_joules: f64);
}