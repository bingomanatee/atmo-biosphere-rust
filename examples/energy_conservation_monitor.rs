use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use atmo_biosphere_rust::component::SimComponent;
use atmo_biosphere_rust::energy_mass::energy_mass::EnergyMass;
use atmo_biosphere_rust::events::event_emitter::EventEmitter;
use atmo_biosphere_rust::events::event_listener::EventListener;
use atmo_biosphere_rust::events::event_types::EventType;
use h3o::Resolution;
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use std::sync::{Arc, Mutex};

/// Energy conservation monitor that tracks total system energy
#[derive(Debug, Clone)]
struct EnergyConservationMonitor {
    step_energies: Arc<Mutex<Vec<(i64, f64)>>>, // (step, total_energy)
    initial_energy: Arc<Mutex<Option<f64>>>,
    energy_violations: Arc<Mutex<Vec<(i64, f64, String)>>>, // (step, energy_change, reason)
}

impl EnergyConservationMonitor {
    fn new() -> Self {
        Self {
            step_energies: Arc::new(Mutex::new(Vec::new())),
            initial_energy: Arc::new(Mutex::new(None)),
            energy_violations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn record_energy(&self, step: i64, total_energy: f64) {
        let mut energies = self.step_energies.lock().unwrap();
        let mut initial = self.initial_energy.lock().unwrap();
        
        // Set initial energy on first recording
        if initial.is_none() {
            *initial = Some(total_energy);
            println!("🔋 Initial system energy: {:.6e} J", total_energy);
        }
        
        energies.push((step, total_energy));
        
        // Check for energy violations
        if let Some(initial_energy) = *initial {
            let energy_change = total_energy - initial_energy;
            let energy_change_percent = (energy_change / initial_energy) * 100.0;
            
            println!("📊 Step {}: Energy = {:.6e} J, Change = {:.6e} J ({:.6}%)", 
                     step, total_energy, energy_change, energy_change_percent);
            
            // Flag significant energy changes (should only be cooling to space)
            if energy_change_percent.abs() > 0.001 { // More than 0.001% change
                let mut violations = self.energy_violations.lock().unwrap();
                let reason = if energy_change > 0.0 {
                    "ENERGY CREATION DETECTED".to_string()
                } else {
                    "Energy loss (expected from space radiation)".to_string()
                };
                violations.push((step, energy_change, reason.clone()));
                
                if energy_change > 0.0 {
                    println!("🚨 VIOLATION: {}", reason);
                } else {
                    println!("❄️  {}", reason);
                }
            }
        }
    }

    fn generate_report(&self) -> String {
        let energies = self.step_energies.lock().unwrap();
        let initial = self.initial_energy.lock().unwrap();
        let violations = self.energy_violations.lock().unwrap();
        
        let mut report = String::new();
        report.push_str("\n🔋 ENERGY CONSERVATION ANALYSIS REPORT\n");
        report.push_str("=====================================\n\n");
        
        if let Some(initial_energy) = *initial {
            if let Some((final_step, final_energy)) = energies.last() {
                let total_change = final_energy - initial_energy;
                let total_change_percent = (total_change / initial_energy) * 100.0;
                
                report.push_str(&format!("Initial Energy: {:.6e} J\n", initial_energy));
                report.push_str(&format!("Final Energy:   {:.6e} J\n", final_energy));
                report.push_str(&format!("Total Change:   {:.6e} J ({:.6}%)\n", total_change, total_change_percent));
                report.push_str(&format!("Steps Monitored: {}\n\n", energies.len()));
                
                // Energy conservation verdict
                if total_change > 0.0 {
                    report.push_str("🚨 ENERGY CONSERVATION VIOLATION: System gained energy!\n");
                    report.push_str("   This indicates a bug in the radiative transfer or transaction system.\n\n");
                } else if total_change < 0.0 {
                    report.push_str("❄️  Energy loss detected (expected from space radiation)\n");
                    report.push_str(&format!("   Energy lost to space: {:.6e} J\n\n", -total_change));
                } else {
                    report.push_str("✅ Perfect energy conservation (no change)\n\n");
                }
                
                // Violations summary
                if !violations.is_empty() {
                    report.push_str("⚠️  Energy Change Events:\n");
                    for (step, change, reason) in violations.iter() {
                        report.push_str(&format!("   Step {}: {:.6e} J - {}\n", step, change, reason));
                    }
                    report.push_str("\n");
                }
                
                // Energy stability analysis
                if energies.len() > 1 {
                    let mut max_step_change = 0.0;
                    for i in 1..energies.len() {
                        let step_change = (energies[i].1 - energies[i-1].1).abs();
                        max_step_change = max_step_change.max(step_change);
                    }
                    report.push_str(&format!("Max step-to-step change: {:.6e} J\n", max_step_change));
                }
            }
        }
        
        report
    }
}

impl EventListener for EnergyConservationMonitor {
    fn handle_event(&self, event_type: &EventType, data: &str) {
        match event_type {
            EventType::SimulationStepCompleted => {
                // Parse step and energy data from event
                if let Some(step_start) = data.find("step:") {
                    if let Some(energy_start) = data.find("total_energy:") {
                        let step_str = &data[step_start + 5..].split(',').next().unwrap_or("0");
                        let energy_str = &data[energy_start + 13..].split(',').next().unwrap_or("0");
                        
                        if let (Ok(step), Ok(energy)) = (step_str.trim().parse::<i64>(), energy_str.trim().parse::<f64>()) {
                            self.record_energy(step, energy);
                        }
                    }
                }
            }
            _ => {} // Ignore other events
        }
    }
}

fn main() {
    println!("🔋 Energy Conservation Monitor");
    println!("==============================");
    println!("Testing radiative transfer for energy conservation violations");

    // Create energy monitor
    let monitor = EnergyConservationMonitor::new();
    
    // Create event emitter and register monitor
    let mut event_emitter = EventEmitter::new();
    event_emitter.add_listener(Box::new(monitor.clone()));

    // Create simulation configuration with radiative transfer
    let config = SimulationConfigImmut {
        steps: 5, // Run 5 steps to see energy trends
        years_per_step: 10000.0, // 10,000 years per step
        warmup_steps: 0,
        surface_temp_k: 288.15, // 15°C surface temperature
        layer_set_params: default_layer_set_params_immut(Resolution::Two, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig {
            years_per_step: 10000.0,
            max_transfer_rate: 0.01, // 1% max transfer per step
            enable_space_radiation: true,  // Enable space cooling
            enable_inter_layer_radiation: true,
            enable_intra_layer_radiation: true,
        },
    };

    // Create components (no additional energy sources)
    let mut components: Vec<Box<dyn SimComponent>> = vec![
        // No components - testing pure radiative transfer
    ];

    // Create immutable simulation
    let mut sim = SimulationImmut::new(config, &mut components);

    println!("\n🌍 Starting energy-monitored simulation...");
    println!("Configuration:");
    println!("   - Steps: {}", sim.config.steps);
    println!("   - Years per step: {}", sim.config.years_per_step);
    println!("   - Space radiation: {}", sim.config.radiative_transfer_config.enable_space_radiation);
    println!("   - Layer structure: 5+5+5 cells (15 total, 165km depth)");

    // Calculate and emit initial energy
    let initial_total_energy: f64 = sim.layer_sets.iter()
        .flat_map(|layer_set| layer_set.layers.values())
        .flat_map(|column| &column.cells)
        .map(|cell| cell.energy_joules())
        .sum();

    event_emitter.emit(
        EventType::SimulationStepCompleted,
        &format!("step:0,total_energy:{}", initial_total_energy)
    );

    // Run simulation steps with energy monitoring
    for step in 0..sim.config.steps {
        println!("\n--- Step {} ---", step + 1);
        
        sim.step();
        
        // Calculate total energy after step
        let total_energy: f64 = sim.layer_sets.iter()
            .flat_map(|layer_set| layer_set.layers.values())
            .flat_map(|column| &column.cells)
            .map(|cell| cell.energy_joules())
            .sum();

        // Emit energy monitoring event
        event_emitter.emit(
            EventType::SimulationStepCompleted,
            &format!("step:{},total_energy:{}", step + 1, total_energy)
        );
    }

    // Generate and display energy conservation report
    println!("{}", monitor.generate_report());

    println!("🔬 Energy Conservation Test Complete!");
    println!("=====================================");
    println!("If energy increased: BUG in radiative transfer");
    println!("If energy decreased: Expected space cooling");
    println!("If energy unchanged: Perfect conservation");
}
