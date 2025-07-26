# Atmo-Biosphere Rust

A high-performance geological simulation system designed for billion-year timescales with real-time video game performance targets.

## 🎯 Project Goals

### Primary Objective
Create a geological simulation capable of **watchable video game experience** performance:
- **Target**: 60 FPS real-time capability (~16.7ms per simulation step)
- **Current**: 4.5 hours for billion-year simulation (16.1ms per step)
- **Challenge**: Bridge the gap between geological accuracy and game performance

### Core Architecture
- **Immutable cell-based simulation** with binary pairing system
- **Component-driven geological processes** (heat transfer, core radiance, convection)
- **Transaction-based energy/mass conservation** with safety validation
- **Cross-platform compatibility** (native servers + WebAssembly)

## 🔧 Performance Analysis

### Current Bottlenecks (Identified 2025-01)
The real performance limitation is **not** the transaction system (only ~16% overhead), but:

1. **3,200 binary pairs processed per step**
2. **Complex physics calculations per pair**:
   - Heat transfer equations
   - Stefan-Boltzmann radiation
   - Perlin noise generation
   - Temperature/pressure lookups
3. **~51,200 physics operations per simulation step**

### Optimization Strategies

#### 1. Reduce Computational Load
- **Coarser H3 resolution**: Resolution::Two (~200 cells vs ~800 cells) = 4x speedup
- **Spatial culling**: Only process "active" geological regions
- **Skip trivial calculations**: Ignore pairs with minimal temperature differences
- **Caching**: Temperature, conductivity, and Perlin noise results

#### 2. GPU Acceleration (Primary Strategy)
**Technology**: `wgpu` (WebGPU/compute shaders)

**Why wgpu**:
- ✅ Cross-platform: Native (Metal/Vulkan/DirectX) + WebAssembly (WebGL/WebGPU)
- ✅ No vendor lock-in: Auto-selects best available backend
- ✅ Graceful CPU fallback: Works on any hardware
- ✅ Massive parallelization: 100-1000x speedup potential

**Target GPU Operations**:
```rust
// Heat diffusion - embarrassingly parallel
for each cell in parallel {
    new_temp = calculate_heat_transfer(neighbors);
}

// Perlin noise - parallel generation  
for each cell in parallel {
    energy_input = perlin_noise(x, y, time) * base_energy;
}

// Stefan-Boltzmann radiation - parallel
for each surface_cell in parallel {
    energy_loss = stefan_boltzmann_law(temperature);
}
```

## 🌐 Deployment Strategy

### Development Environment
- **Local GPU acceleration**: Fast iteration and testing
- **wgpu auto-detection**: Uses available GPU hardware

### Production Deployment

#### Standard Cloud (Cost-Effective)
- **AWS t3.large**: ~$60/month, CPU-only
- **wgpu CPU fallback**: Automatic degradation
- **Performance**: Current speeds, reliable

#### High-Performance Cloud (When Justified)
- **AWS g4dn.xlarge**: ~$380/month, Tesla T4 GPU
- **wgpu GPU acceleration**: 100-1000x potential speedup
- **Cost justification**: Need 6x performance improvement

#### Browser/WebAssembly
- **User's hardware**: Leverages client-side GPU
- **WebGPU/WebGL**: Same wgpu codebase
- **Graceful degradation**: Falls back to WebGL or CPU

### Cost Analysis
```
Standard AWS (CPU): $60/month
GPU AWS: $380/month  
Speedup needed: 6x to justify GPU costs
Browser deployment: Free (user's hardware)
```

## 🏗️ Current Architecture

### Core Components
- **SimulationImmut**: Main simulation engine with immutable cells
- **BinaryPairingSystem**: Manages cell-to-cell interactions
- **SimpleTransactionManager**: Energy/mass conservation with validation
- **Component System**: Modular geological processes

### Geological Processes
- **RadiativeTransferListener**: Heat diffusion between neighboring cells
- **CoreHeatListener**: Perlin noise variation + hotspot energy injection
- **SurfaceEmissionListener**: Stefan-Boltzmann radiation to space

### Data Flow
1. **Generate binary pairs**: 3,200 cell interactions per step
2. **Process components**: Each listener calculates energy/mass changes
3. **Accumulate transactions**: Safe energy conservation tracking
4. **Apply changes**: Immutable cell reconstruction with new values
5. **Validate results**: Energy conservation and safety checks

## 🚀 Next Steps

### Phase 1: GPU Prototype
- Implement wgpu compute shaders for heat diffusion
- Benchmark GPU vs CPU performance
- Validate cross-platform compatibility

### Phase 2: Optimization
- Profile and optimize hottest code paths
- Implement spatial culling and caching
- Reduce unnecessary calculations

### Phase 3: Production
- Deploy with automatic GPU/CPU selection
- Monitor performance in cloud environments
- Scale based on actual usage patterns

## 📊 Performance Targets

### Current State
- **16.1ms per step** (62 FPS equivalent)
- **4.5 hours** for billion-year simulation
- **Transaction overhead**: ~16% (acceptable)

### Target State
- **<16.7ms per step** (60+ FPS real-time)
- **<1 hour** for billion-year simulation
- **GPU acceleration**: 100-1000x speedup for parallel operations

### Success Metrics
- ✅ Real-time geological evolution visualization
- ✅ Interactive billion-year timescales
- ✅ Cross-platform deployment (server + browser)
- ✅ Cost-effective cloud hosting options

---

*Last updated: January 2025*
*Performance analysis based on comprehensive profiling and optimization attempts*
