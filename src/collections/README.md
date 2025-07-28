# Collections System

A concurrent, lock-free collections system built on `crossbeam` and `dashmap` for high-performance geological simulations.

## Architecture Overview

The collections system provides **100% memory-resident**, **thread-safe** access to shared data structures with **lock-free concurrent operations**.

### Core Components

- **`CollectionsManager`** - Central registry for all collections
- **`Collection<K, T>`** - Generic concurrent collection using `DashMap`
- **`CollectionChange` trait** - Changes that know how to apply themselves
- **`ChangeFactory`** - Generic factory for creating change objects

## Thread-Safe Event-Driven Pattern (Recommended)

The collections system supports a **thread-safe event-driven pattern** where components emit events instead of directly modifying data. This enables:

- **Parallel component processing** - Multiple components run concurrently
- **Event compression** - Multiple `AddToField` events for the same field are compressed
- **Atomic application** - All events applied together atomically
- **No direct mutations** - Components never modify data directly

### Event-Driven Usage

```rust
// 1. Get thread-safe event emitter
let emitter = manager.get_event_emitter();

// 2. Components emit events (thread-safe)
crossbeam::scope(|s| {
    let emitter1 = emitter.clone();
    let emitter2 = emitter.clone();

    // Component 1: Thermal processing
    s.spawn(|_| {
        for entry in cells.iter() {
            let temp_delta = calculate_heating(entry.value());
            emitter1.add_to_field("GEOLOGICAL_CELLS", *entry.key(), "temperature_k", temp_delta);
        }
    });

    // Component 2: Pressure processing
    s.spawn(|_| {
        for entry in cells.iter() {
            let pressure_delta = calculate_pressure(entry.value());
            emitter2.add_to_field("GEOLOGICAL_CELLS", *entry.key(), "pressure_pa", pressure_delta);
        }
    });
});

// 3. Apply all events atomically (with compression)
manager.apply_pending_events().unwrap();
```

### Event Types

- **`add_to_field(collection, key, field, delta)`** - Add delta to field (compressible)
- **`set_field(collection, key, field, value)`** - Set field to absolute value
- **`delete_cell(collection, key)`** - Remove cell

### Event Compression

Multiple `AddToField` events for the same cell+field are automatically compressed:

```rust
// These three events:
emitter.add_to_field("GEOLOGICAL_CELLS", location, "temperature_k", 5.0);
emitter.add_to_field("GEOLOGICAL_CELLS", location, "temperature_k", 3.0);
emitter.add_to_field("GEOLOGICAL_CELLS", location, "temperature_k", 2.0);

// Become one event:
// AddToField { location, field: "temperature_k", delta: 10.0 }
```

## Direct Access Patterns (Legacy)

### 1. Collection Registration

Collections must be registered before use:

```rust
let mut manager = CollectionsManager::new();

// Register collections with specific key/value types
manager.add_empty_collection::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS");
manager.add_empty_collection::<u32, ComponentData>("COMPONENTS");
```

### 2. Direct Collection Access

Get type-safe references to collections:

```rust
// Read-only access
let cells = manager.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();

// Check if cell exists
if cells.contains_key(&cell_location) {
    // Cell exists
}

// Get cell data (returns Option<Ref<K, T>>)
if let Some(cell_data) = cells.get(&cell_location) {
    println!("Temperature: {}K", cell_data.temperature_k);
    // Ref is automatically dropped here
}
```

### 3. Concurrent Iteration

Iterate over collections safely across threads:

```rust
// Iterate over all cells
for entry in cells.iter() {
    let (location, data) = (entry.key(), entry.value());
    println!("Cell at {:?} has temp {}K", location, data.temperature_k);
}

// Filter and process
for entry in cells.iter() {
    let (location, data) = (entry.key(), entry.value());
    if data.temperature_k > 1000.0 {
        // Process hot cells
    }
}
```

### 4. Direct Mutations

Modify data directly (lock-free):

```rust
// Insert new data
cells.insert(cell_location, geological_data);

// Remove data
cells.remove(&cell_location);

// Atomic modify-in-place
cells.modify(&cell_location, |cell_data| {
    cell_data.temperature_k += 10.0;
    cell_data.pressure_pa *= 1.1;
});
```

