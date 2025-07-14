# Materials Loader

This module provides functionality to load material properties from JSON data and retrieve phase properties by material name and phase.

## Overview

The materials loader system consists of several components:

1. **MaterialPhase struct** - Stores material properties as u32/u64 values for consistency with the codebase
2. **MaterialPhases enum** - Represents the three phases: Solid, Liquid, Gas
3. **MaterialsLoader** - Main loader that parses JSON and provides access functions
4. **MaterialUtils** - Utility functions for converting scaled values back to f64 for calculations

## Key Features

- **JSON Loading**: Loads materials from `src/material/materials.json` using the existing JsonParser
- **Type Safety**: Uses u32/u64 types for material properties to match codebase expectations
- **Enum-based Phases**: Uses MaterialPhases enum (Solid, Liquid, Gas) with string conversion
- **Caching**: Materials are cached in memory for efficient repeated access
- **Error Handling**: Comprehensive error handling for missing materials/phases
- **Scaling**: Handles fractional values by scaling (e.g., gas_interference_factor scaled by 1000)

## Usage Examples

### Basic Usage

```rust
use atmo_biosphere_rust::material::{MaterialsLoader, MaterialPhases, get_phase_properties_by_name};

// Method 1: Using enum directly
let basalt_solid = MaterialsLoader::get_phase_properties("basalt", MaterialPhases::Solid)?;
println!("Density: {} kg/m³", basalt_solid.density_kg_m3);

// Method 2: Using string (converted to enum internally)
let water_liquid = get_phase_properties_by_name("water", "liquid")?;
println!("Density: {} kg/m³", water_liquid.density_kg_m3);
```

### Getting Available Materials and Phases

```rust
// Get all material names
let materials = MaterialsLoader::get_material_names()?;

// Get available phases for a material (as enums)
let phases = MaterialsLoader::get_available_phases("steel")?;

// Get available phases as strings
let phase_names = MaterialsLoader::get_available_phase_names("steel")?;
```

### Enum Conversion

```rust
// Convert string to enum (case-insensitive)
let phase = MaterialPhases::from_str("SOLID"); // Some(MaterialPhases::Solid)

// Convert enum to string
let phase_str = MaterialPhases::Solid.as_str(); // "solid"

// Get all valid phase names
let all_phases = MaterialPhases::all_phase_names(); // ["solid", "liquid", "gas"]
```

## Data Structure

The JSON data is structured as:
```json
{
  "material_name": {
    "solid": { /* properties */ },
    "liquid": { /* properties */ },
    "gas": { /* properties */ },
    "emission_compounds": { /* compounds */ }
  }
}
```

## Property Scaling

Some properties are scaled to fit in u32 ranges while preserving precision:

- **gas_interference_factor**: Scaled by 1000 (0.8 → 800)
- **thermal_conduction_modifier**: Scaled by 1000 (0.9 → 900)  
- **thermal_expansivity**: Scaled by 1e9 (1e-05 → 10)
- **activation_volume_m3_per_mol**: Scaled by 1e9 (1e-05 → 10)

Use `MaterialUtils` functions to convert back to f64 for calculations:

```rust
use atmo_biosphere_rust::material::MaterialUtils;

let original_value = MaterialUtils::gas_interference_factor_as_f64(&phase); // Some(0.8)
```

## Available Materials

The current JSON includes:
- **basalt** - Volcanic rock (solid, liquid, gas phases)
- **granite** - Igneous rock (solid, liquid, gas phases)
- **silicate** - Silicate minerals (solid, liquid, gas phases)
- **steel** - Metal alloy (solid, liquid, gas phases)
- **water** - H2O (solid, liquid, gas phases)

## Function Reference

### MaterialsLoader

- `load_materials()` - Load all materials from JSON
- `get_phase_properties(material_name, phase)` - Get phase properties using enum
- `get_material_names()` - Get list of available material names
- `get_available_phases(material_name)` - Get available phases as enums
- `get_available_phase_names(material_name)` - Get available phases as strings
- `get_emission_compounds(material_name)` - Get emission compounds data
- `clear_cache()` - Clear the materials cache

### Convenience Functions

- `get_phase_properties_by_name(material_name, phase_name)` - Get properties using string phase name

### MaterialPhases Enum

- `as_str()` - Convert enum to string
- `from_str(s)` - Convert string to enum (case-insensitive)
- `all_phase_names()` - Get all valid phase names as strings
- `all_phases()` - Get all enum variants

## Error Handling

Functions return `Result<T, String>` with descriptive error messages:
- Material not found
- Phase not available for material
- Invalid phase name
- JSON parsing errors

## Testing

Run the materials loader tests:
```bash
cargo test materials_loader
```

Run the demo examples:
```bash
cargo run --example simple_materials_test
cargo run --example materials_demo
```
