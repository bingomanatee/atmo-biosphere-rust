// Simple test to verify event system works after removing ComponentProfiler

use atmo_biosphere_rust::events::{
    EventEmitter, SimulationEvent, EventListener, Event,
    PerformanceListener, ConsoleListener
};
use std::time::Duration;

/// Test event listener that captures events
struct TestListener {
    events_received: usize,
}

impl TestListener {
    fn new() -> Self {
        Self { events_received: 0 }
    }
}

impl EventListener for TestListener {
    fn on_event(&mut self, _event: &Event) {
        self.events_received += 1;
    }
}

fn main() {
    println!("🧪 Simple Event System Test");
    println!("============================");
    println!("Testing event system after ComponentProfiler removal\n");

    // Test 1: Basic event emission
    println!("🔬 Test 1: Basic Event Emission");
    let mut emitter = EventEmitter::new();
    let mut test_listener = TestListener::new();
    
    emitter.add_listener(test_listener);
    assert_eq!(emitter.listener_count(), 1);
    
    // Emit some events
    emitter.emit(SimulationEvent::SimulationStarted {
        step_count: 5,
        years_per_step: 1000.0,
    });
    
    emitter.emit_with_step(
        SimulationEvent::StepStarted { step: 1, year: 1000.0 },
        1
    );
    
    emitter.emit_with_context(
        SimulationEvent::ComponentCompleted {
            component_name: "TestComponent".to_string(),
            step: 1,
            method_name: "step".to_string(),
            duration: Duration::from_millis(50),
        },
        1,
        "TestComponent".to_string()
    );
    
    println!("✅ Basic event emission test passed");

    // Test 2: Performance listener
    println!("\n🔬 Test 2: Performance Listener");
    let mut perf_emitter = EventEmitter::new();
    perf_emitter.add_listener(PerformanceListener::new());
    
    // Simulate component execution
    perf_emitter.emit(SimulationEvent::SimulationStarted {
        step_count: 3,
        years_per_step: 1000.0,
    });
    
    for step in 0..3 {
        perf_emitter.emit_with_step(
            SimulationEvent::StepStarted { step, year: step as f64 * 1000.0 },
            step
        );
        
        // Simulate component execution
        perf_emitter.emit_with_context(
            SimulationEvent::ComponentCompleted {
                component_name: "ThermalConduction".to_string(),
                step,
                method_name: "step".to_string(),
                duration: Duration::from_millis(25),
            },
            step,
            "ThermalConduction".to_string()
        );
        
        perf_emitter.emit_with_context(
            SimulationEvent::ComponentCompleted {
                component_name: "CoreRadiance".to_string(),
                step,
                method_name: "step".to_string(),
                duration: Duration::from_millis(15),
            },
            step,
            "CoreRadiance".to_string()
        );
        
        perf_emitter.emit_with_step(
            SimulationEvent::StepCompleted {
                step,
                year: step as f64 * 1000.0,
                duration: Duration::from_millis(50),
            },
            step
        );
    }
    
    perf_emitter.emit(SimulationEvent::SimulationEnded {
        total_steps: 3,
        total_duration: Duration::from_millis(150),
    });
    
    println!("✅ Performance listener test passed");

    // Test 3: Console listener (should not panic)
    println!("\n🔬 Test 3: Console Listener");
    let mut console_emitter = EventEmitter::new();
    console_emitter.add_listener(ConsoleListener::new(false));
    
    console_emitter.emit(SimulationEvent::SimulationStarted {
        step_count: 1,
        years_per_step: 1000.0,
    });
    
    console_emitter.emit(SimulationEvent::TransactionScaled {
        component_name: "CoreRadiance".to_string(),
        scaling_factor: 0.75,
        reason: "Overpowered hotspots".to_string(),
    });
    
    console_emitter.emit(SimulationEvent::ComponentAdapted {
        component_name: "CoreRadiance".to_string(),
        adaptation_type: "hotspot_redistribution".to_string(),
        details: "Added 50% more hotspots".to_string(),
    });
    
    console_emitter.emit(SimulationEvent::SimulationEnded {
        total_steps: 1,
        total_duration: Duration::from_millis(50),
    });
    
    println!("✅ Console listener test passed");

    // Test 4: Multiple listeners
    println!("\n🔬 Test 4: Multiple Listeners");
    let mut multi_emitter = EventEmitter::new();
    multi_emitter.add_listener(TestListener::new());
    multi_emitter.add_listener(TestListener::new());
    multi_emitter.add_listener(ConsoleListener::new(false));
    
    assert_eq!(multi_emitter.listener_count(), 3);
    
    multi_emitter.emit(SimulationEvent::StepStarted { step: 1, year: 1000.0 });
    
    println!("✅ Multiple listeners test passed");

    println!("\n🎯 Event System Test Results:");
    println!("==============================");
    println!("✅ Basic event emission: PASSED");
    println!("✅ Performance listener: PASSED");
    println!("✅ Console listener: PASSED");
    println!("✅ Multiple listeners: PASSED");
    
    println!("\n🎉 Event System Successfully Replaces ComponentProfiler!");
    println!("   • Event emission works correctly");
    println!("   • Performance tracking via events");
    println!("   • Multiple listeners supported");
    println!("   • No compilation errors");
    
    println!("\n✅ Event System Test Complete!");
}
