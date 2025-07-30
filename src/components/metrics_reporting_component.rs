use crate::collections::{Actor, CollectionsManager};
use crate::simulation::{Component, Simulation, SimulationConfig, CollectionName, PerformanceMetricKey, PerformanceMetricData};
use std::collections::HashMap;

/// Component that reports performance metrics and timing analysis
pub struct MetricsReportingComponent {
    /// Whether to print detailed per-step metrics
    pub detailed_reporting: bool,
    /// Whether to print component comparison analysis
    pub component_analysis: bool,
    /// Whether to print performance trends over time
    pub trend_analysis: bool,
    /// Minimum duration threshold for reporting (ms)
    pub min_duration_threshold_ms: f64,
}

impl MetricsReportingComponent {
    /// Create new metrics reporting component with default settings
    pub fn new() -> Self {
        Self {
            detailed_reporting: true,
            component_analysis: true,
            trend_analysis: false,
            min_duration_threshold_ms: 0.1, // Report anything over 0.1ms
        }
    }
    
    /// Create metrics reporter with custom settings
    pub fn with_settings(
        detailed: bool, 
        component_analysis: bool, 
        trend_analysis: bool,
        threshold_ms: f64
    ) -> Self {
        Self {
            detailed_reporting: detailed,
            component_analysis,
            trend_analysis,
            min_duration_threshold_ms: threshold_ms,
        }
    }
    
