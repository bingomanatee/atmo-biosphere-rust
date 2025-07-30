use crate::collections::{CollectionsManager, Actor, ChangeController};
use crate::cell_location::CellLocation;
use crate::energy_mass::EnergyMass;
use crate::utils::h3_utils::H3Utils;
use h3o::Resolution;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectionName {
    GeologicalCells,
    HotSpots,
}

impl CollectionName {
    /// Convert the enum to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            CollectionName::GeologicalCells => "geological_cells",
            CollectionName::HotSpots => "upwell_hotspots",
        }
    }
}


/// Minimal simulation configuration
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub planet: PlanetConfig,
    pub years_per_step: u32,
    pub steps: u32,
    pub layers: Vec<LayerConfig>,
}

/// Planet configuration
#[derive(Debug, Clone)]
pub struct PlanetConfig {
    pub radius_km: f64,
    pub surface_gravity_m_s_s: f64,
    pub surface_temperature_k: f64,
}

/// Layer configuration
#[derive(Debug, Clone)]
pub struct LayerConfig {
    pub height_per_step_km: f64,
    pub depth_steps: usize,  // Number of depth steps in this layer
    pub resolution: Resolution,
    pub name: String,
    pub temperature_gradient_k_per_km: f64,  // Temperature gradient for this layer (K/km)
}

/// Cell data - closely resembles energy_mass_cell
#[derive(Debug, Clone)]
pub struct GeologicalCellData {
    pub energy_mass: EnergyMass,
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub density_kg_m3: f64,
}



/// Component trait for Actor-based processing with lifecycle phases
pub trait Component: Send + Sync {
    /// Component name/key for debugging and identification
    fn name(&self) -> &'static str;

    /// Initialize the component with the simulation (called once at start)
    fn initialize(&mut self, sim: &mut Simulation, config: &SimulationConfig);

    /// Process one simulation step - add changes to actor
    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, year: f64, config: &SimulationConfig);

    /// Complete/summarize when simulation is done (called once at end)
    fn complete(&mut self, sim: &Simulation, config: &SimulationConfig);
}

/// Main simulation struct
pub struct Simulation {
    pub coll_mgr: CollectionsManager,
    pub config: SimulationConfig,
    pub current_step: u32,
    pub components: Vec<Box<dyn Component>>,
}

impl Simulation {
    /// Create new simulation with config
    pub fn new(config: SimulationConfig) -> Self {
        let mut coll_mgr = CollectionsManager::new();
        
        // Add the geological cells collection
        coll_mgr.add_empty_collection::<CellLocation, GeologicalCellData>(CollectionName::GeologicalCells.as_str());
        
        Self {
            coll_mgr,
            config,
            current_step: 0,
            components: Vec::new(),
        }
    }
    
    /// Initialize cells based on configuration
    pub fn initialize_cells(&mut self) {
        // Get the collection
        let cells_collection = self.coll_mgr
            .get::<CellLocation, GeologicalCellData>(CollectionName::GeologicalCells.as_str())
            .expect("GeologicalCells collection should exist");
        
        // Initialize cells for each layer
        for (layer_index, layer_config) in self.config.layers.iter().enumerate() {
            self.initialize_layer(layer_index, layer_config, &cells_collection);
        }
    }
    
    /// Initialize a single layer using H3Utils
    fn initialize_layer(
        &self,
        layer_index: usize,
        layer_config: &LayerConfig,
        cells_collection: &crate::collections::Collection<CellLocation, GeologicalCellData>
    ) {
        // Use H3Utils to get proper H3 cells with base cells
        let h3_cells_with_base = H3Utils::iter_cells_with_base(layer_config.resolution);

        // Use configured depth steps for this layer
        let depth_steps = layer_config.depth_steps;



        let mut cell_count = 0;

        // Initialize each H3 cell at each depth
        for (h3_cell, _base_cell) in h3_cells_with_base {
            for depth_index in 0..depth_steps {
                let cell_location = CellLocation::new(layer_index, h3_cell, depth_index);

                // Create initial cell data with depth-dependent properties
                let initial_energy_mass = EnergyMass::new(1000.0, 1000.0); // Default values

                // Temperature increases with depth
                let initial_temp = 300.0 + (depth_index as f64 * 10.0); // 10K per depth step

                // Pressure increases significantly with depth
                let initial_pressure = 101325.0 + (depth_index as f64 * 50000.0); // 50kPa per depth step

                // Density increases slightly with depth and pressure
                let initial_density = 2500.0 + (depth_index as f64 * 100.0); // 100 kg/m³ per depth step

                let cell_data = GeologicalCellData {
                    energy_mass: initial_energy_mass,
                    temperature_k: initial_temp,
                    pressure_pa: initial_pressure,
                    density_kg_m3: initial_density,
                };

                cells_collection.insert(cell_location, cell_data);
                cell_count += 1;
            }
        }


    }
    

    
    /// Get the geological cells collection
    pub fn get_geological_cells(&self) -> &crate::collections::Collection<CellLocation, GeologicalCellData> {
        self.coll_mgr
            .get::<CellLocation, GeologicalCellData>(CollectionName::GeologicalCells.as_str())
            .expect("GeologicalCells collection should exist")
    }
    