### 5. Change-Based Updates

Use the change system for complex updates:

```rust
// Create changes
let changes = vec![
    Box::new(GeologicalChange::UpdateTemperature { 
        location: cell_location, 
        new_temp: 350.0 
    }) as Box<dyn CollectionChange>,
    Box::new(GeologicalChange::AddEnergy { 
        location: cell_location, 
        energy_delta: 1000.0 
    }) as Box<dyn CollectionChange>,
];

// Apply atomically
manager.apply_changes(changes).unwrap();
```

## Thread-Safe Component Architecture

### Component Design Pattern

Components should **never modify data directly**. Instead, they emit events:

```rust
struct ThermalComponent;

impl ThermalComponent {
    // ✅ GOOD: Emit events, no direct modification
    fn process(&self, sim: &Simulation, emitter: &EventEmitter) {
        let cells = sim.get_geological_cells();

        for entry in cells.iter() {
            let temp_delta = calculate_thermal_change(entry.value());

            // EMIT EVENT instead of direct modification
            emitter.add_to_field("GEOLOGICAL_CELLS", *entry.key(), "temperature_k", temp_delta);
        }
    }

    // ❌ BAD: Direct modification (not thread-safe)
    fn process_bad(&self, cells: &Collection<CellLocation, GeologicalCellData>) {
        for entry in cells.iter() {
            cells.modify(entry.key(), |data| {
                data.temperature_k += 10.0; // DON'T DO THIS
            });
        }
    }
}
```

### Simulation Step Pattern

```rust
impl Simulation {
    pub fn step(&mut self) {
        let emitter = self.coll_mgr.get_event_emitter();

        // 1. Process all components in parallel
        crossbeam::scope(|s| {
            let emitter1 = emitter.clone();
            let emitter2 = emitter.clone();
            let emitter3 = emitter.clone();

            s.spawn(|_| self.thermal_component.process(self, &emitter1));
            s.spawn(|_| self.pressure_component.process(self, &emitter2));
            s.spawn(|_| self.density_component.process(self, &emitter3));
        }).unwrap();

        // 2. Apply all events atomically
        self.coll_mgr.apply_pending_events().unwrap();

        self.current_step += 1;
    }
}
```

## Legacy Parallel Processing Patterns

### 1. Shared Read Access (Direct)

Multiple threads can read simultaneously:

```rust
use std::sync::Arc;

let manager = Arc::new(manager);

crossbeam::scope(|s| {
    // Component 1: Temperature processor
    let manager1 = Arc::clone(&manager);
    s.spawn(move |_| {
        let cells = manager1.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        for entry in cells.iter() {
            // Process temperatures
        }
    });
    
    // Component 2: Pressure processor  
    let manager2 = Arc::clone(&manager);
    s.spawn(move |_| {
        let cells = manager2.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        for entry in cells.iter() {
            // Process pressures
        }
    });
});
```

### 2. Concurrent Modifications

Multiple threads can modify different cells simultaneously:

```rust
crossbeam::scope(|s| {
    // Thread 1: Process layer 0
    s.spawn(|_| {
        let cells = manager.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        for entry in cells.iter() {
            if entry.key().layer_set_index() == 0 {
                cells.modify(entry.key(), |data| {
                    data.temperature_k += surface_heating;
                });
            }
        }
    });
    
    // Thread 2: Process layer 1
    s.spawn(|_| {
        let cells = manager.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
        for entry in cells.iter() {
            if entry.key().layer_set_index() == 1 {
                cells.modify(entry.key(), |data| {
                    data.pressure_pa += mantle_pressure;
                });
            }
        }
    });
});
```

### 3. Producer-Consumer Pattern

Generate changes in parallel, apply in batches:

