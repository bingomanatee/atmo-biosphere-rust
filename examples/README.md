# Examples Directory

This directory contains organized examples demonstrating different aspects of the atmo-biosphere simulation system.

## 📁 Directory Structure

### 🧪 `poc_tests/` - Proof of Concept Tests
Simple, focused examples that demonstrate specific concepts or test individual components:

- **`simple_transaction_test.rs`** - Basic transaction system demonstration
- **`test_event_system_simple.rs`** - Simple event system testing
- **`standalone_event_system.rs`** - Isolated event system proof of concept
- **`geological_poc.rs`** - Geological simulation proof of concept

**Use these for:**
- Learning how individual components work
- Testing specific features in isolation
- Quick prototyping and experimentation

### 🌍 `full_simulations/` - Complete Use Cases
Full-featured examples that demonstrate complete simulation workflows:

- **`adaptive_hotspot_demo.rs`** - Complete hotspot simulation with adaptive behavior
- **`detailed_component_logging.rs`** - Full simulation with comprehensive logging
- **`embedded_event_tracking.rs`** - Complete simulation with event tracking
- **`event_emission_demo.rs`** - Full event emission and handling demonstration

**Use these for:**
- Understanding complete simulation workflows
- Seeing how components work together
- Production-ready simulation patterns
- Performance benchmarking

### 🛠️ `utilities/` - Reusable Tools
Utility examples and helper tools that can be reused across projects:

- **`reusable_performance_reporting.rs`** - Performance measurement and reporting utilities

**Use these for:**
- Performance analysis
- Debugging tools
- Reusable components
- Development utilities

### 🗂️ `deprecated/` - Legacy Examples
Older examples that are kept for reference but may use outdated patterns:

- Various legacy transaction and component examples
- Historical implementations
- Superseded approaches

**Note:** These examples may not compile with current code and are kept for reference only.

## 🚀 Getting Started

### For Beginners
Start with examples in `poc_tests/` to understand individual concepts:

```bash
# Run a simple proof of concept
cargo run --example simple_transaction_test

# Test basic event system
cargo run --example test_event_system_simple
```

### For Complete Simulations
Use examples in `full_simulations/` for production-ready patterns:

```bash
# Run a complete geological simulation
cargo run --example adaptive_hotspot_demo

# Run with detailed logging
cargo run --example detailed_component_logging
```

### For Performance Analysis
Use utilities for benchmarking and analysis:

```bash
# Run performance reporting
cargo run --example reusable_performance_reporting
```

## 📝 Example Naming Convention

- **`*_poc.rs`** - Proof of concept examples
- **`*_test.rs`** - Testing-focused examples
- **`*_demo.rs`** - Demonstration examples
- **`*_simple.rs`** - Simplified versions
- **`standalone_*.rs`** - Self-contained examples

## 🎯 Best Practices

1. **Start Simple**: Begin with POC tests to understand concepts
2. **Progress to Full**: Move to complete simulations for real use cases
3. **Use Utilities**: Leverage utility examples for development tools
4. **Check Deprecated**: Reference deprecated examples for historical context only

## 📚 Related Documentation

- See `src/test_geological_simulation.rs` for comprehensive test examples
- Check component documentation in `src/component/`
- Review simulation patterns in `src/sim/`

## 🔄 Immutable Paradigm Examples

The test suite in `src/test_geological_simulation.rs` contains cutting-edge examples of the new immutable paradigm:

- **Immutable constructor patterns**
- **Builder pattern for performance**
- **Functional programming approaches**
- **Performance comparisons with mutable systems**

These represent the future direction of the simulation system architecture.
