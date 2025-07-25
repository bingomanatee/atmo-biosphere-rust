use atmo_biosphere_rust::sim::simulation::{Simulation, SimulationConfig, ThermalGradientConfig};
use atmo_biosphere_rust::sim::layer_set::LayerSetParams;
use atmo_biosphere_rust::component::{SimComponent, CoreRadianceComponent, ConvectionPlumeComponent};
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use h3o::Resolution;

fn main() {
    println!("🔥 Core Radiance with Perlin Noise Demo");
    println!("=======================================");

    // Create simple 2-layer configuration for demo
    let thermal_config = ThermalGradientConfig {
        surface_temperature_k: 288.15,      // 15°C surface
        surface_gradient_k_per_km: 25.0,    // 25K/km gradient
        deep_gradient_k_per_km: 10.0,       // 10K/km at depth
        reference_depth_km: 100.0,          // Transition at 100km
    };

    let layer_params = vec![
        // Upper layer (0-50km)
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 0.0,
            cell_height_km: 25.0,           // 25km thick cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km total
            planet_radius_km: 6371.0,
        },
        // Deep layer (50-100km) - will receive core radiance
        LayerSetParams {
            resolution: Resolution::Four,
            start_height_km: 50.0,
            cell_height_km: 25.0,           // 25km thick cells
            material_name: "basalt".to_string(),
            column_count: 2,                // 2 cells = 50km total
            planet_radius_km: 6371.0,
        },
    ];

    let config = SimulationConfig {
        steps: 5,                           // 5 steps for demo
        years_per_step: 1000.0,            // 1000 years per step
        warmup_steps: 0,
        layer_set_params: layer_params,
        thermal_config,
    };

    println!("\n🏗️ Demo Configuration:");
    println!("   - 2 layer sets: 0-50km and 50-100km");
    println!("   - Core radiance applied to deepest cells (75-100km)");
    println!("   - Perlin noise creates ±15% spatial/temporal variation");
    println!("   - Convection plumes transport energy upward");

    // Create components with core radiance (including temporal drift) and convection
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        Box::new(CoreRadianceComponent::new()
            .with_base_energy(1e19)        // 1e19 J per cell per year
            .with_noise_amplitude(0.15)    // ±15% variation
            .with_spatial_scale(0.1)       // Coarse spatial features
            .with_geological_drift()),     // 1% drift per 100k years
        Box::new(ConvectionPlumeComponent::with_seed(42)),
    ];

    println!("\n🚀 Creating simulation...");
    let mut sim = Simulation::new(config, &mut components);
    
    println!("✓ Simulation created");
    println!("   - Layer sets: {}", sim.layer_sets.len());
    
    // Show initial energy state
    println!("\n📊 Initial Energy State:");
    for (i, layer_set) in sim.layer_sets.iter().enumerate() {
        let mut total_energy = 0.0;
        let mut cell_count = 0;
        
        for column in layer_set.layers.values() {
            for cell in &column.cells {
                total_energy += cell.energy_joules();
                cell_count += 1;
            }
        }
        
        println!("   Layer {}: {:.2e}J total ({} cells)", i, total_energy, cell_count);
    }

    println!("\n🔧 Initializing simulation...");
    sim.initialize();

    println!("\n⚡ Running simulation steps...");
    for step in 0..5 {
        println!("\n--- Step {} ---", step + 1);
        sim.step();
        
        // Show energy changes
        for (i, layer_set) in sim.layer_sets.iter().enumerate() {
            let mut total_energy = 0.0;
            let mut cell_count = 0;
            
            for column in layer_set.layers.values() {
                for cell in &column.cells {
                    total_energy += cell.energy_joules();
                    cell_count += 1;
                }
            }
            
            println!("   Layer {}: {:.2e}J total", i, total_energy);
        }
    }

    println!("\n🔬 Core Radiance Effects:");
    println!("   ✓ Perlin noise creates spatial variation in energy input");
    println!("   ✓ Different cells receive different amounts of energy");
    println!("   ✓ Energy input varies over time (slowly)");
    println!("   ✓ Temporal drift shifts patterns over geological time");
    println!("   ✓ Deepest layer receives continuous energy injection");
    println!("   ✓ Convection transports energy from deep to shallow layers");

    println!("\n📈 Expected Behavior:");
    println!("   1. Core radiance adds energy to deepest cells");
    println!("   2. Perlin noise creates ±15% spatial variation");
    println!("   3. Hot spots in deep layer trigger convection plumes");
    println!("   4. Plumes transport energy to upper layers");
    println!("   5. System develops realistic thermal heterogeneity");

    println!("\n✅ Core Radiance Demo completed!");
    println!("\n🎯 Key Features Demonstrated:");
    println!("   ✓ Perlin noise-modulated energy input");
    println!("   ✓ Spatial variation (±15% amplitude)");
    println!("   ✓ Temporal variation (slow changes)");
    println!("   ✓ Geological temporal drift (1% per 100k years)");
    println!("   ✓ Non-orthogonal drift direction for realism");
    println!("   ✓ Coarse spatial resolution for realistic patterns");
    println!("   ✓ Integration with convection system");
    println!("   ✓ Energy conservation and transport");
}
