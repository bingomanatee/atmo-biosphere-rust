// Event types for simulation system

use std::time::Duration;

/// All possible simulation events
#[derive(Debug, Clone)]
pub enum SimulationEvent {
    // Simulation lifecycle events
    SimulationStarted {
        step_count: usize,
        years_per_step: f64,
    },
    SimulationEnded {
        total_steps: usize,
        total_duration: Duration,
    },
    
    // Step events
    StepStarted {
        step: i64,
        year: f64,
    },
    StepCompleted {
        step: i64,
        year: f64,
        duration: Duration,
    },
    
    // Component events
    ComponentStarted {
        component_name: String,
        step: i64,
        method_name: String,
    },
    ComponentCompleted {
        component_name: String,
        step: i64,
        method_name: String,
        duration: Duration,
    },
    
    // Transaction events
    TransactionProposed {
        component_name: String,
        source_cell: String,
        target_cell: Option<String>,
        energy_delta: f64,
        mass_delta: f64,
        description: String,
    },
    TransactionScaled {
        component_name: String,
        scaling_factor: f64,
        reason: String,
    },
    TransactionBatchProcessed {
        transaction_count: usize,
        scaled_count: usize,
        duration: Duration,
    },
    
    // Performance events
    PerformanceSnapshot {
        step: i64,
        component_timings: Vec<ComponentTiming>,
        total_simulation_time: Duration,
    },
    
    // System events
    MemoryUsage {
        step: i64,
        heap_bytes: usize,
        cell_count: usize,
    },
    
    // Error/Warning events
    Warning {
        component_name: String,
        message: String,
        step: i64,
    },
    Error {
        component_name: String,
        error: String,
        step: i64,
    },
    
    // Adaptation events
    ComponentAdapted {
        component_name: String,
        adaptation_type: String,
        details: String,
    },
}

/// Component timing information for performance events
#[derive(Debug, Clone)]
pub struct ComponentTiming {
    pub component_name: String,
    pub total_time: Duration,
    pub method_timings: Vec<MethodTiming>,
}

/// Method timing information
#[derive(Debug, Clone)]
pub struct MethodTiming {
    pub method_name: String,
    pub duration: Duration,
    pub call_count: usize,
}

/// Event metadata
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub timestamp: std::time::Instant,
    pub step: Option<i64>,
    pub component: Option<String>,
}

/// Complete event with metadata
#[derive(Debug, Clone)]
pub struct Event {
    pub event: SimulationEvent,
    pub metadata: EventMetadata,
}

impl Event {
    pub fn new(event: SimulationEvent) -> Self {
        Self {
            event,
            metadata: EventMetadata {
                timestamp: std::time::Instant::now(),
                step: None,
                component: None,
            },
        }
    }
    
    pub fn with_step(mut self, step: i64) -> Self {
        self.metadata.step = Some(step);
        self
    }
    
    pub fn with_component(mut self, component: String) -> Self {
        self.metadata.component = Some(component);
        self
    }
}
