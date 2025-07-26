use atmo_biosphere_rust::sim_immut::simulation_immut::{SimulationImmut, SimulationConfigImmut};
use atmo_biosphere_rust::sim_immut::layer_set_immut::default_layer_set_params_immut;
use atmo_biosphere_rust::sim_immut::radiative_transfer::RadiativeTransferConfig;
use h3o::Resolution;

#[test]
fn test_method_exists() {
    println!("🔍 CHECKING IF METHOD EXISTS");
    
    // Create minimal simulation
    let config = SimulationConfigImmut {
        warmup_steps: 0,
        steps: 1,
        years_per_step: 1000.0,
        surface_temp_k: 288.0,
        layer_set_params: default_layer_set_params_immut(Resolution::Three, 6371.0),
        radiative_transfer_config: RadiativeTransferConfig::default(),
    };
    
    let mut components = vec![];
    let mut sim = SimulationImmut::new(config, &mut components);
    
    // Check if the method exists by trying to get a reference to it
    let _method_ref: fn(&mut SimulationImmut) = SimulationImmut::step_with_binary_pairing;
    
    println!("✅ Method step_with_binary_pairing exists!");
}
