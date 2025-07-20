// Event emitter for simulation system

use super::event_types::*;
use super::event_listener::EventListener;
use std::sync::{Arc, Mutex};

/// Thread-safe event emitter
pub struct EventEmitter {
    listeners: Vec<Arc<Mutex<dyn EventListener + Send>>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
    
    /// Add an event listener
    pub fn add_listener<L: EventListener + Send + 'static>(&mut self, listener: L) {
        self.listeners.push(Arc::new(Mutex::new(listener)));
    }
    
    /// Emit an event to all listeners
    pub fn emit(&self, event: SimulationEvent) {
        let event = Event::new(event);
        
        for listener in &self.listeners {
            if let Ok(mut listener) = listener.lock() {
                listener.on_event(&event);
            }
        }
    }
    
    /// Emit an event with step context
    pub fn emit_with_step(&self, event: SimulationEvent, step: i64) {
        let event = Event::new(event).with_step(step);
        
        for listener in &self.listeners {
            if let Ok(mut listener) = listener.lock() {
                listener.on_event(&event);
            }
        }
    }
    
    /// Emit an event with component context
    pub fn emit_with_component(&self, event: SimulationEvent, component: String) {
        let event = Event::new(event).with_component(component);
        
        for listener in &self.listeners {
            if let Ok(mut listener) = listener.lock() {
                listener.on_event(&event);
            }
        }
    }
    
    /// Emit an event with full context
    pub fn emit_with_context(&self, event: SimulationEvent, step: i64, component: String) {
        let event = Event::new(event).with_step(step).with_component(component);
        
        for listener in &self.listeners {
            if let Ok(mut listener) = listener.lock() {
                listener.on_event(&event);
            }
        }
    }
    
    /// Get number of registered listeners
    pub fn listener_count(&self) -> usize {
        self.listeners.len()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience macros for emitting events
#[macro_export]
macro_rules! emit_event {
    ($emitter:expr, $event:expr) => {
        $emitter.emit($event);
    };

    ($emitter:expr, $event:expr, step: $step:expr) => {
        $emitter.emit_with_step($event, $step);
    };

    ($emitter:expr, $event:expr, component: $component:expr) => {
        $emitter.emit_with_component($event, $component.to_string());
    };

    ($emitter:expr, $event:expr, step: $step:expr, component: $component:expr) => {
        $emitter.emit_with_context($event, $step, $component.to_string());
    };
}

/// Macro for timing methods with automatic event emission
#[macro_export]
macro_rules! time_method_with_events {
    ($emitter:expr, $component:expr, $method:expr, $step:expr, $code:block) => {
        {
            use crate::events::SimulationEvent;

            // Emit method started event
            $emitter.emit_with_context(
                SimulationEvent::MethodStarted {
                    component_name: $component.to_string(),
                    method_name: $method.to_string(),
                    step: $step,
                },
                $step,
                $component.to_string()
            );

            let start = std::time::Instant::now();
            let result = $code;
            let duration = start.elapsed();

            // Emit method completed event
            $emitter.emit_with_context(
                SimulationEvent::MethodCompleted {
                    component_name: $component.to_string(),
                    method_name: $method.to_string(),
                    step: $step,
                    duration,
                },
                $step,
                $component.to_string()
            );

            result
        }
    };
}