    /// Add a component to the simulation
    pub fn add_component(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }

    /// Initialize all components (call once after adding all components)
    pub fn initialize_components(&mut self) {
        // Temporarily take ownership to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        for component in &mut components {
            component.initialize(self, &self.config);
        }

        // Put components back
        self.components = components;
    }

    /// Complete all components (call once at end of simulation)
    pub fn complete_components(&mut self) {
        // Temporarily take ownership to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        for component in &mut components {
            component.complete(self, &self.config);
        }

        // Put components back
        self.components = components;
    }

    /// Run one simulation step with Actor pattern
    pub fn step(&mut self) {
        if self.components.is_empty() {
            self.current_step += 1;
            return;
        }

        // Process all components with Actor pattern
        let current_step = self.current_step + 1;
        let year = current_step as f64 * self.config.years_per_step as f64;

        // Update the current step in the collections manager
        self.coll_mgr.set_current_step(current_step);

        // For now, let's pass the collections manager reference directly to components
        // We'll use a different approach that doesn't require Arc for the initial setup

        // For now, process components sequentially to avoid borrowing issues
        // TODO: Implement proper parallel processing with Arc<CollectionsManager>
        let mut actors = Vec::new();

        for component in &self.components {
            let mut actor = Actor::new();

            // Component processes one step and adds changes to actor
            component.step(&self.coll_mgr, &mut actor, current_step, year, &self.config);

            actors.push(actor);
        }

        // Blend all actor changes with compression
        let blended_changes = ChangeController::blend(actors);

        // Apply blended changes atomically
        self.coll_mgr.apply_events(blended_changes).unwrap();

        self.current_step += 1;
    }
    
    /// Run the full simulation with complete lifecycle
    pub fn run(&mut self) {
        // Initialize all components
        self.initialize_components();

        // Run simulation steps
        while self.current_step < self.config.steps {
            self.step();
        }

        // Complete all components
        self.complete_components();
    }
    
    /// Get simulation statistics
    pub fn get_stats(&self) -> SimulationStats {
        let cells_collection = self.get_geological_cells();
        let total_cells = cells_collection.len();
        
        SimulationStats {
            current_step: self.current_step,
            total_steps: self.config.steps,
            total_cells,
            years_simulated: self.current_step * self.config.years_per_step,
        }
    }


}

/// Simulation statistics
#[derive(Debug)]
pub struct SimulationStats {
    pub current_step: u32,
    pub total_steps: u32,
    pub total_cells: usize,
    pub years_simulated: u32,
}

