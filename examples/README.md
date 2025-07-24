# Examples Directory

This directory contains organized examples demonstrating different aspects of the atmo-biosphere simulation system.

## 📁 Directory Structure

### 🧪 `poc_tests/` - Proof of Concept Tests
Simple, focused examples that demonstrate specific concepts or test individual components:

- **`test_event_system_simple.rs`** - Simple event system testing
- **`standalone_event_system.rs`** - Isolated event system proof of concept

**Use these for:**
- Learning how individual components work
- Testing specific features in isolation
- Quick prototyping and experimentation

### 🌍 Current Examples - Immutable System
Examples using the current immutable simulation system:

- **`geological_poc_immutable.rs`** - Geological simulation using immutable patterns
- **`immutable_simulation_test.rs`** - Basic immutable simulation testing

**Use these for:**
- Understanding the current immutable simulation architecture
- Learning modern simulation patterns
- Production-ready immutable simulation workflows

### 🗂️ `deprecated/` - Legacy Examples
Older examples that are kept for reference but may use outdated patterns:

- **Old mutable simulation examples** - `geological_poc.rs`, `adaptive_hotspot_demo.rs`, etc.
- **Legacy transaction examples** - Various transaction system implementations
- **Historical component examples** - Superseded component patterns
- **Utility examples** - Old performance and debugging tools

**Note:** These examples use the deprecated mutable simulation system and may not compile with current code. They are kept for reference only.

## 🚀 Getting Started

### For Beginners
Start with simple examples to understand individual concepts:

```bash
# Test basic event system
cargo run --example test_event_system_simple

# Test standalone event system
cargo run --example standalone_event_system
```

### For Current Simulation System
Use the immutable simulation examples:

```bash
# Run immutable geological simulation
cargo run --example geological_poc_immutable

# Test immutable simulation patterns
cargo run --example immutable_simulation_test
```

## 📝 Example Naming Convention

- **`*_immutable.rs`** - Immutable simulation examples (current)
- **`*_test.rs`** - Testing-focused examples
- **`*_simple.rs`** - Simplified versions
- **`standalone_*.rs`** - Self-contained examples

## 🎯 Best Practices

1. **Use Immutable System**: All new development should use the immutable simulation system
2. **Start Simple**: Begin with basic examples to understand concepts
3. **Check Tests**: Review `src/test_geological_simulation.rs` for advanced patterns
4. **Avoid Deprecated**: Do not use deprecated examples for new development

## 📚 Related Documentation

- See `src/test_geological_simulation.rs` for comprehensive test examples
- Check component documentation in `src/component/`
- Review immutable simulation patterns in `src/sim_immut/`
- See `deprecated/src/README.md` for information about deprecated code

## 🔄 Immutable Paradigm Examples

The test suite in `src/test_geological_simulation.rs` contains cutting-edge examples of the new immutable paradigm:

- **Immutable constructor patterns**
- **Builder pattern for performance**
- **Functional programming approaches**
- **Performance comparisons with mutable systems**

These represent the future direction of the simulation system architecture.
