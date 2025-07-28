use crate::collections::{CollectionsManager, Actor, ChangeController};
use crate::cell_location::CellLocation;
use crate::energy_mass::EnergyMass;
use crate::utils::h3_utils::H3Utils;
use h3o::Resolution;

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
}

/// Layer configuration
#[derive(Debug, Clone)]
pub struct LayerConfig {
    pub height_per_step_km: f64,
    pub depth_steps: usize,  // Number of depth steps in this layer
    pub resolution: Resolution,
    pub name: String,
}

/// Cell data - closely resembles energy_mass_cell
#[derive(Debug, Clone)]
pub struct GeologicalCellData {
    pub energy_mass: EnergyMass,
    pub temperature_k: f64,
    pub pressure_pa: f64,
    pub density_kg_m3: f64,
}



/// Component trait for Actor-based processing
pub trait Component: Send + Sync {
    /// Process cells and add changes to actor
    fn process(&self, manager: &CollectionsManager, actor: &mut Actor);

    /// Component name for debugging
    fn name(&self) -> &'static str;
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

        println!("  Initializing layer {} '{}' with resolution {:?}, {} depth steps ({}km per step)",
                 layer_index, layer_config.name, layer_config.resolution, depth_steps, layer_config.height_per_step_km);

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

        println!("    Layer {} initialized with {} cells", layer_index, cell_count);
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

    /// Run one simulation step with Actor pattern
    pub fn step(&mut self) {
        if self.components.is_empty() {
            println!("Simulation step {}/{} (no components)", self.current_step + 1, self.config.steps);
            self.current_step += 1;
            return;
        }

        println!("🚀 Step {}/{}: Processing {} components in parallel...",
                 self.current_step + 1, self.config.steps, self.components.len());

        // Process all components with Actor pattern
        let actors: Vec<Actor> = crossbeam::scope(|s| {
            let handles: Vec<_> = self.components.iter().map(|component| {
                let manager = &self.coll_mgr;
                s.spawn(move |_| {
                    let mut actor = Actor::new();
                    println!("  {} processing...", component.name());

                    // Component processes cells and adds changes to actor
                    component.process(manager, &mut actor);

                    println!("    {}: {} changes queued", component.name(), actor.change_count());
                    actor
                })
            }).collect();

            // Collect all actors
            handles.into_iter().map(|handle| handle.join().unwrap()).collect()
        }).unwrap();

        // Blend all actor changes with compression
        let total_changes: usize = actors.iter().map(|a| a.change_count()).sum();
        let blended_changes = ChangeController::blend(actors);

        println!("  🔄 Compressed {} changes into {} optimized changes",
                 total_changes, blended_changes.len());

        // Apply blended changes atomically
        self.coll_mgr.apply_events(blended_changes).unwrap();

        self.current_step += 1;
        println!("  ✅ Step {} completed", self.current_step);
    }
    
    /// Run the full simulation
    pub fn run(&mut self) {
        println!("Starting simulation with {} steps", self.config.steps);
        
        while self.current_step < self.config.steps {
            self.step();
        }
        
        println!("Simulation completed");
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
}
