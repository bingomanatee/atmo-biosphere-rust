pub mod component_profiler;

pub use component_profiler::{ComponentProfiler, MethodMetrics, ComponentMetrics};

/// Re-export the timing macro for easy use
pub use crate::time_method;
