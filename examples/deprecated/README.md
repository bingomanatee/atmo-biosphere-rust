# 🗂️ Deprecated Examples

**⚠️ WARNING: These examples may not compile with the current codebase.**

This folder contains legacy examples that are kept for historical reference but may use outdated patterns or APIs that have been superseded.

## Why These Examples Are Deprecated

- **Outdated APIs** - Use old function signatures or module structures
- **Superseded patterns** - Replaced by better approaches in newer examples
- **Compilation issues** - May not work with current dependencies
- **Performance issues** - Replaced by more efficient implementations

## Examples in this folder:

- `component_transaction_integration.rs` - Old component integration patterns
- `flat_transaction_system.rs` - Early transaction system design
- `standalone_transaction_test.rs` - Legacy transaction testing
- `test_3d_transaction_system.rs` - Old 3D transaction approach
- `test_mass_conservation.rs` - Early mass conservation testing
- `test_transaction_manager.rs` - Legacy transaction manager tests
- `test_transaction_manager_unit.rs` - Old unit testing patterns
- `test_transaction_system_trial.rs` - Early transaction system trials
- `transaction_merging_demo.rs` - Old transaction merging approach

## 🔄 Modern Alternatives

Instead of these deprecated examples, use:

### For POC Testing:
- `../poc_tests/simple_transaction_test.rs` - Modern transaction testing
- `../poc_tests/geological_poc.rs` - Current geological simulation patterns

### For Full Simulations:
- `../full_simulations/adaptive_hotspot_demo.rs` - Complete modern simulation
- `../full_simulations/detailed_component_logging.rs` - Current logging patterns

### For Advanced Patterns:
- `src/test_geological_simulation.rs` - Cutting-edge immutable paradigm examples
- Performance comparison tests with builder patterns
- Immutable constructor patterns

## 📚 Historical Value

These examples are kept because they:
- Show the evolution of the codebase
- Demonstrate lessons learned
- Provide context for design decisions
- May contain useful concepts for future development

## ⚠️ Usage Warning

**Do not use these examples as starting points for new development.**

They are provided for reference only and may:
- Fail to compile
- Use deprecated APIs
- Demonstrate anti-patterns
- Have security or performance issues

For current best practices, see the examples in `../poc_tests/`, `../full_simulations/`, and the test suite in `src/test_geological_simulation.rs`.
