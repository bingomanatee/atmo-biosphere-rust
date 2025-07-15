use crate::component::SimComponent;
use std::collections::HashMap;

pub struct SimulationConfig {
    pub steps: u64,
    pub years_per_step: f64,
    pub warmup_steps: u64,
}

pub struct Simulation {
    state: SimulationState,
    step: i64,
    steps: u64,
    config: SimulationConfig,
    components: HashMap<&'static str, Box<dyn SimComponent>>,
}

pub enum SimulationState {
    Created,
    RunningWarmup,
    Running,
    Paused,
    Stopped,
    Error,
}

impl Simulation {
    pub fn new(config: SimulationConfig, components: &mut Vec<Box<dyn SimComponent>>) -> Self {
        let mut sim = Simulation {
            state: SimulationState::Created,
            step: 0.min((config.warmup_steps as i64) * -1),
            steps: 0,
            config: config,
            components: HashMap::new(),
        };
        for comp in components.drain(..) {
            sim.register_box(comp);
        }
        sim
    }

    pub fn register_box(&mut self, comp_box: Box<dyn SimComponent>) {
        let key = comp_box.key();
        self.components.insert(key, comp_box);
    }

    fn run(&mut self) {
        match self.state {
            SimulationState::Created => {
                self.initialize();
                if self.config.warmup_steps == 0 {
                    self.state = SimulationState::Running;
                } else {
                    self.state = SimulationState::RunningWarmup;
                }
                self.step();
            },
            _ => todo!(),
        }
    }

    pub fn initialize(&mut self) {
        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        for (_, comp) in components.iter_mut() {
            comp.initialize(self);
        }

        // Put the components back
        self.components = components;
    }

    fn current_year(&self) -> i64 {
        self.step * self.config.years_per_step as i64
    }

    fn step(&mut self) {
        let step = self.step;
        let year = self.current_year();
        // We need to temporarily take ownership of components to avoid borrowing issues
        let mut components = std::mem::take(&mut self.components);

        for (_, comp) in components.iter_mut() {
            comp.update(self, step, year);
        }

        for (_, comp) in components.iter_mut() {
            comp.report(self, step, year);
        }

        // Put the components back
        self.components = components;
    }
}
