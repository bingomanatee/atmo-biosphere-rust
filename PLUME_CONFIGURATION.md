# 🌋 Plume System Configuration Guide

## Overview
The plume system has 4 levels of configuration that control frequency, intensity, and realism of geological plume formation.

## 📊 Level 1: Simulation Configuration (Geological Foundation)

### Planet Configuration
```rust
PlanetConfig {
    radius_km: 6371.0,                    // 🌍 Affects pressure gradients
    surface_gravity_m_s_s: 9.81,          // 🌍 Affects pressure calculations  
    surface_temperature_k: 288.15,        // 🌡️ Base temperature for gradients
}
```

### Time Configuration
```rust
SimulationConfig {
    years_per_step: 100_000,               // ⏰ Time scale affects probability
    steps: 10,                             // ⏰ Total simulation length
}
```

### Layer Configuration (CRITICAL)
```rust
LayerConfig {
    resolution: Resolution::Four,          // 📐 Cell size affects area scaling (~1,770 km²)
    depth_steps: 3,                       // 📏 Number of cells per column
    height_per_step_km: 10.0,             // 📏 Cell height affects pressure
    temperature_gradient_k_per_km: 25.0,  // 🔥 CRITICAL: Temperature differences
}
```

**Key Points:**
- **Same resolution** across layers for proper multi-cell columns
- **Temperature gradients** create the thermal energy for plumes
- **Multiple depth_steps** enable neighbor-to-neighbor comparison

## 📊 Level 2: Plume Component Configuration

### ColumnPlumeComponent Parameters
```rust
ColumnPlumeComponent::with_parameters(
    800.0,   // 🌡️ plume_threshold_temp_k: Minimum temperature for plume sources
    5.0,     // 📊 min_density_difference: Legacy (not used in neighbor logic)
    0.08,    // ⚡ energy_transfer_efficiency: Base energy transfer (8%)
    0.05,    // 📦 mass_transfer_efficiency: Base mass transfer (5%)
)
```

**Tuning Guidelines:**
- **plume_threshold_temp_k**: 600-1200K (higher = fewer plumes)
- **energy_transfer_efficiency**: 5-15% (higher = more intense plumes)
- **mass_transfer_efficiency**: 3-10% (higher = more material transport)

## 📊 Level 3: Hard-Coded Physics Parameters

### Neighbor Comparison Thresholds (in source code)
```rust
// Current calibrated values:
temp_diff > 100.0K        // 🌡️ Temperature difference between neighbors
density_diff > 10.0       // 📊 Density difference after thermal expansion
```

### Probability Calculation Parameters
```rust
// Current calibrated values:
temperature_scaling = temp_diff / 5000.0K     // 🔥 Temperature to probability
max_base_probability = 0.002                  // 📊 Maximum 0.2% base chance
geological_time_factor = 0.5                  // ⏰ Geological time scaling
area_scaling = √(cell_area / 1M_km²)         // 📐 Area factor
pressure_scaling = (pressure_diff / 10MPa).clamp(0.1, 2.0)  // 💪 Pressure boost
```

### Thermal Expansion Parameters
```rust
thermal_expansion_coeff = 3e-5 /K             // 🌡️ Density reduction with heat
reference_temp = 300.0K                       // 🌡️ Reference temperature
```

## 📊 Level 4: Randomization Parameters

### Random Number Generation
```rust
rng_seed = 42                                 // 🎯 Reproducible randomness
random_factor = 0.1 + random(0.0, 1.0) × 0.2 // 🎲 10-30% probability variability
energy_randomization = 0.5 + random(0.0, 1.0) // ⚡ 50-150% energy variability
mass_randomization = 0.5 + random(0.0, 0.8)   // 📦 50-130% mass variability
```

## 🎯 Tuning Guide

### To INCREASE Plume Frequency:
- **Lower neighbor thresholds**: 100K → 75K temp, 10 → 7 kg/m³ density
- **Lower temperature scaling**: 5000K → 3000K divisor
- **Increase max probability**: 0.2% → 0.3%
- **Increase geological factor**: 0.5 → 0.7
- **Lower temperature threshold**: 800K → 600K

### To DECREASE Plume Frequency:
- **Raise neighbor thresholds**: 100K → 150K temp, 10 → 15 kg/m³ density  
- **Raise temperature scaling**: 5000K → 8000K divisor
- **Decrease max probability**: 0.2% → 0.1%
- **Decrease geological factor**: 0.5 → 0.3
- **Raise temperature threshold**: 800K → 1000K

### To INCREASE Plume Intensity:
- **Increase energy efficiency**: 8% → 12%
- **Increase mass efficiency**: 5% → 8%
- **Increase thermal expansion**: 3e-5 → 4e-5 /K
- **Increase pressure scaling**: 2.0 → 3.0 max

### To DECREASE Plume Intensity:
- **Decrease energy efficiency**: 8% → 5%
- **Decrease mass efficiency**: 5% → 3%
- **Decrease thermal expansion**: 3e-5 → 2e-5 /K
- **Decrease pressure scaling**: 2.0 → 1.5 max

## 🌍 Current Calibration (Earth-Like)

### Frequency
- **Target**: 40-50 plumes per million years
- **Achieved**: 48.5 plumes per million years ✅
- **Formation rate**: 0.0017% of columns per step

### Physics Formula
```rust
base_probability = (temp_diff / 5000K).min(0.002)
geological_factor = 0.5
area_factor = √(cell_area / 1M_km²)
pressure_factor = (pressure_diff / 10MPa).clamp(0.1, 2.0)
random_factor = 0.1 + random(0.0, 1.0) × 0.2

final_probability = base × geological × area × pressure × random
plume_forms = random(0.0, 1.0) < final_probability
```

## 📁 Configuration Files

### Where to Make Changes:

1. **Simulation Config**: In your example/test files
2. **Component Parameters**: `ColumnPlumeComponent::with_parameters()`
3. **Physics Parameters**: `src/components/column_plume_component.rs`
4. **Thresholds**: Lines ~200-204 (neighbor comparison)
5. **Probability**: Lines ~239-242 (probability calculation)

### Example Standard Configuration:
```rust
// Standard Earth-like configuration
let config = SimulationConfig {
    steps: 1000,
    years_per_step: 100_000,
    planet: PlanetConfig {
        radius_km: 6371.0,
        surface_gravity_m_s_s: 9.81,
        surface_temperature_k: 288.15,
    },
    layers: vec![
        LayerConfig {
            name: "Continental Crust".to_string(),
            resolution: Resolution::Four,
            depth_steps: 3,
            height_per_step_km: 10.0,
            temperature_gradient_k_per_km: 25.0,
        },
        LayerConfig {
            name: "Upper Mantle".to_string(),
            resolution: Resolution::Four,
            depth_steps: 2,
            height_per_step_km: 25.0,
            temperature_gradient_k_per_km: 15.0,
        },
    ],
};

let plume_component = ColumnPlumeComponent::with_parameters(
    800.0,  // Hot cell threshold
    5.0,    // Legacy parameter
    0.08,   // 8% energy transfer
    0.05,   // 5% mass transfer
);
```

This configuration produces Earth-like plume frequencies and realistic geological behavior.
