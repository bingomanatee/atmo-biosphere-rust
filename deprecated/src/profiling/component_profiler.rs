use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Performance metrics for a single component method
#[derive(Debug, Clone)]
pub struct MethodMetrics {
    pub total_time: Duration,
    pub call_count: u64,
    pub min_time: Duration,
    pub max_time: Duration,
    pub last_time: Duration,
}

impl Default for MethodMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl MethodMetrics {
    pub fn new() -> Self {
        Self {
            total_time: Duration::ZERO,
            call_count: 0,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
            last_time: Duration::ZERO,
        }
    }
    
    pub fn record_call(&mut self, duration: Duration) {
        self.total_time += duration;
        self.call_count += 1;
        self.min_time = self.min_time.min(duration);
        self.max_time = self.max_time.max(duration);
        self.last_time = duration;
    }
    
    pub fn average_time(&self) -> Duration {
        if self.call_count > 0 {
            self.total_time / self.call_count as u32
        } else {
            Duration::ZERO
        }
    }
    
    pub fn total_time_ms(&self) -> f64 {
        self.total_time.as_secs_f64() * 1000.0
    }
    
    pub fn average_time_ms(&self) -> f64 {
        self.average_time().as_secs_f64() * 1000.0
    }
}

/// Performance metrics for a single component
#[derive(Debug, Clone)]
pub struct ComponentMetrics {
    pub component_name: String,
    pub methods: HashMap<String, MethodMetrics>,
}

impl Default for ComponentMetrics {
    fn default() -> Self {
        Self::new("unknown".to_string())
    }
}

impl ComponentMetrics {
    pub fn new(component_name: String) -> Self {
        Self {
            component_name,
            methods: HashMap::new(),
        }
    }
    
    pub fn record_method_call(&mut self, method_name: &str, duration: Duration) {
        let metrics = self.methods.entry(method_name.to_string()).or_insert_with(MethodMetrics::new);
        metrics.record_call(duration);
    }
    
    pub fn total_time(&self) -> Duration {
        self.methods.values().map(|m| m.total_time).sum()
    }
    
    pub fn total_time_ms(&self) -> f64 {
        self.total_time().as_secs_f64() * 1000.0
    }
}

/// Global profiler for all components
#[derive(Debug, Default)]
pub struct ComponentProfiler {
    components: HashMap<String, ComponentMetrics>,
    simulation_start: Option<Instant>,
    simulation_total_time: Duration,
}

