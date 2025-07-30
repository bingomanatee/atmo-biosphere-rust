/// Simple energy and mass data structure for geological cells
#[derive(Debug, Clone)]
pub struct EnergyMass {
    energy_joules: f64,
    mass_kg: f64,
}

impl EnergyMass {
    /// Create new EnergyMass
    pub fn new(energy_joules: f64, mass_kg: f64) -> Self {
        Self {
            energy_joules,
            mass_kg,
        }
    }

    /// Get energy in joules
    pub fn energy_joules(&self) -> f64 {
        self.energy_joules
    }

    /// Get mass in kg
    pub fn mass_kg(&self) -> f64 {
        self.mass_kg
    }

    /// Set energy in joules
    pub fn set_energy_joules(&mut self, energy_joules: f64) {
        self.energy_joules = energy_joules;
    }

    /// Add energy in joules
    pub fn add_energy_joules(&mut self, energy_joules: f64) {
        self.energy_joules += energy_joules;
    }

    /// Remove energy in joules
    pub fn remove_energy_joules(&mut self, energy_joules: f64) {
        self.energy_joules -= energy_joules;
    }

    /// Set mass in kg
    pub fn set_mass_kg(&mut self, mass_kg: f64) {
        self.mass_kg = mass_kg;
    }

    /// Add mass in kg
    pub fn add_mass_kg(&mut self, mass_kg: f64) {
        self.mass_kg += mass_kg;
    }

    /// Remove mass in kg
    pub fn remove_mass_kg(&mut self, mass_kg: f64) {
        self.mass_kg -= mass_kg;
    }
}