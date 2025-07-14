use crate::material::material::MaterialPhase;

pub trait EnergyMass {
    fn energy_joules(&self) -> u64;
    fn mass_kg(&self) -> u64;
    fn volume_km3(&self) -> u64;
    fn material(&self) -> &MaterialPhase;
}