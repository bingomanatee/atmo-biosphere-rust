use crate::sim::Simulation;

pub trait SimComponent: std::any::Any + Send + Sync {
    /// A key for this component instance
    fn key(&self) -> &'static str;

    /// Initialize the component with the simulation
    fn initialize(&mut self, sim: &mut Simulation);

    /// Execute one simulation step - components organize internally as needed
    fn step(&mut self, sim: &mut Simulation, step: i64, year: i64);

    /// Clean up when simulation is complete
    fn complete(&mut self, sim: &Simulation);
}
