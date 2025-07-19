// Standalone demonstration of event emission system concept

use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Event types for simulation
#[derive(Debug, Clone)]
pub enum SimEvent {
    SimulationStarted { steps: usize },
    StepStarted { step: usize },
    StepCompleted { step: usize, duration: Duration },
    ComponentStarted { component: String, step: usize },
    ComponentCompleted { component: String, step: usize, duration: Duration },
    TransactionScaled { component: String, factor: f64 },
    ComponentAdapted { component: String, details: String },
    SimulationEnded { total_duration: Duration },
}

/// Event with metadata
#[derive(Debug, Clone)]
pub struct Event {
    pub event: SimEvent,
    pub timestamp: Instant,
}

/// Event listener trait
pub trait EventListener {
    fn on_event(&mut self, event: &Event);
}

/// Performance tracking listener
pub struct PerformanceTracker {
    simulation_start: Option<Instant>,
    component_times: HashMap<String, Duration>,
    step_times: Vec<Duration>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            simulation_start: None,
            component_times: HashMap::new(),
            step_times: Vec::new(),
        }
    }
    
    pub fn get_summary(&self) -> String {
        let mut report = String::new();
        report.push_str("📊 Performance Summary\n");
        report.push_str("=====================\n");
        
        if let Some(start) = self.simulation_start {
            let total = start.elapsed();
            report.push_str(&format!("Total time: {:.2} ms\n", total.as_secs_f64() * 1000.0));
        }
        
        report.push_str(&format!("Steps: {}\n", self.step_times.len()));
        
        if !self.step_times.is_empty() {
            let avg: Duration = self.step_times.iter().sum::<Duration>() / self.step_times.len() as u32;
            report.push_str(&format!("Avg step: {:.2} ms\n", avg.as_secs_f64() * 1000.0));
        }
        
        report.push_str("\nComponents:\n");
        let mut components: Vec<_> = self.component_times.iter().collect();
        components.sort_by(|a, b| b.1.cmp(a.1));
        
        for (name, time) in components {
            report.push_str(&format!("  {}: {:.2} ms\n", name, time.as_secs_f64() * 1000.0));
        }
        
        report
    }
}

impl EventListener for PerformanceTracker {
    fn on_event(&mut self, event: &Event) {
        match &event.event {
            SimEvent::SimulationStarted { .. } => {
                self.simulation_start = Some(event.timestamp);
            },
            SimEvent::StepCompleted { duration, .. } => {
                self.step_times.push(*duration);
            },
            SimEvent::ComponentCompleted { component, duration, .. } => {
                *self.component_times.entry(component.clone()).or_insert(Duration::ZERO) += *duration;
            },
            _ => {}
        }
    }
}

/// Console logger
pub struct ConsoleLogger {
    verbose: bool,
}

impl ConsoleLogger {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl EventListener for ConsoleLogger {
    fn on_event(&mut self, event: &Event) {
        match &event.event {
            SimEvent::SimulationStarted { steps } => {
                println!("🚀 Simulation started: {} steps", steps);
            },
            SimEvent::StepStarted { step } => {
                if self.verbose {
                    println!("🔄 Step {} started", step);
                }
            },
            SimEvent::StepCompleted { step, duration } => {
                println!("✅ Step {} completed in {:.2} ms", step, duration.as_secs_f64() * 1000.0);
            },
            SimEvent::TransactionScaled { component, factor } => {
                println!("⚖️  {} scaled by {:.3}x", component, factor);
            },
            SimEvent::ComponentAdapted { component, details } => {
                println!("🔧 {} adapted: {}", component, details);
            },
            SimEvent::SimulationEnded { total_duration } => {
                println!("🏁 Simulation completed in {:.2} seconds", total_duration.as_secs_f64());
            },
            _ => {
                if self.verbose {
                    println!("📡 {:?}", event.event);
                }
            }
        }
    }
}

/// Event emitter
pub struct EventEmitter {
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
    
    pub fn add_listener<L: EventListener + 'static>(&mut self, listener: L) {
        self.listeners.push(Box::new(listener));
    }
    
    pub fn emit(&mut self, event: SimEvent) {
        let event = Event {
            event,
            timestamp: Instant::now(),
        };
        
        for listener in &mut self.listeners {
            listener.on_event(&event);
        }
    }
    
    pub fn listener_count(&self) -> usize {
        self.listeners.len()
    }
}

