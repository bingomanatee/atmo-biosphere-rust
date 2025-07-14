use crate::sim::Simulation;

pub trait SimComponent: std::any::Any + Send + Sync {
    /// A key for this component instance
    fn key(&self) -> &'static str;

    fn initialize(&mut self, sim: &mut Simulation);

    fn update(&mut self, sim: &Simulation, step: i64, year: i64);

    fn report(&mut self, sim: & Simulation, step: i64, year: i64);

    fn complete(&mut self, sim: & Simulation);
}
