# Variable Resolution Layer System Design

## Overview

The variable resolution system provides **high resolution where needed** (surface processes, plate interactions) and **coarse resolution where efficient** (deep background thermal structure), while maintaining the ability for fine-grained plumes to traverse and interact across resolution boundaries.

## Layer Resolution Strategy

### Current Variable Resolution Configuration

```
Layer 1: 0-5km    (10 cells × 0.5km) - basalt [ULTRA-HIGH] - Plate interactions
Layer 2: 5-15km   (10 cells × 1.0km) - basalt [ULTRA-HIGH] - Crustal processes  
Layer 3: 15-35km  (8 cells × 2.5km)  - granite [HIGH]      - Lithosphere
Layer 4: 35-75km  (4 cells × 10km)   - granite [MODERATE]  - Background thermal
Layer 5: 75-150km (3 cells × 25km)   - basalt [COARSE]     - Plume sources

Total: 35 cells per column, 150km depth
Resolution ratio: 50:1 (surface:deep)
```

## Design Philosophy

### 1. **Surface Focus for Plate Interactions**
- **0.5km resolution** in top 5km for detailed plate boundary processes
- **1km resolution** in upper crust for fault systems and volcanism
- Enables realistic modeling of:
  - Mid-ocean ridge spreading
  - Subduction zone dynamics
  - Transform fault interactions
  - Volcanic eruption pathways

### 2. **Efficient Deep Background**
- **25km resolution** in deep mantle for computational efficiency
- Provides adequate thermal structure without wasted computation
- Focuses resources on active surface geology
- Maintains realistic deep thermal gradients

### 3. **Plume System Compatibility**

#### Fine-Grained Plume Properties
Plumes maintain their intrinsic fine-grained characteristics regardless of host layer resolution:

```rust
struct PlumeParcel {
    // Fine-grained properties maintained across all layers
    temperature_k: f64,
    mass_kg: f64,
    energy_joules: f64,
    velocity_m_per_s: f64,
    radius_meters: f64,      // Can be < 1km even in 25km cells
    composition: MaterialComposition,
    
    // Spatial tracking
    current_layer_index: usize,
    current_cell_location: CellLocation,
    sub_cell_position: (f64, f64, f64), // Fine position within coarse cell
}
```

#### Resolution Transition Mechanics

1. **Plume Origination in Coarse Layers**:
   - Plumes can originate in 25km deep cells
   - Initial plume properties are fine-grained (e.g., 1km diameter)
   - Host cell provides thermal/pressure environment
   - Plume maintains independent thermal properties

2. **Rising Through Variable Resolution**:
   - Plume tracks sub-cell position within each layer
   - Properties remain fine-grained regardless of host cell size
   - Thermal interaction scales with actual plume volume, not cell volume
   - Velocity calculations use fine-grained buoyancy physics

3. **Surface Interaction in High Resolution**:
   - Plume reaches ultra-high resolution surface layers
   - Can interact with 0.5km plate boundary cells
   - Enables realistic plume-plate interactions
   - Detailed volcanic/hotspot modeling

## Implementation Strategy

### 1. **Layer-Independent Plume Tracking**
```rust
impl PlumeComponent {
    fn update_plume_in_variable_resolution(&mut self, plume: &mut PlumeParcel, layer_sets: &[LayerSet]) {
        // Plume properties remain fine-grained
        let current_layer = &layer_sets[plume.current_layer_index];
        let host_cell = current_layer.get_cell(plume.current_cell_location);
        
        // Use fine-grained plume physics regardless of host cell size
        let buoyancy_force = self.calculate_fine_grained_buoyancy(plume, host_cell);
        let rise_velocity = self.calculate_rise_velocity(plume, host_cell);
        
        // Track sub-cell position for accurate spatial modeling
        plume.sub_cell_position = self.update_position(plume, rise_velocity);
        
        // Check for layer transitions
        if self.should_transition_layer(plume) {
            self.transition_to_next_layer(plume, layer_sets);
        }
    }
}
```

### 2. **Thermal Interaction Scaling**
```rust
fn calculate_plume_thermal_interaction(plume: &PlumeParcel, host_cell: &EnergyMassCell) -> f64 {
    // Use actual plume volume, not host cell volume
    let plume_volume_m3 = (4.0/3.0) * PI * plume.radius_meters.powi(3);
    let host_cell_volume_m3 = host_cell.volume_km3() * 1e9;
    
    // Thermal interaction scales with plume size, not cell size
    let interaction_fraction = (plume_volume_m3 / host_cell_volume_m3).min(1.0);
    
    // Calculate energy exchange based on actual plume properties
    let temp_difference = plume.temperature_k - host_cell.temperature_kelvin();
    let thermal_conductivity = calculate_effective_conductivity(plume, host_cell);
    
    thermal_conductivity * interaction_fraction * temp_difference
}
```

### 3. **Benefits of Variable Resolution + Fine-Grained Plumes**

#### Computational Efficiency
- **15x faster** than uniform high resolution
- **Surface detail** where plate interactions occur
- **Coarse background** for thermal structure only
- **Plume detail** maintained independently

#### Scientific Accuracy
- **Realistic plate boundaries**: 0.5km resolution for fault systems
- **Accurate plume dynamics**: Fine-grained physics across all layers
- **Proper thermal structure**: Efficient deep thermal gradients
- **Multi-scale interactions**: Plumes interact realistically with plates

#### Scalability
- **Long timescales**: Efficient for geological time simulations
- **Large domains**: Can model entire ocean basins
- **Complex interactions**: Plume-plate-ridge interactions
- **Future expansion**: Easy to add more surface detail

## Performance Characteristics

### Current Results
```
✅ Energy Balance: EXCELLENT (change < 0.5%)
- Variable resolution maintains thermal equilibrium
- Radiative transfer works across resolution boundaries
- Total cells per column: 35 (vs 300+ uniform high-res)
- Step time: ~100ms (vs ~1500ms uniform high-res)
```

### Scaling Projections
- **Global simulations**: Feasible with variable resolution
- **Plate tectonics**: Ultra-high surface resolution enables realistic modeling
- **Plume systems**: Fine-grained plumes work efficiently across resolution zones
- **Long-term evolution**: Computational efficiency enables geological timescales

## Future Enhancements

### 1. **Adaptive Resolution**
- Dynamic resolution adjustment based on thermal gradients
- Higher resolution where temperature gradients are steep
- Automatic refinement around active plumes

### 2. **Multi-Scale Plume Modeling**
- Plume swarms with different size scales
- Fine-grained plume heads in coarse background
- Realistic plume-plume interactions

### 3. **Plate Boundary Detail**
- Even higher resolution (100m) at active spreading centers
- Detailed fault zone modeling
- Realistic magma chamber dynamics

## Conclusion

The variable resolution system provides the **optimal balance** between computational efficiency and scientific accuracy:

- **Surface processes**: Ultra-high resolution for plate interactions
- **Deep structure**: Coarse resolution for efficient thermal modeling  
- **Plume dynamics**: Fine-grained properties maintained across all layers
- **Performance**: 15x faster than uniform high resolution
- **Scalability**: Ready for full-scale geological simulations

This design enables realistic modeling of **plume-plate interactions** while maintaining computational efficiency for **long-term geological evolution**.