impl ComponentProfiler {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            simulation_start: None,
            simulation_total_time: Duration::ZERO,
        }
    }
    
    pub fn start_simulation(&mut self) {
        self.simulation_start = Some(Instant::now());
    }
    
    pub fn end_simulation(&mut self) {
        if let Some(start) = self.simulation_start.take() {
            self.simulation_total_time = start.elapsed();
        }
    }
    
    pub fn record_method_call(&mut self, component_name: &str, method_name: &str, duration: Duration) {
        let component_metrics = self.components
            .entry(component_name.to_string())
            .or_insert_with(|| ComponentMetrics::new(component_name.to_string()));
        
        component_metrics.record_method_call(method_name, duration);
    }
    
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("🔍 COMPONENT PERFORMANCE REPORT\n");
        report.push_str("================================\n\n");
        
        // Simulation overview
        let sim_time_ms = self.simulation_total_time.as_secs_f64() * 1000.0;
        report.push_str(&format!("📊 Simulation Total Time: {:.2} ms ({:.2} seconds)\n\n", 
            sim_time_ms, self.simulation_total_time.as_secs_f64()));
        
        // Sort components by total time
        let mut components: Vec<_> = self.components.values().collect();
        components.sort_by(|a, b| b.total_time().cmp(&a.total_time()));
        
        // Component summary
        report.push_str("🏆 COMPONENT RANKING (by total time)\n");
        report.push_str("====================================\n");
        
        for (rank, component) in components.iter().enumerate() {
            let percentage = if sim_time_ms > 0.0 {
                (component.total_time_ms() / sim_time_ms) * 100.0
            } else {
                0.0
            };
            
            report.push_str(&format!("{}. {}: {:.2} ms ({:.1}%)\n", 
                rank + 1, component.component_name, component.total_time_ms(), percentage));
        }
        
        report.push_str("\n");
        
        // Detailed breakdown
        report.push_str("📋 DETAILED BREAKDOWN\n");
        report.push_str("=====================\n\n");
        
        for component in &components {
            report.push_str(&format!("🔧 {}\n", component.component_name));
            report.push_str(&format!("   Total: {:.2} ms\n", component.total_time_ms()));
            
            // Sort methods by total time
            let mut methods: Vec<_> = component.methods.iter().collect();
            methods.sort_by(|a, b| b.1.total_time.cmp(&a.1.total_time));
            
            for (method_name, metrics) in methods {
                let method_percentage = if component.total_time_ms() > 0.0 {
                    (metrics.total_time_ms() / component.total_time_ms()) * 100.0
                } else {
                    0.0
                };
                
                report.push_str(&format!("   📌 {}: {:.2} ms ({:.1}%) - {} calls, avg: {:.2} ms\n",
                    method_name, 
                    metrics.total_time_ms(),
                    method_percentage,
                    metrics.call_count,
                    metrics.average_time_ms()));
                
                if metrics.call_count > 1 {
                    report.push_str(&format!("      ⏱️  min: {:.2} ms, max: {:.2} ms, last: {:.2} ms\n",
                        metrics.min_time.as_secs_f64() * 1000.0,
                        metrics.max_time.as_secs_f64() * 1000.0,
                        metrics.last_time.as_secs_f64() * 1000.0));
                }
            }
            report.push_str("\n");
        }
        
        // Performance insights
        report.push_str("💡 PERFORMANCE INSIGHTS\n");
        report.push_str("=======================\n");
        
        if let Some(slowest) = components.first() {
            report.push_str(&format!("🐌 Slowest component: {} ({:.1}% of total time)\n", 
                slowest.component_name, 
                (slowest.total_time_ms() / sim_time_ms) * 100.0));
        }
        
        // Find most called method
        let mut all_methods: Vec<(&str, &str, &MethodMetrics)> = Vec::new();
        for component in &components {
            for (method_name, metrics) in &component.methods {
                all_methods.push((&component.component_name, method_name, metrics));
            }
        }
        
        if let Some((comp_name, method_name, metrics)) = all_methods.iter()
            .max_by_key(|(_, _, m)| m.call_count) {
            report.push_str(&format!("🔄 Most called method: {}::{} ({} calls)\n", 
                comp_name, method_name, metrics.call_count));
        }
        
        if let Some((comp_name, method_name, metrics)) = all_methods.iter()
            .max_by_key(|(_, _, m)| m.max_time) {
            report.push_str(&format!("⏰ Slowest single call: {}::{} ({:.2} ms)\n", 
                comp_name, method_name, metrics.max_time.as_secs_f64() * 1000.0));
        }
        
        report
    }
    
    pub fn reset(&mut self) {
        self.components.clear();
        self.simulation_start = None;
        self.simulation_total_time = Duration::ZERO;
    }

    /// Get component summary data for custom reporting
    pub fn get_component_summary(&self) -> &HashMap<String, ComponentMetrics> {
        &self.components
    }

    /// Get method summary for a specific component
    pub fn get_method_summary(&self, component_name: &str) -> Option<&HashMap<String, MethodMetrics>> {
        self.components.get(component_name).map(|c| &c.methods)
    }

    /// Get total simulation time in milliseconds
    pub fn total_time_ms(&self) -> f64 {
        self.simulation_total_time.as_secs_f64() * 1000.0
    }
}

/// Macro for easy method timing
#[macro_export]
macro_rules! time_method {
    ($profiler:expr, $component_name:expr, $method_name:expr, $code:block) => {
        {
            let start = std::time::Instant::now();
            let result = $code;
            let duration = start.elapsed();
            $profiler.record_method_call($component_name, $method_name, duration);
            result
        }
    };
}

/// RAII timer for automatic method timing
pub struct MethodTimer<'a> {
    profiler: &'a mut ComponentProfiler,
    component_name: String,
    method_name: String,
    start: Instant,
}

impl<'a> MethodTimer<'a> {
    pub fn new(profiler: &'a mut ComponentProfiler, component_name: &str, method_name: &str) -> Self {
        Self {
            profiler,
            component_name: component_name.to_string(),
            method_name: method_name.to_string(),
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for MethodTimer<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.profiler.record_method_call(&self.component_name, &self.method_name, duration);
    }
}