```rust
let all_changes = crossbeam::scope(|s| {
    let handles: Vec<_> = (0..num_cpus::get()).map(|worker_id| {
        let manager_clone = Arc::clone(&manager);
        s.spawn(move |_| {
            // Each worker processes a subset of cells
            let mut changes = Vec::new();
            let cells = manager_clone.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
            
            for entry in cells.iter() {
                if entry.key().h3_cell_index().raw() % num_cpus::get() as u64 == worker_id as u64 {
                    // Process this cell and generate changes
                    changes.push(create_change_for_cell(entry.key(), entry.value()));
                }
            }
            changes
        })
    }).collect();
    
    // Collect all changes
    let mut all_changes = Vec::new();
    for handle in handles {
        all_changes.extend(handle.join().unwrap());
    }
    all_changes
}).unwrap();

// Apply all changes atomically
manager.apply_changes(all_changes).unwrap();
```

## Event-Driven Performance Benefits

### Thread Safety
- **Lock-free event emission** - Components can emit events concurrently
- **Channel-based communication** - Uses crossbeam unbounded channels
- **Atomic event application** - All events applied together safely
- **No data races** - Components never access mutable data directly

### Event Compression
- **Automatic compression** - Multiple `AddToField` events for same field are merged
- **Reduced memory allocation** - Fewer actual data modifications
- **Better cache performance** - Batched updates are more cache-friendly
- **Energy transfer optimization** - Perfect for geological energy calculations

### Parallel Scaling
- **Component parallelism** - All components run concurrently
- **Event parallelism** - Event emission is lock-free across threads
- **Linear scaling** - Performance scales with number of CPU cores
- **No contention** - Components don't compete for data access

## Performance Characteristics

### Memory Usage
- **100% memory-resident** - no disk I/O
- **Structural sharing** - efficient cloning via Arc
- **Lock-free** - no memory barriers for reads

### Concurrency
- **Lock-free reads** - unlimited concurrent readers
- **Lock-free writes** - concurrent writes to different keys
- **Atomic operations** - single-key modifications are atomic

### Scalability
- **Linear scaling** - performance scales with CPU cores
- **NUMA-aware** - works well on multi-socket systems
- **Cache-friendly** - hot data stays in CPU cache

## Best Practices

### 1. Use Event-Driven Pattern (Recommended)
```rust
// ✅ GOOD: Event-driven with parallel components
let emitter = manager.get_event_emitter();
crossbeam::scope(|s| {
    s.spawn(|_| component1.process(&sim, &emitter.clone()));
    s.spawn(|_| component2.process(&sim, &emitter.clone()));
});
manager.apply_pending_events().unwrap();

// ❌ AVOID: Direct modification in components
component1.process_direct(&mut cells); // Not thread-safe
```

### 2. Leverage Event Compression
```rust
// ✅ GOOD: Multiple adds will be compressed automatically
emitter.add_to_field("GEOLOGICAL_CELLS", location, "energy_joules", delta1);
emitter.add_to_field("GEOLOGICAL_CELLS", location, "energy_joules", delta2);
emitter.add_to_field("GEOLOGICAL_CELLS", location, "energy_joules", delta3);
// Results in single update: energy_joules += (delta1 + delta2 + delta3)

// ❌ AVOID: Manual batching (unnecessary complexity)
let total_delta = delta1 + delta2 + delta3;
emitter.add_to_field("GEOLOGICAL_CELLS", location, "energy_joules", total_delta);
```

### 3. Batch Operations (Legacy)
```rust
// Good: Batch multiple changes
let changes = vec![change1, change2, change3];
manager.apply_changes(changes).unwrap();

// Avoid: Individual change applications
manager.apply_changes(vec![change1]).unwrap();
manager.apply_changes(vec![change2]).unwrap();
manager.apply_changes(vec![change3]).unwrap();
```

### 2. Minimize Lock Scope
```rust
// Good: Short-lived references
{
    let cells = manager.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
    let temp = cells.get(&location).map(|data| data.temperature_k);
} // Reference dropped here

// Avoid: Long-lived references
let cells = manager.get::<CellLocation, GeologicalCellData>("GEOLOGICAL_CELLS").unwrap();
// ... lots of other work ...
let temp = cells.get(&location).map(|data| data.temperature_k);
```

### 3. Use Appropriate Collection Types
```rust
// Geological cells: Use CellLocation for 3D indexing
Collection<CellLocation, GeologicalCellData>

// Components: Use simple IDs
Collection<u32, ComponentData>

// Temporal data: Use time-based keys
Collection<u64, TimeSeriesData>
```

## Thread Safety Guarantees