    /// Generate comprehensive performance report from collections
    fn generate_performance_report(&self, coll_mgr: &CollectionsManager) -> String {
        let perf_collection = match coll_mgr.get::<PerformanceMetricKey, PerformanceMetricData>(
            CollectionName::PerformanceMetrics.as_str()
        ) {
            Some(collection) => collection,
            None => return "❌ Performance metrics collection not found".to_string(),
        };
        
        let mut report = String::new();
        report.push_str("📊 COMPREHENSIVE PERFORMANCE REPORT\n");
        report.push_str("=====================================\n\n");
        
        // Collect and organize metrics
        let mut step_times = Vec::new();
        let mut component_totals: HashMap<String, f64> = HashMap::new();
        let mut component_counts: HashMap<String, usize> = HashMap::new();
        let mut step_component_times: HashMap<u32, HashMap<String, f64>> = HashMap::new();
        
        for entry in perf_collection.iter() {
            let data = entry.value();
            
            if data.duration_ms >= self.min_duration_threshold_ms {
                if data.component_name == "simulation" && data.method_name == "step" {
                    step_times.push((data.step, data.duration_ms));
                } else {
                    // Component timing
                    *component_totals.entry(data.component_name.clone()).or_insert(0.0) += data.duration_ms;
                    *component_counts.entry(data.component_name.clone()).or_insert(0) += 1;
                    
                    // Per-step component timing for trend analysis
                    step_component_times
                        .entry(data.step)
                        .or_insert_with(HashMap::new)
                        .insert(data.component_name.clone(), data.duration_ms);
                }
            }
        }
        
        // Overall simulation timing
        if !step_times.is_empty() {
            let total_simulation_time: f64 = step_times.iter().map(|(_, time)| time).sum();
            let avg_step_time = total_simulation_time / step_times.len() as f64;
            let max_step = step_times.iter().map(|(step, _)| *step).max().unwrap_or(0);
            
            report.push_str("🚀 SIMULATION OVERVIEW\n");
            report.push_str(&format!("Total steps completed: {}\n", step_times.len()));
            report.push_str(&format!("Total simulation time: {:.2} ms ({:.2} seconds)\n", 
                                   total_simulation_time, total_simulation_time / 1000.0));
            report.push_str(&format!("Average step time: {:.2} ms\n", avg_step_time));
            report.push_str(&format!("Steps per second: {:.1}\n", 1000.0 / avg_step_time));
            report.push_str(&format!("Highest step number: {}\n\n", max_step));
        }
        
        // Component analysis
        if self.component_analysis && !component_totals.is_empty() {
            report.push_str("⚡ COMPONENT PERFORMANCE ANALYSIS\n");
            
            let total_component_time: f64 = component_totals.values().sum();
            let mut components: Vec<_> = component_totals.iter().collect();
            components.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            for (name, total_time) in &components {
                let count = component_counts.get(*name).unwrap_or(&0);
                let avg_time = if *count > 0 { *total_time / *count as f64 } else { 0.0 };
                let percentage = if total_component_time > 0.0 { 
                    (*total_time / total_component_time) * 100.0 
                } else { 0.0 };
                
                report.push_str(&format!("  📈 {}: {:.2} ms total ({:.1}%) | {:.2} ms avg | {} calls\n", 
                                       name, total_time, percentage, avg_time, count));
            }
            report.push_str("\n");
        }
        
        // Performance insights and recommendations
        if self.component_analysis && !component_totals.is_empty() {
            report.push_str("💡 PERFORMANCE INSIGHTS\n");
            
            let components: Vec<_> = component_totals.iter().collect();
            if let Some((slowest_name, slowest_time)) = components.first() {
                let total_time: f64 = component_totals.values().sum();
                let percentage = (**slowest_time / total_time) * 100.0;
                report.push_str(&format!("🐌 Slowest component: {} ({:.1}% of total time)\n", 
                                       slowest_name, percentage));
                
                if percentage > 50.0 {
                    report.push_str("   ⚠️  Consider optimizing this component for better performance\n");
                }
            }
            
            // Check for performance balance
            let component_count = components.len();
            if component_count > 1 {
                let avg_component_time = component_totals.values().sum::<f64>() / component_count as f64;
                let max_time = component_totals.values().fold(0.0f64, |a, &b| a.max(b));
                let imbalance_ratio = max_time / avg_component_time;
                
                if imbalance_ratio > 3.0 {
                    report.push_str(&format!("⚖️  Performance imbalance detected (ratio: {:.1}:1)\n", imbalance_ratio));
                    report.push_str("   💡 Consider load balancing or parallel processing\n");
                }
            }
            
            report.push_str("\n");
        }
        
        // Trend analysis (if enabled and we have enough data)
        if self.trend_analysis && step_times.len() > 5 {
            report.push_str("📈 PERFORMANCE TRENDS\n");
            
            // Analyze step time trends
            let first_half_avg = step_times[..step_times.len()/2].iter()
                .map(|(_, time)| time).sum::<f64>() / (step_times.len()/2) as f64;
            let second_half_avg = step_times[step_times.len()/2..].iter()
                .map(|(_, time)| time).sum::<f64>() / (step_times.len() - step_times.len()/2) as f64;
            
            let trend_change = ((second_half_avg - first_half_avg) / first_half_avg) * 100.0;
            
            if trend_change.abs() > 5.0 {
                if trend_change > 0.0 {
                    report.push_str(&format!("📉 Performance degradation: {:.1}% slower in second half\n", trend_change));
                } else {
                    report.push_str(&format!("📈 Performance improvement: {:.1}% faster in second half\n", -trend_change));
                }
            } else {
                report.push_str("✅ Stable performance throughout simulation\n");
            }
            
            report.push_str("\n");
        }
        
        report.push_str("✅ Performance analysis complete!\n");
        report
    }
}

impl Component for MetricsReportingComponent {
    fn name(&self) -> &'static str {
        "MetricsReportingComponent"
    }
    
    fn initialize(&mut self, _coll_mgr: &mut CollectionsManager, _config: &SimulationConfig) {
        println!("📊 MetricsReportingComponent initialized - will report performance metrics at completion");
    }
    
    fn step(&self, _coll_mgr: &CollectionsManager, _actor: &mut Actor, _step: u32, _year: f64, _config: &SimulationConfig) {
        // This component doesn't do anything during steps - it only reports at the end
    }
    
    fn complete(&mut self, sim: &Simulation, _config: &SimulationConfig) {
        println!("\n📊 GENERATING PERFORMANCE REPORT...\n");
        
        let report = self.generate_performance_report(&sim.coll_mgr);
        println!("{}", report);
        
        // Additional summary for quick reference
        println!("🎯 QUICK PERFORMANCE SUMMARY:");
        println!("   Use this data to identify bottlenecks and optimize your simulation!");
        println!("   Focus optimization efforts on components with highest total time.");
    }
}

impl Default for MetricsReportingComponent {
    fn default() -> Self {
        Self::new()
    }
}
