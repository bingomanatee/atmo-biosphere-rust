// Demonstration of event emission system for decoupled monitoring

use atmo_biosphere_rust::events::{
    EventEmitter, SimulationEvent, EventListener, 
    PerformanceListener, ConsoleListener, FileListener
};
use std::time::Duration;

/// Custom analytics listener
struct AnalyticsListener {
    transaction_count: usize,
    scaling_events: usize,
    component_adaptations: usize,
}

impl AnalyticsListener {
    fn new() -> Self {
        Self {
            transaction_count: 0,
            scaling_events: 0,
            component_adaptations: 0,
        }
    }
    
    fn get_analytics_report(&self) -> String {
        format!(
            "📈 Analytics Report\n\
             ==================\n\
             Transactions processed: {}\n\
             Scaling events: {}\n\
             Component adaptations: {}\n\
             Scaling rate: {:.1}%",
            self.transaction_count,
            self.scaling_events,
            self.component_adaptations,
            if self.transaction_count > 0 {
                (self.scaling_events as f64 / self.transaction_count as f64) * 100.0
            } else {
                0.0
            }
        )
    }
}

impl EventListener for AnalyticsListener {
    fn on_event(&mut self, event: &atmo_biosphere_rust::events::Event) {
        match &event.event {
            SimulationEvent::TransactionBatchProcessed { transaction_count, scaled_count, .. } => {
                self.transaction_count += transaction_count;
                self.scaling_events += scaled_count;
            },
            SimulationEvent::ComponentAdapted { .. } => {
                self.component_adaptations += 1;
            },
            _ => {}
        }
    }
}

fn main() {
    println!("📡 Event Emission System Demo");
    println!("=============================");
    println!("Demonstrates decoupled monitoring through events\n");

    // Create event emitter
    let mut emitter = EventEmitter::new();
    
    // Add various listeners
    emitter.add_listener(ConsoleListener::new(false)); // Non-verbose console output
    emitter.add_listener(PerformanceListener::new());
    emitter.add_listener(FileListener::new("simulation_events.log".to_string()));
    emitter.add_listener(AnalyticsListener::new());
    
    println!("🔧 Event system configured with {} listeners:", emitter.listener_count());
    println!("   • ConsoleListener: Real-time console output");
    println!("   • PerformanceListener: Performance tracking");
    println!("   • FileListener: Event logging to file");
    println!("   • AnalyticsListener: Custom analytics");

    // Simulate a geological simulation with events
    println!("\n🚀 Simulating geological simulation events...");
    
    // Simulation start
    emitter.emit(SimulationEvent::SimulationStarted {
        step_count: 5,
        years_per_step: 10000.0,
    });
    
    // Simulate 5 simulation steps
    for step in 0..5 {
        let year = step as f64 * 10000.0;
        
        // Step start
        emitter.emit_with_step(
            SimulationEvent::StepStarted { step, year },
            step
        );
        
        // Simulate component execution
        let components = ["thermal_conduction", "core_radiance", "convection_plumes"];
        
        for component in &components {
            // Component start
            emitter.emit_with_context(
                SimulationEvent::ComponentStarted {
                    component_name: component.to_string(),
                    step,
                    method_name: "step".to_string(),
                },
                step,
                component.to_string()
            );
            
            // Simulate some work
            std::thread::sleep(Duration::from_millis(10));
            
            // Component completion
            emitter.emit_with_context(
                SimulationEvent::ComponentCompleted {
                    component_name: component.to_string(),
                    step,
                    method_name: "step".to_string(),
                    duration: Duration::from_millis(10 + step as u64 * 2),
                },
                step,
                component.to_string()
            );
        }
        
        // Simulate transaction processing
        let transaction_count = 50 + step * 10;
        let scaled_count = if step > 2 { step * 3 } else { 0 }; // Scaling starts at step 3
        
        emitter.emit(SimulationEvent::TransactionBatchProcessed {
            transaction_count,
            scaled_count,
            duration: Duration::from_millis(5),
        });
        
        // Simulate scaling events
        if scaled_count > 0 {
            emitter.emit_with_component(
                SimulationEvent::TransactionScaled {
                    component_name: "core_radiance".to_string(),
                    scaling_factor: 0.75,
                    reason: "Hotspots overpowered".to_string(),
                },
                "core_radiance".to_string()
            );
            
            // Simulate component adaptation
            if step == 3 {
                emitter.emit_with_component(
                    SimulationEvent::ComponentAdapted {
                        component_name: "core_radiance".to_string(),
                        adaptation_type: "hotspot_redistribution".to_string(),
                        details: "Added 50% more hotspots, reduced energy by 33%".to_string(),
                    },
                    "core_radiance".to_string()
                );
            }
        }
        
        // Step completion
        emitter.emit_with_step(
            SimulationEvent::StepCompleted {
                step,
                year,
                duration: Duration::from_millis(50 + step as u64 * 10),
            },
            step
        );
        
        // Simulate some processing time
        std::thread::sleep(Duration::from_millis(20));
    }
    
    // Simulation end
    emitter.emit(SimulationEvent::SimulationEnded {
        total_steps: 5,
        total_duration: Duration::from_millis(400),
    });
    
    println!("\n📊 Event emission complete! Demonstrating decoupled access to data...");
    
    // The beauty of the event system: we can access listener data independently
    // without coupling to the simulation
    
    println!("\n🎯 Benefits of Event Emission System:");
    println!("=====================================");
    
    println!("✅ Decoupled Monitoring:");
    println!("   • Simulation doesn't know about listeners");
    println!("   • Listeners can be added/removed dynamically");
    println!("   • No performance impact when no listeners");
    
    println!("\n✅ Flexible Data Access:");
    println!("   • Multiple listeners can process same events");
    println!("   • Each listener maintains its own state");
    println!("   • Custom analytics without modifying simulation");
    
    println!("\n✅ Real-time Capabilities:");
    println!("   • Events emitted immediately when they occur");
    println!("   • No need to wait for simulation completion");
    println!("   • Live monitoring and alerting possible");
    
    println!("\n✅ Extensibility:");
    println!("   • Easy to add new event types");
    println!("   • Custom listeners for specific needs");
    println!("   • Event filtering and routing");
    
    println!("\n📈 Example Use Cases:");
    println!("   🔍 Real-time performance monitoring");
    println!("   📊 Live dashboards and visualizations");
    println!("   🚨 Alerting on performance issues");
    println!("   📝 Detailed audit logging");
    println!("   🧪 A/B testing different configurations");
    println!("   🎯 Custom analytics and reporting");
    
    println!("\n✅ Event Emission System Demo Complete!");
    println!("The timer is now completely decoupled from the simulation! 🎉");
}