#[cfg(test)]
mod geological_tests;

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_config() -> SimulationConfig {
        SimulationConfig {
            planet: PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
            },
            years_per_step: 1000,
            steps: 100,
            layers: vec![
                LayerConfig {
                    height_per_step_km: 10.0,
                    depth_steps: 2,  // 2 steps = 20km total depth
                    resolution: Resolution::Five,
                    name: "Crust".to_string(),
                },
                LayerConfig {
                    height_per_step_km: 50.0,
                    depth_steps: 10, // 10 steps = 500km total depth
                    resolution: Resolution::Four,
                    name: "Upper Mantle".to_string(),
                },
            ],
        }
    }
    
    #[test]
    fn test_simulation_creation() {
        let config = create_test_config();
        let sim = Simulation::new(config);
        
        assert_eq!(sim.current_step, 0);
        assert_eq!(sim.config.steps, 100);
        assert_eq!(sim.config.layers.len(), 2);
    }
    
    #[test]
    fn test_cell_initialization() {
        let config = create_test_config();
        let mut sim = Simulation::new(config);
        
        sim.initialize_cells();
        
        let cells = sim.get_geological_cells();
        assert!(cells.len() > 0);
        
        println!("Initialized {} cells", cells.len());
    }
    
    #[test]
    fn test_simulation_step() {
        let config = create_test_config();
        let mut sim = Simulation::new(config);
        
        sim.initialize_cells();
        sim.step();
        
        assert_eq!(sim.current_step, 1);
        
        let stats = sim.get_stats();
        assert_eq!(stats.years_simulated, 1000);
    }

    #[test]
    fn test_starting_masses_and_temperatures_are_reasonable() {

        // Create realistic geological simulation
        let config = SimulationConfig {
            planet: PlanetConfig {
                radius_km: 6371.0,
                surface_gravity_m_s_s: 9.81,
            },
            years_per_step: 1000,
            steps: 1,
            layers: vec![
                LayerConfig {
                    height_per_step_km: 5.0,   // 5km crust steps
                    depth_steps: 3,            // 15km total crust
                    resolution: Resolution::Four, // Medium resolution for testing
                    name: "Continental Crust".to_string(),
                },
                LayerConfig {
                    height_per_step_km: 25.0,  // 25km mantle steps
                    depth_steps: 2,            // 50km upper mantle
                    resolution: Resolution::Three,
                    name: "Upper Mantle".to_string(),
                },
            ],
        };

        let mut sim = Simulation::new(config);
        sim.initialize_cells();

        // Add LayerCellComponent to initialize geological properties
        sim.add_component(Box::new(crate::components::LayerCellComponent::with_surface_temperature(288.15)));
        sim.initialize_components();
        sim.step(); // Apply geological initialization

        let cells = sim.get_geological_cells();


        let mut surface_temps = Vec::new();
        let mut deep_temps = Vec::new();
        let mut surface_masses = Vec::new();
        let mut energy_per_kg_values = Vec::new();

        // Collect sample data from different depths
        for entry in cells.iter().take(100) { // Test first 100 cells
            let (location, data) = (entry.key(), entry.value());
            let mass_kg = data.energy_mass.mass_kg();
            let energy_j = data.energy_mass.energy_joules();
            let energy_per_kg = if mass_kg > 0.0 { energy_j / mass_kg } else { 0.0 };

            energy_per_kg_values.push(energy_per_kg);

            if location.depth_index() == 0 {
                // Surface cells
                surface_temps.push(data.temperature_k);
                surface_masses.push(mass_kg);
            } else if location.depth_index() >= 2 {
                // Deeper cells
                deep_temps.push(data.temperature_k);
            }

            // Individual cell validation
            assert!(data.temperature_k > 200.0,
                   "Temperature too low: {:.1}K at layer {} depth {}",
                   data.temperature_k, location.layer_set_index(), location.depth_index());
            assert!(data.temperature_k < 3000.0,
                   "Temperature too high: {:.1}K at layer {} depth {}",
                   data.temperature_k, location.layer_set_index(), location.depth_index());

            assert!(mass_kg > 1e12,
                   "Mass too low: {:.2e}kg at layer {} depth {}",
                   mass_kg, location.layer_set_index(), location.depth_index());
            assert!(mass_kg < 1e20,
                   "Mass too high: {:.2e}kg at layer {} depth {}",
                   mass_kg, location.layer_set_index(), location.depth_index());

            assert!(energy_per_kg > 1e4,
                   "Energy per kg too low: {:.2e}J/kg at layer {} depth {}",
                   energy_per_kg, location.layer_set_index(), location.depth_index());
            assert!(energy_per_kg < 1e8,
                   "Energy per kg too high: {:.2e}J/kg at layer {} depth {}",
                   energy_per_kg, location.layer_set_index(), location.depth_index());

            assert!(data.pressure_pa > 1e3,
                   "Pressure too low: {:.2e}Pa at layer {} depth {}",
                   data.pressure_pa, location.layer_set_index(), location.depth_index());

            assert!(data.density_kg_m3 > 1000.0,
                   "Density too low: {:.1}kg/m³ at layer {} depth {}",
                   data.density_kg_m3, location.layer_set_index(), location.depth_index());
            assert!(data.density_kg_m3 < 10000.0,
                   "Density too high: {:.1}kg/m³ at layer {} depth {}",
                   data.density_kg_m3, location.layer_set_index(), location.depth_index());
        }

        // Statistical validation
        if !surface_temps.is_empty() {
            let avg_surface_temp = surface_temps.iter().sum::<f64>() / surface_temps.len() as f64;
            assert!(avg_surface_temp > 250.0 && avg_surface_temp < 350.0,
                   "Average surface temperature unrealistic: {:.1}K", avg_surface_temp);
        }

        if !deep_temps.is_empty() {
            let avg_deep_temp = deep_temps.iter().sum::<f64>() / deep_temps.len() as f64;
            assert!(avg_deep_temp > surface_temps.get(0).copied().unwrap_or(300.0),
                   "Deep temperature should be higher than surface");
        }

        if !surface_masses.is_empty() {
            let avg_surface_mass = surface_masses.iter().sum::<f64>() / surface_masses.len() as f64;
            assert!(avg_surface_mass > 1e15 && avg_surface_mass < 1e18,
                   "Average surface mass unrealistic: {:.2e}kg", avg_surface_mass);
        }

        if !energy_per_kg_values.is_empty() {
            let avg_energy_per_kg = energy_per_kg_values.iter().sum::<f64>() / energy_per_kg_values.len() as f64;
            assert!(avg_energy_per_kg > 1e5 && avg_energy_per_kg < 1e7,
                   "Average energy per kg unrealistic: {:.2e}J/kg", avg_energy_per_kg);
        }
    }
}
