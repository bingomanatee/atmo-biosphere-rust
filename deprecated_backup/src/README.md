# Deprecated Source Code

This directory contains the old mutable simulation system that has been replaced by the immutable system in `src/sim_immut/`.

## 🚫 Deprecated Files

### Core Simulation System (Mutable)
- `sim/simulation.rs` - Old mutable simulation engine
- `sim/layer_set.rs` - Old mutable layer set implementation  
- `sim/energy_mass_cell.rs` - Old mutable energy/mass cell implementation
- `sim/energy_mass_cell_conductivity.rs` - Old conductivity calculations
- `sim/energy_mass_cell_immut.rs` - Duplicate immutable cell (moved from src/sim/)
- `sim/layer_set_immut.rs` - Duplicate immutable layer set (moved from src/sim/)
- `sim/comductivity.md` - Old conductivity documentation

## 🔄 Modern Replacements

Instead of these deprecated files, use:

### For Simulation:
- `src/sim_immut/simulation_immut.rs` - New immutable simulation engine
- `src/sim_immut/layer_set_immut.rs` - New immutable layer sets
- `src/sim_immut/energy_mass_cell_immut.rs` - New immutable energy/mass cells

### For Transactions:
- `src/sim/transaction_manager.rs` - Still active (used by immutable system)

## 📚 Historical Context

The mutable simulation system was the original implementation that:
- Used in-place mutation of cells and layer sets
- Had performance issues with large simulations
- Required complex state management
- Made debugging difficult due to side effects

The new immutable system provides:
- Better performance through constructor patterns
- Easier debugging and testing
- Functional programming benefits
- Cleaner transaction handling

## ⚠️ Usage Warning

**Do not use these files for new development.**

They are kept for historical reference only and may:
- Fail to compile with current dependencies
- Have unresolved bugs or performance issues
- Use deprecated APIs
- Conflict with the new immutable system

For current best practices, see:
- `src/sim_immut/` - Current immutable simulation system
- `src/test_geological_simulation.rs` - Modern test patterns
- `examples/geological_poc_immutable.rs` - Current example patterns