- **Data Race Free** - All operations are thread-safe
- **Memory Safe** - No use-after-free or double-free
- **Deadlock Free** - No locks means no deadlocks
- **ABA Problem Free** - Handled by underlying DashMap implementation

## Integration with Geological Simulation

The collections system is designed specifically for geological simulations:

- **CellLocation indexing** - 3D spatial indexing with H3 + layers + depth
- **Flat storage** - All cells in single collection for efficiency
- **Component-friendly** - Easy parallel component processing
- **Change tracking** - Audit trail of all modifications
- **High performance** - Optimized for 60 FPS simulation targets

## Change System Patterns

### Creating Custom Changes

Implement the `CollectionChange` trait for your data types:

```rust
#[derive(Debug, Clone)]
pub enum GeologicalChange {
    UpdateTemperature { location: CellLocation, new_temp: f64 },
    AddEnergy { location: CellLocation, energy_delta: f64 },
    SetPressure { location: CellLocation, pressure: f64 },
}

impl CollectionChange for GeologicalChange {
    fn collection_name(&self) -> &'static str {
        "GEOLOGICAL_CELLS"
    }

    fn apply_to_collection(&self, collection: &mut dyn std::any::Any) -> Result<(), String> {
        let collection = collection
            .downcast_mut::<Collection<CellLocation, GeologicalCellData>>()
            .ok_or("Collection type mismatch")?;

        match self {
            GeologicalChange::UpdateTemperature { location, new_temp } => {
                collection.modify(location, |data| {
                    data.temperature_k = *new_temp;
                });
            },
            GeologicalChange::AddEnergy { location, energy_delta } => {
                collection.modify(location, |data| {
                    data.energy_mass.add_energy_joules(*energy_delta);
                });
            },
            GeologicalChange::SetPressure { location, pressure } => {
                collection.modify(location, |data| {
                    data.pressure_pa = *pressure;
                });
            },
        }
        Ok(())
    }
}
```

### Change Factories

Create helper functions for common changes:

```rust
pub fn create_temperature_change(location: CellLocation, temp: f64) -> Box<dyn CollectionChange> {
    Box::new(GeologicalChange::UpdateTemperature { location, new_temp: temp })
}

pub fn create_energy_change(location: CellLocation, delta: f64) -> Box<dyn CollectionChange> {
    Box::new(GeologicalChange::AddEnergy { location, energy_delta: delta })
}
```

## Testing Patterns

The collections system includes comprehensive tests:

```bash
# Run all collections tests
cargo test --lib collections

# Run specific test with output
cargo test test_parallel_collections_execution -- --nocapture

# Run integration tests
cargo test test_collections_system_integration -- --nocapture
```

## Examples

- **`examples/event_driven_simulation.rs`** - **RECOMMENDED**: Thread-safe event-driven geological simulation
- **`examples/basic_simulation.rs`** - Basic geological simulation (legacy direct access)
- **`src/collections/tests.rs`** - Comprehensive test suite with parallel examples
- **`src/collections/parallel_example.rs`** - Advanced parallel processing patterns

### Running Examples

```bash
# Run the recommended event-driven example
cargo run --example event_driven_simulation

# Run the basic simulation example
cargo run --example basic_simulation

# Run collections tests
cargo test --lib collections -- --nocapture
```

## Migration Guide

### From Direct Access to Event-Driven

**Old Pattern (Direct Modification):**
```rust
// Component modifies data directly
cells.modify(&location, |data| {
    data.temperature_k += heating_delta;
    data.pressure_pa += pressure_delta;
});
```

**New Pattern (Event-Driven):**
```rust
// Component emits events
emitter.add_to_field("GEOLOGICAL_CELLS", location, "temperature_k", heating_delta);
emitter.add_to_field("GEOLOGICAL_CELLS", location, "pressure_pa", pressure_delta);
```

**Benefits of Migration:**
- ✅ **Thread-safe** - Components can run in parallel
- ✅ **Event compression** - Multiple field updates are optimized
- ✅ **Better performance** - Reduced contention and better cache usage
- ✅ **Cleaner architecture** - Clear separation between computation and mutation

See these files for complete working examples of all patterns described above.
