# Examples

This directory contains **current, working examples** for the atmo-biosphere simulation system.

## 🎯 **CURRENT EXAMPLES TO RUN:**

### **Main Validation Example**
```bash
cargo run --example comprehensive_poc_validation
```
**THE PRIMARY EXAMPLE** - Complete system validation with:
- ✅ Scientifically accurate radiative transfer
- ✅ 5+5+5 layer structure (15 cells, 165km depth)
- ✅ Cell-by-cell thermal analysis table
- ✅ Energy balance validation
- ✅ Performance metrics
- ✅ Ready for heat input components

### **Basic Testing Example**
```bash
cargo run --example geological_poc_immutable
```
Simple test of the immutable simulation system with correct layer structure and core radiance.

### **Utility Examples**
```bash
cargo run --example h3_resolution_check
```
Check H3 resolution information and cell dimensions.

## 📁 **File Organization**

- **`examples/`** - Current, working examples only
- **`examples/deprecated/`** - Outdated examples moved here aggressively
- **`examples/poc_tests/`** - Specialized POC tests

## 🔄 **Maintenance Policy**

Examples are **aggressively moved to deprecated** to maintain focus:
- Only current, correct examples stay in main folder
- Outdated layer structures → deprecated
- Superseded approaches → deprecated
- Minor test files → deleted or deprecated

**Always run `comprehensive_poc_validation` first** - it's the gold standard validation.

## 🚀 **Quick Start**

1. **Start here**: `cargo run --example comprehensive_poc_validation`
2. **Basic testing**: `cargo run --example immutable_simulation_test`
3. **Check H3 info**: `cargo run --example h3_resolution_check`

The comprehensive POC shows everything working together with the correct 5+5+5 layer structure and validates all of today's improvements.
