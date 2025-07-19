// Event listener trait and implementations

use super::event_types::*;
use std::time::Duration;
use std::collections::HashMap;

/// Trait for handling simulation events
pub trait EventListener {
    fn on_event(&mut self, event: &Event);
}

/// Performance monitoring listener (replaces ComponentProfiler)
pub struct PerformanceListener {
    simulation_start: Option<std::time::Instant>,
    step_timings: Vec<Duration>,
    component_timings: HashMap<String, ComponentPerformance>,
    current_step: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ComponentPerformance {
    pub component_name: String,
    pub total_time: Duration,
    pub method_calls: HashMap<String, MethodPerformance>,
}

#[derive(Debug, Clone)]
pub struct MethodPerformance {
    pub total_time: Duration,
    pub call_count: usize,
}

impl PerformanceListener {
    pub fn new() -> Self {
        Self {
            simulation_start: None,
            step_timings: Vec::new(),
            component_timings: HashMap::new(),
            current_step: None,
        }
    }
    
    pub fn get_performance_summary(&self) -> String {
        let mut report = String::new();
        report.push_str("📊 Performance Summary\n");
        report.push_str("=====================\n");
        
        if let Some(start) = self.simulation_start {
            let total_time = start.elapsed();
            report.push_str(&format!("Total simulation time: {:.2} ms\n", total_time.as_secs_f64() * 1000.0));
        }
        
        report.push_str(&format!("Steps completed: {}\n", self.step_timings.len()));
        
        if !self.step_timings.is_empty() {
            let avg_step_time: Duration = self.step_timings.iter().sum::<Duration>() / self.step_timings.len() as u32;
            report.push_str(&format!("Average step time: {:.2} ms\n", avg_step_time.as_secs_f64() * 1000.0));
        }
        
        // Component rankings
        let mut components: Vec<_> = self.component_timings.values().collect();
        components.sort_by(|a, b| b.total_time.cmp(&a.total_time));
        
        report.push_str("\n🏆 Component Rankings:\n");
        for (rank, component) in components.iter().enumerate() {
            report.push_str(&format!("{}. {}: {:.2} ms\n", 
                rank + 1, 
                component.component_name, 
                component.total_time.as_secs_f64() * 1000.0));
        }
        
        report
    }
    
    pub fn get_component_details(&self, component_name: &str) -> Option<String> {
        if let Some(component) = self.component_timings.get(component_name) {
            let mut report = String::new();
            report.push_str(&format!("🔧 {} Details\n", component_name));
            report.push_str(&format!("Total time: {:.2} ms\n", component.total_time.as_secs_f64() * 1000.0));
            
            let mut methods: Vec<_> = component.method_calls.iter().collect();
            methods.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
            
            for (method_name, method_perf) in methods {
                report.push_str(&format!("  {}: {:.2} ms ({} calls)\n", 
                    method_name, 
                    method_perf.total_time.as_secs_f64() * 1000.0,
                    method_perf.call_count));
            }
            
            Some(report)
        } else {
            None
        }
    }
}

impl EventListener for PerformanceListener {
    fn on_event(&mut self, event: &Event) {
        match &event.event {
            SimulationEvent::SimulationStarted { .. } => {
                self.simulation_start = Some(event.metadata.timestamp);
            },
            
            SimulationEvent::StepStarted { step, .. } => {
                self.current_step = Some(*step);
            },
            
            SimulationEvent::StepCompleted { duration, .. } => {
                self.step_timings.push(*duration);
            },
            
            SimulationEvent::ComponentCompleted { component_name, method_name, duration, .. } => {
                let component = self.component_timings
                    .entry(component_name.clone())
                    .or_insert_with(|| ComponentPerformance {
                        component_name: component_name.clone(),
                        total_time: Duration::ZERO,
                        method_calls: HashMap::new(),
                    });
                
                component.total_time += *duration;
                
                let method = component.method_calls
                    .entry(method_name.clone())
                    .or_insert_with(|| MethodPerformance {
                        total_time: Duration::ZERO,
                        call_count: 0,
                    });
                
                method.total_time += *duration;
                method.call_count += 1;
            },
            
            _ => {} // Ignore other events
        }
    }
}

/// Console logging listener
pub struct ConsoleListener {
    verbose: bool,
}

impl ConsoleListener {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl EventListener for ConsoleListener {
    fn on_event(&mut self, event: &Event) {
        match &event.event {
            SimulationEvent::SimulationStarted { step_count, years_per_step } => {
                println!("🚀 Simulation started: {} steps, {:.0} years/step", step_count, years_per_step);
            },
            
            SimulationEvent::StepStarted { step, year } => {
                if self.verbose {
                    println!("🔄 Step {} started (Year {})", step, year);
                }
            },
            
            SimulationEvent::StepCompleted { step, duration, .. } => {
                println!("✅ Step {} completed in {:.2} ms", step, duration.as_secs_f64() * 1000.0);
            },
            
            SimulationEvent::TransactionScaled { component_name, scaling_factor, reason } => {
                println!("⚖️  {} transactions scaled by {:.3}x: {}", component_name, scaling_factor, reason);
            },
            
            SimulationEvent::ComponentAdapted { component_name, adaptation_type, details } => {
                println!("🔧 {} adapted ({}): {}", component_name, adaptation_type, details);
            },
            
            SimulationEvent::Warning { component_name, message, step } => {
                println!("⚠️  Warning in {} (Step {}): {}", component_name, step, message);
            },
            
            SimulationEvent::Error { component_name, error, step } => {
                println!("❌ Error in {} (Step {}): {}", component_name, step, error);
            },
            
            SimulationEvent::SimulationEnded { total_steps, total_duration } => {
                println!("🏁 Simulation completed: {} steps in {:.2} seconds", 
                    total_steps, total_duration.as_secs_f64());
            },
            
            _ => {
                if self.verbose {
                    println!("📡 Event: {:?}", event.event);
                }
            }
        }
    }
}

/// File logging listener
pub struct FileListener {
    file_path: String,
    events: Vec<Event>,
}

impl FileListener {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            events: Vec::new(),
        }
    }
    
    pub fn save_to_file(&self) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(&self.file_path)?;
        
        writeln!(file, "Simulation Event Log")?;
        writeln!(file, "===================")?;
        
        for event in &self.events {
            writeln!(file, "{:?}", event)?;
        }
        
        Ok(())
    }
}

impl EventListener for FileListener {
    fn on_event(&mut self, event: &Event) {
        self.events.push(event.clone());
    }
}
