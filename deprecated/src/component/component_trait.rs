use crate::sim_immut::simulation_immut::SimulationImmut;

pub trait SimComponent: std::any::Any + Send + Sync {
    /// A key for this component instance
    fn key(&self) -> &'static str;

    /// Initialize the component with the simulation
    fn initialize(&mut self, sim: &mut SimulationImmut);

    /// Execute one simulation step - components organize internally as needed
    fn step(&mut self, sim: &mut SimulationImmut, step: i64, year: i64);

    /// Clean up when simulation is complete
    fn complete(&mut self, sim: &SimulationImmut);

    /// Adapt component if it's causing excessive transaction scaling (optional)
    fn adapt_if_overpowered(&mut self, _sim: &SimulationImmut, _scaling_detected: bool) {
        // Default implementation does nothing
        // Components that can adapt (like CoreRadianceComponent) should override this
    }
}