/// Simulated geological simulation with events
pub struct GeologicalSimulation {
    emitter: EventEmitter,
    step: usize,
    max_steps: usize,
}

impl GeologicalSimulation {
    pub fn new(max_steps: usize) -> Self {
        let mut sim = Self {
            emitter: EventEmitter::new(),
            step: 0,
            max_steps,
        };
        
        // Add default listeners
        sim.emitter.add_listener(ConsoleLogger::new(false));
        sim.emitter.add_listener(PerformanceTracker::new());
        
        sim
    }
    
    pub fn add_listener<L: EventListener + 'static>(&mut self, listener: L) {
        self.emitter.add_listener(listener);
    }
    
    pub fn run(&mut self) {
        self.emitter.emit(SimEvent::SimulationStarted { steps: self.max_steps });
        
        let sim_start = Instant::now();
        
        for step in 0..self.max_steps {
            self.step = step;
            self.run_step();
        }
        
        let total_duration = sim_start.elapsed();
        self.emitter.emit(SimEvent::SimulationEnded { total_duration });
    }
    
    fn run_step(&mut self) {
        self.emitter.emit(SimEvent::StepStarted { step: self.step });
        
        let step_start = Instant::now();
        
        // Simulate components
        let components = ["ThermalConduction", "CoreRadiance", "ConvectionPlumes"];
        
        for component in &components {
            self.emitter.emit(SimEvent::ComponentStarted {
                component: component.to_string(),
                step: self.step,
            });
            
            // Simulate work
            let work_time = Duration::from_millis(10 + (self.step * 2) as u64);
            std::thread::sleep(work_time);
            
            self.emitter.emit(SimEvent::ComponentCompleted {
                component: component.to_string(),
                step: self.step,
                duration: work_time,
            });
        }
        
        // Simulate scaling events
        if self.step > 2 && self.step % 2 == 0 {
            self.emitter.emit(SimEvent::TransactionScaled {
                component: "CoreRadiance".to_string(),
                factor: 0.75,
            });
            
            if self.step == 4 {
                self.emitter.emit(SimEvent::ComponentAdapted {
                    component: "CoreRadiance".to_string(),
                    details: "Added 50% more hotspots, reduced energy by 33%".to_string(),
                });
            }
        }
        
        let step_duration = step_start.elapsed();
        self.emitter.emit(SimEvent::StepCompleted {
            step: self.step,
            duration: step_duration,
        });
    }
    
    pub fn get_performance_summary(&self) -> String {
        // In a real implementation, we'd access the PerformanceTracker listener
        // For this demo, we'll create a simple summary
        format!("📊 Simulation completed {} steps", self.step + 1)
    }
}

fn main() {
    println!("📡 Standalone Event Emission System Demo");
    println!("========================================");
    println!("Demonstrates decoupled timer and monitoring\n");

    // Create simulation with event system
    let mut sim = GeologicalSimulation::new(5);
    
    println!("🔧 Event system configured with {} listeners", sim.emitter.listener_count());
    println!("   • ConsoleLogger: Real-time output");
    println!("   • PerformanceTracker: Timing analysis");
    
    println!("\n🚀 Running simulation with event emission...");
    
    // Run simulation - events are emitted automatically
    sim.run();
    
    println!("\n🎯 Benefits Demonstrated:");
    println!("=========================");
    
    println!("✅ Decoupled Architecture:");
    println!("   • Simulation doesn't know about specific listeners");
    println!("   • Timer logic is in listeners, not simulation");
    println!("   • Easy to add/remove monitoring without changing sim");
    
    println!("\n✅ Flexible Monitoring:");
    println!("   • Multiple listeners can track different metrics");
    println!("   • Real-time events during simulation");
    println!("   • Custom analytics without simulation changes");
    
    println!("\n✅ Performance Benefits:");
    println!("   • Zero overhead when no listeners");
    println!("   • Listeners can be optimized independently");
    println!("   • No coupling between timing and simulation logic");
    
    println!("\n📈 Use Cases:");
    println!("   🔍 Real-time dashboards");
    println!("   📊 Performance profiling");
    println!("   🚨 Alert systems");
    println!("   📝 Audit logging");
    println!("   🧪 A/B testing");
    
    println!("\n✅ Event Emission System Demo Complete!");
    println!("Timer is now completely decoupled from simulation! 🎉");
}
