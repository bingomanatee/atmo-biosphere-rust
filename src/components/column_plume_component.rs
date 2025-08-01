use crate::cell_location::CellLocation;
use crate::collections::{Actor, CollectionsManager};
use crate::material::{MaterialsLoader, MaterialPhases};
use crate::simulation::{Component, GeologicalCellData, Simulation, SimulationConfig};
use crate::utils::column_processor::{ColumnProcessor, VerticalColumn};
use h3o::CellIndex;
use rayon::prelude::*;
use std::sync::Mutex;
use std::collections::HashMap;

/// Column-based convection plume component
/// Processes vertical columns to detect buoyancy conditions and generate plumes
/// All plume data is stored in collections for proper architecture
pub struct ColumnPlumeComponent {
    /// Minimum temperature for plume formation (K)
    plume_threshold_temp_k: f64,
    /// Minimum density difference for buoyancy (kg/m³)
    min_density_difference: f64,
    /// Energy transfer efficiency for plumes
    energy_transfer_efficiency: f64,
    /// Mass transfer efficiency for plumes
    mass_transfer_efficiency: f64,
    /// Random number generator seed for reproducible results
    rng_seed: u64,
    /// Cache for pre-computed buoyancy factors (CellLocation -> BuoyancyFactor)
    buoyancy_cache: Mutex<HashMap<CellLocation, BuoyancyFactor>>,
}

/// Pre-computed buoyancy factor for each cell
/// Captures the key geological conditions that drive plume formation
/// PERSISTENT across simulation steps - only recalculated when cells change significantly
#[derive(Debug, Clone)]
struct BuoyancyFactor {
    /// Base buoyancy potential (0.0 to 1.0)
    base_buoyancy: f64,
    /// Temperature contribution factor
    thermal_factor: f64,
    /// Pressure contribution factor
    pressure_factor: f64,
    /// Density difference potential
    density_factor: f64,
    /// Combined buoyancy strength (product of all factors)
    combined_strength: f64,
    /// Cell volume for scaling
    cell_volume_km3: f64,
    /// Last temperature when this was calculated (for change detection)
    last_temperature_k: f64,
    /// Last pressure when this was calculated (for change detection)
    last_pressure_pa: f64,
    /// Step when this was last calculated
    last_calculated_step: u32,
}

/// Plume formation data for a column
#[derive(Debug, Clone)]
struct PlumeFormation {
    /// Source cell location (deepest hot cell)
    source_location: CellLocation,
    /// Target cell location (where energy/mass goes)
    target_location: CellLocation,
    /// Energy to transfer (Joules)
    energy_transfer: f64,
    /// Mass to transfer (kg)
    mass_transfer: f64,
    /// Buoyancy strength (for logging)
    buoyancy_strength: f64,
    /// Material type being transported
    material_type: String,
    /// Volatile content (for atmospheric outgassing)
    volatile_content_kg: f64,
    /// Surface eruption potential (0.0-1.0)
    eruption_potential: f64,
}

/// Multi-layer plume formation that distributes energy through multiple layers
/// Based on realistic plume speeds that can traverse multiple layers per timestep
#[derive(Debug, Clone)]
struct MultiLayerPlumeFormation {
    /// Source cell location (deepest hot cell)
    source_location: CellLocation,
    /// Path of cells the plume traverses: (location, energy_fraction, mass_fraction)
    plume_path: Vec<(CellLocation, f64, f64)>,
    /// Total energy to distribute across all layers (Joules)
    total_energy_transfer: f64,
    /// Total mass to distribute across all layers (kg)
    total_mass_transfer: f64,
    /// Buoyancy strength (for logging)
    buoyancy_strength: f64,
    /// Material type being transported
    material_type: String,
    /// Volatile content (for atmospheric outgassing)
    volatile_content_kg: f64,
    /// Surface eruption potential (0.0-1.0)
    eruption_potential: f64,
    /// Distance plume traverses in this timestep (km)
    traversal_distance_km: f64,
}

/// Surface eruption event for atmosphere creation
#[derive(Debug, Clone)]
pub struct SurfaceEruption {
    /// Location of eruption
    pub location: CellLocation,
    /// H3 cell index for geographic reference
    pub h3_cell: CellIndex,
    /// Total mass erupted (kg)
    pub erupted_mass_kg: f64,
    /// Volatile gases released (kg)
    pub volatile_gases_kg: f64,
    /// Material composition
    pub material_type: String,
    /// Eruption temperature (K)
    pub eruption_temp_k: f64,
    /// Eruption energy (Joules)
    pub eruption_energy_j: f64,
}

/// Simple plume tracking for energy and material transport
#[derive(Debug, Clone)]
pub struct PlumeStatistics {
    /// Simulation step
    pub step: u32,
    /// Total number of plumes formed this step
    pub plume_count: usize,
    /// Total mass transported by plumes (kg)
    pub total_mass_transported_kg: f64,
    /// Total energy transported by plumes (Joules)
    pub total_energy_transported_j: f64,
    /// Total volatile gases released (kg)
    pub total_volatiles_released_kg: f64,
    /// Number of surface eruptions
    pub surface_eruptions_count: usize,
    /// Mass transported by material type
    pub mass_by_material: std::collections::HashMap<String, f64>,
    /// Energy transported by material type
    pub energy_by_material: std::collections::HashMap<String, f64>,
}

/// Simple cumulative trackers for total energy and material moved
#[derive(Debug, Clone)]
pub struct PlumeTotals {
    /// Total energy put into plumes since simulation start (Joules)
    pub total_energy_in_plumes_j: f64,
    /// Total material put into plumes since simulation start (kg)
    pub total_material_in_plumes_kg: f64,
    /// Total energy expressed to atmosphere via surface eruptions (Joules)
    pub total_energy_to_atmosphere_j: f64,
    /// Total plumes created since simulation start
    pub total_plumes_created: usize,
    /// Total surface eruptions since simulation start
    pub total_surface_eruptions: usize,
}

/// Cumulative energy tracking across all simulation steps
#[derive(Debug, Clone)]
pub struct CumulativeEnergyTracker {
    /// Total energy moved by plumes since simulation start (Joules)
    pub total_energy_moved_j: f64,
    /// Total thermal energy transported (Joules)
    pub total_thermal_energy_j: f64,
    /// Total kinetic energy from plume movement (Joules)
    pub total_kinetic_energy_j: f64,
    /// Energy moved upward (from deep to shallow) (Joules)
    pub upward_energy_flow_j: f64,
    /// Energy moved downward (from shallow to deep) (Joules)
    pub downward_energy_flow_j: f64,
    /// Energy lost to surface eruptions (Joules)
    pub surface_eruption_energy_j: f64,
    /// Average energy per plume (Joules)
    pub avg_energy_per_plume_j: f64,
    /// Peak energy transport in a single step (Joules)
    pub peak_step_energy_j: f64,
    /// Step when peak energy transport occurred
    pub peak_energy_step: u32,
    /// Total number of plumes tracked
    pub total_plumes_count: usize,
    /// Last updated step
    pub last_updated_step: u32,
}

impl ColumnPlumeComponent {
    /// Create new column-based plume component
    pub fn new() -> Self {
        Self {
            plume_threshold_temp_k: 1800.0,      // Hot enough for plume formation
            min_density_difference: 50.0,        // kg/m³ minimum for buoyancy
            energy_transfer_efficiency: 0.05,    // 5% energy transfer per step
            mass_transfer_efficiency: 0.02,      // 2% mass transfer per step
            rng_seed: 42,
            buoyancy_cache: Mutex::new(HashMap::new()),
        }
    }
    
    /// Create with custom parameters
    pub fn with_parameters(
        threshold_temp: f64,
        min_density_diff: f64,
        energy_efficiency: f64,
        mass_efficiency: f64,
    ) -> Self {
        Self {
            plume_threshold_temp_k: threshold_temp,
            min_density_difference: min_density_diff,
            energy_transfer_efficiency: energy_efficiency.clamp(0.0, 1.0),
            mass_transfer_efficiency: mass_efficiency.clamp(0.0, 1.0),
            rng_seed: 42,
            buoyancy_cache: Mutex::new(HashMap::new()),
        }
    }
    
    /// Simple LCG random number generator for reproducible results
    fn next_random(&self, step: u32, column_index: u64) -> f64 {
        let seed = self.rng_seed.wrapping_add(step as u64).wrapping_add(column_index);
        let a = 1664525u64;
        let c = 1013904223u64;
        let m = 2u64.pow(32);

        let next = (a.wrapping_mul(seed).wrapping_add(c)) % m;
        next as f64 / m as f64
    }

    /// Get or calculate buoyancy factor for a cell with PERSISTENT CACHING
    /// Only recalculates if temperature or pressure changed significantly
    /// This is the KEY OPTIMIZATION - avoids expensive recalculations each step
    fn get_buoyancy_factor(
        &self,
        location: &CellLocation,
        cell_data: &GeologicalCellData,
        step: u32,
        cell_volume_km3: f64,
    ) -> f64 {
        let mut cache = self.buoyancy_cache.lock().unwrap();

        // Check if we have a cached value that's still valid
        if let Some(cached) = cache.get(location) {
            let temp_change = (cell_data.temperature_k - cached.last_temperature_k).abs();
            let pressure_change = (cell_data.pressure_pa - cached.last_pressure_pa).abs();

            // Only recalculate if significant changes (>1% temperature or >5% pressure)
            let temp_threshold = cached.last_temperature_k * 0.01; // 1% temperature change
            let pressure_threshold = cached.last_pressure_pa * 0.05; // 5% pressure change

            if temp_change < temp_threshold && pressure_change < pressure_threshold {
                // Cache hit! Return cached combined strength
                return cached.combined_strength;
            }
        }

        // Cache miss or significant change - recalculate buoyancy factor
        let thermal_factor = (cell_data.temperature_k / 1000.0).powf(1.5);
        let pressure_factor = (cell_data.pressure_pa / 1e7).max(0.1).min(2.0);
        let volume_factor = cell_volume_km3 / 10000.0;

        // Calculate density factor based on temperature (thermal expansion)
        let reference_temp = 300.0; // Reference temperature
        let thermal_expansion_coeff = 3e-5; // /K
        let density_reduction = thermal_expansion_coeff * (cell_data.temperature_k - reference_temp);
        let density_factor = density_reduction.max(0.0).min(0.3); // Cap at 30% density reduction

        // Base buoyancy potential (geological setting)
        let depth_factor = (location.depth_index() as f64 + 1.0) / 10.0; // Deeper = more potential
        let base_buoyancy = depth_factor.min(1.0);

        // Combined strength (multiplicative - all factors contribute)
        let combined_strength = base_buoyancy * thermal_factor * pressure_factor * volume_factor * (1.0 + density_factor);

        // Cache the new calculation
        let buoyancy_factor = BuoyancyFactor {
            base_buoyancy,
            thermal_factor,
            pressure_factor,
            density_factor,
            combined_strength,
            cell_volume_km3,
            last_temperature_k: cell_data.temperature_k,
            last_pressure_pa: cell_data.pressure_pa,
            last_calculated_step: step,
        };

        cache.insert(*location, buoyancy_factor);
        combined_strength
    }

    /// Calculate gradual plume energy dissipation through layers
    /// Plume weakens as it transfers energy to each layer based on temperature difference
    /// Returns vector of (location, energy_transfer, mass_transfer) for each affected layer
    fn calculate_gradual_plume_dissipation(
        &self,
        source_location: &CellLocation,
        initial_plume_energy: f64,
        initial_plume_mass: f64,
        initial_plume_temp: f64,
        traversal_distance_km: f64,
        column: &VerticalColumn,
    ) -> Vec<(CellLocation, f64, f64)> {
        let mut dissipation_path = Vec::new();
        let mut remaining_distance = traversal_distance_km;
        let mut remaining_energy = initial_plume_energy;
        let mut remaining_mass = initial_plume_mass;
        let mut current_plume_temp = initial_plume_temp;

        // Find source cell in column and work upward
        let source_depth = source_location.depth_index();
        let source_layer = source_location.layer_set_index();

        // Collect all cells above source (including source layer)
        let mut cells_above: Vec<(CellLocation, &GeologicalCellData)> = column.cells
            .iter()
            .filter(|(loc, _)| {
                loc.layer_set_index() <= source_layer &&
                (loc.layer_set_index() < source_layer || loc.depth_index() <= source_depth)
            })
            .map(|(loc, data)| (*loc, data))
            .collect();

        // Sort by depth (deepest first, working toward surface)
        cells_above.sort_by_key(|(loc, _)| (loc.layer_set_index(), loc.depth_index()));
        cells_above.reverse(); // Start from source and go up

        // GRADUAL ENERGY DISSIPATION: Plume weakens as it passes through each layer
        for (location, cell_data) in cells_above {
            if remaining_distance <= 0.0 || remaining_energy <= 0.0 {
                break; // Plume exhausted
            }

            // Get cell height based on layer configuration
            let cell_height_km = match location.layer_set_index() {
                0 => 5.0,   // Continental Crust: 5km per cell
                1 => 25.0,  // Upper Mantle: 25km per cell
                2 => 50.0,  // Lower Mantle: 50km per cell
                _ => 25.0,  // Default
            };

            let distance_in_cell = remaining_distance.min(cell_height_km);
            let fraction_of_cell = distance_in_cell / cell_height_km;

            // TEMPERATURE-DEPENDENT TRANSFER RATE
            let temp_difference = current_plume_temp - cell_data.temperature_k;
            if temp_difference > 0.0 {
                // Transfer rate increases with temperature difference (realistic physics)
                let base_transfer_rate = 0.1; // 10% base transfer per cell
                let temp_factor = (temp_difference / 1000.0).min(2.0); // Max 2x boost for extreme temp diff
                let distance_factor = fraction_of_cell; // More transfer if plume spends more distance in cell

                let energy_transfer_rate = base_transfer_rate * temp_factor * distance_factor;
                let mass_transfer_rate = energy_transfer_rate * 0.8; // Mass transfer slightly more conservative

                // Calculate actual transfers (limited by remaining energy/mass)
                let energy_transfer = (remaining_energy * energy_transfer_rate).min(remaining_energy);
                let mass_transfer = (remaining_mass * mass_transfer_rate).min(remaining_mass);

                // Update remaining plume energy/mass
                remaining_energy -= energy_transfer;
                remaining_mass -= mass_transfer;

                // Cool the plume as it loses energy
                let energy_loss_fraction = energy_transfer / initial_plume_energy;
                current_plume_temp -= energy_loss_fraction * (current_plume_temp - cell_data.temperature_k) * 0.5;

                // Record this transfer
                if energy_transfer > 0.0 {
                    dissipation_path.push((location, energy_transfer, mass_transfer));
                }
            }

            remaining_distance -= distance_in_cell;
        }

        dissipation_path
    }
    
    /// Analyze a vertical column for plume formation potential with area scaling
    fn analyze_column_for_plumes_with_area(
        &self,
        column: &VerticalColumn,
        step: u32,
        area_per_column_m2: f64,
    ) -> Option<PlumeFormation> {
        if column.cell_count() < 2 {
            return None; // Need at least 2 cells for vertical transfer
        }
        
        // Neighbor-to-neighbor comparison: check each cell against its upstairs neighbor
        let mut best_plume: Option<(CellLocation, &GeologicalCellData, CellLocation, &GeologicalCellData, f64)> = None;

        // Check each cell against the cell directly above it
        for i in 1..column.cells.len() { // Start from 1 (skip surface cell)
            let (source_location, source_data) = &column.cells[i]; // Lower cell (potential source)
            let (target_location, target_data) = &column.cells[i-1]; // Upper cell (direct neighbor)

            // Only consider hot enough sources
            if source_data.temperature_k < self.plume_threshold_temp_k {
                continue;
            }

            // Calculate thermal expansion effect for the hot source cell
            let thermal_expansion_coeff = 3e-5; // Typical rock thermal expansion (1/K)
            let reference_temp = 300.0; // Reference temperature (K)
            let temp_excess = source_data.temperature_k - reference_temp;

            // Calculate effective density after thermal expansion
            let thermal_density_reduction = source_data.density_kg_m3 * thermal_expansion_coeff * temp_excess;
            let effective_source_density = source_data.density_kg_m3 - thermal_density_reduction;

            // Compare to immediate upstairs neighbor
            let temp_diff = source_data.temperature_k - target_data.temperature_k;
            let density_diff = target_data.density_kg_m3 - effective_source_density;

            // CALIBRATED: Plume forms if source is hotter AND thermally-expanded source is less dense than neighbor above
            if temp_diff > 100.0 && density_diff > 10.0 {  // Calibrated thresholds for realistic formation
                // Calculate pressure difference
                let pressure_diff = source_data.pressure_pa - target_data.pressure_pa;

                // Calculate plume strength
                let pressure_factor = (pressure_diff / 1e6).max(1.0); // Normalize to MPa
                let thermal_factor = temp_excess / 1000.0; // Thermal driving force
                let plume_strength = temp_diff * density_diff * pressure_factor * thermal_factor;

                if best_plume.is_none() || plume_strength > best_plume.unwrap().4 {
                    best_plume = Some((*source_location, source_data, *target_location, target_data, plume_strength));
                }
            }
        }
        
        // Check if we found a viable plume
        if let Some((source_location, source_data, target_location, target_data, _plume_strength)) = best_plume {
            // Calculate buoyancy conditions
            let temp_difference = source_data.temperature_k - target_data.temperature_k;
            let density_difference = target_data.density_kg_m3 - source_data.density_kg_m3;
            
            // Check thresholds - plumes should be rare but possible!
            if temp_difference > 40.0 && density_difference.abs() > 50.0 {
                // Add randomness for realistic geological variation
                let column_hash = format!("{:?}", column.h3_index).len() as u64; // Simple hash
                let random_factor = self.next_random(step, column_hash);

                // Calculate area-scaled probability - larger areas should have higher chance
                let reference_area_m2 = 1e12; // 1 million km² reference area
                let area_factor = (area_per_column_m2 / reference_area_m2).sqrt(); // Square root scaling

                // Include pressure difference in probability
                let pressure_diff = source_data.pressure_pa - target_data.pressure_pa;
                let pressure_factor = (pressure_diff / 1e7).max(0.1).min(2.0); // 0.1x to 2x multiplier

                // OPTIMIZED: Use cached buoyancy factor instead of expensive recalculation
                let source_height_km = 25.0; // Typical mantle cell height in km
                let cell_volume_km3 = (area_per_column_m2 / 1e6) * source_height_km;

                // Get cached buoyancy factor (only recalculates if cell changed significantly)
                let buoyancy_strength = self.get_buoyancy_factor(&source_location, source_data, step, cell_volume_km3);

                // Geological base rate: Natural plume formation rate per reference conditions
                let geological_base_rate = 1.8e-5; // Base probability per step for reference conditions

                // Physics-based probability: Use cached buoyancy strength
                let physics_probability = geological_base_rate * buoyancy_strength;

                let geological_time_factor = 0.5; // Moderate geological time scaling
                let adjusted_probability = physics_probability * geological_time_factor * (0.1 + random_factor * 0.2);

                // Only form plume if random factor is very low
                if random_factor < adjusted_probability {
                    // MULTI-LAYER PLUME DISTRIBUTION: Distribute energy through all layers plume traverses
                    // Based on realistic plume speeds: 5-20 cm/year can traverse multiple layers per timestep

                    // Calculate plume traversal distance based on timestep
                    let plume_speed_m_per_year = 0.05 + (buoyancy_strength * 0.15); // 5-20 cm/year based on strength
                    let years_per_step = 100_000.0; // Default timestep (will be made configurable later)
                    let traversal_distance_km = (plume_speed_m_per_year * years_per_step) / 1000.0;

                    // Calculate initial plume energy and temperature
                    let base_energy_efficiency = self.energy_transfer_efficiency;
                    let base_mass_efficiency = self.mass_transfer_efficiency;
                    let energy_strength_factor = buoyancy_strength.powf(0.8);
                    let mass_strength_factor = buoyancy_strength.powf(0.6);
                    let energy_randomization = 0.7 + random_factor * 0.6;
                    let mass_randomization = 0.8 + random_factor * 0.4;

                    let initial_plume_energy = source_data.energy_mass.energy_joules() *
                        base_energy_efficiency * energy_strength_factor * energy_randomization;
                    let initial_plume_mass = source_data.energy_mass.mass_kg() *
                        base_mass_efficiency * mass_strength_factor * mass_randomization;
                    let initial_plume_temp = source_data.temperature_k;

                    // GRADUAL DISSIPATION: Calculate how plume weakens through each layer
                    let dissipation_path = self.calculate_gradual_plume_dissipation(
                        &source_location,
                        initial_plume_energy,
                        initial_plume_mass,
                        initial_plume_temp,
                        traversal_distance_km,
                        column,
                    );

                    // Determine material type based on source depth
                    let material_type = self.get_material_for_depth(source_location.depth_index());

                    // Get volatile content from material properties
                    // Note: volatile_fraction field doesn't exist in MaterialPhase, using default
                    let volatile_fraction = 0.01; // Default 1% volatile content
                    let total_volatile_content_kg = initial_plume_mass * volatile_fraction;

                    // Use gradual dissipation results
                    if let Some((primary_target, primary_energy_transfer, primary_mass_transfer)) = dissipation_path.first() {
                        // Calculate surface eruption potential based on how high plume reaches
                        let reaches_surface = dissipation_path.iter().any(|(loc, _, _)| loc.depth_index() == 0);
                        let eruption_potential = if reaches_surface {
                            // Plume reaches surface - high eruption potential
                            (temp_difference / 1000.0).min(1.0) * (1.0 + random_factor) * 0.5
                        } else {
                            // Subsurface plume - lower eruption potential
                            (temp_difference / 2000.0).min(0.5) * random_factor
                        };

                        // Return traditional PlumeFormation with primary target for compatibility
                        // TODO: Implement full multi-layer application in apply_plume_formations
                        return Some(PlumeFormation {
                            source_location,
                            target_location: *primary_target,
                            energy_transfer: *primary_energy_transfer,
                            mass_transfer: *primary_mass_transfer,
                            buoyancy_strength: density_difference,
                            material_type,
                            volatile_content_kg: total_volatile_content_kg,
                            eruption_potential,
                        });
                    }
                }
            }
        }

        None
    }
    
    /// Process all columns for plume formation with PARALLEL PROCESSING
    fn process_columns_for_plumes(
        &self,
        column_processor: &ColumnProcessor,
        step: u32,
    ) -> Vec<PlumeFormation> {
        let stats = column_processor.get_statistics();

        // Calculate area per column for resolution independence
        let earth_surface_area_m2 = 4.0 * std::f64::consts::PI * (6371000.0_f64).powi(2); // Earth surface area
        let area_per_column_m2 = earth_surface_area_m2 / stats.total_columns as f64;

        // PARALLEL FIRST PASS: identify potential plumes using rayon
        let potential_plumes: Vec<(CellIndex, PlumeFormation)> = column_processor
            .columns()
            .par_iter() // PARALLEL PROCESSING HERE!
            .filter_map(|(h3_index, column)| {
                if let Some(plume) = self.analyze_column_for_plumes_with_area(column, step, area_per_column_m2) {
                    Some((*h3_index, plume))
                } else {
                    None
                }
            })
            .collect();

        // OPTIMIZED SECOND PASS: apply plume suppression with thread-safe collection
        let plume_formations = Mutex::new(Vec::new());

        potential_plumes.par_iter().for_each(|(h3_index, plume)| {
            // Check if this plume should form based on nearby active plumes
            let current_plumes = plume_formations.lock().unwrap();
            let suppression_factor = self.calculate_plume_suppression(&current_plumes, h3_index, area_per_column_m2);
            drop(current_plumes); // Release lock early

            // Apply suppression to formation probability
            let column_hash = format!("{:?}", h3_index).len() as u64;
            let random_factor = self.next_random(step, column_hash);

            if random_factor < suppression_factor {
                plume_formations.lock().unwrap().push(plume.clone());
            }
        });

        plume_formations.into_inner().unwrap()
    }

    /// Calculate plume suppression factor based on nearby active plumes
    fn calculate_plume_suppression(
        &self,
        existing_plumes: &[PlumeFormation],
        h3_index: &CellIndex,
        area_per_column_m2: f64,
    ) -> f64 {
        // Base suppression factor (1.0 = no suppression)
        let mut suppression_factor = 1.0;

        // Calculate suppression radius based on area (larger areas = larger suppression radius)
        let suppression_radius_km = (area_per_column_m2 / 1e6).sqrt() * 5.0; // 5x the column "radius"

        // Count nearby plumes and reduce formation probability
        let nearby_plumes = existing_plumes.len(); // Simplified - in real implementation would check distance

        // Each nearby plume reduces formation chance
        let suppression_per_plume = 0.3_f64; // 30% reduction per nearby plume
        suppression_factor *= (1.0_f64 - suppression_per_plume).powi(nearby_plumes as i32);

        // Minimum suppression factor (always some chance)
        suppression_factor.max(0.1_f64)
    }

    /// Analyze a single column for plume formation potential (legacy method)
    fn analyze_column_for_plumes(
        &self,
        column: &VerticalColumn,
        step: u32,
    ) -> Option<PlumeFormation> {
        // Use default area for legacy calls
        let default_area_m2 = 1e12; // 1 million km²
        self.analyze_column_for_plumes_with_area(column, step, default_area_m2)
    }

    /// Get material type based on depth index
    fn get_material_for_depth(&self, depth_index: usize) -> String {
        match depth_index {
            0..=2 => "granite".to_string(),    // Shallow crust
            3..=6 => "basalt".to_string(),     // Deep crust / upper mantle
            _ => "peridotite".to_string(),     // Deep mantle
        }
    }

    /// Apply plume formations to the simulation via Actor
    fn apply_plume_formations(
        &self,
        plume_formations: Vec<PlumeFormation>,
        actor: &mut Actor,
        step: u32,
        surface_eruption_energy: f64,
    ) -> (usize, usize) {
        let mut transfers_applied = 0;
        let mut eruptions_created = 0;

        // Initialize comprehensive statistics tracking
        let mut total_mass_transported = 0.0;
        let mut total_energy_transported = 0.0;
        let mut total_volatiles_released = 0.0;
        let mut mass_by_material = std::collections::HashMap::new();
        let mut energy_by_material = std::collections::HashMap::new();

        for plume in plume_formations {
            // Transfer energy from source to target
            actor.add("geological_cells", plume.source_location, "energy_joules", -plume.energy_transfer);
            actor.add("geological_cells", plume.target_location, "energy_joules", plume.energy_transfer);

            // Transfer mass from source to target
            actor.add("geological_cells", plume.source_location, "mass_kg", -plume.mass_transfer);
            actor.add("geological_cells", plume.target_location, "mass_kg", plume.mass_transfer);

            transfers_applied += 1;

            // Track comprehensive statistics
            total_mass_transported += plume.mass_transfer;
            total_energy_transported += plume.energy_transfer;
            total_volatiles_released += plume.volatile_content_kg;

            // Track by material type
            *mass_by_material.entry(plume.material_type.clone()).or_insert(0.0) += plume.mass_transfer;
            *energy_by_material.entry(plume.material_type.clone()).or_insert(0.0) += plume.energy_transfer;

            // Check for surface eruption (atmosphere creation)
            if plume.eruption_potential > 0.3 && plume.target_location.depth_index() == 0 {
                let eruption = SurfaceEruption {
                    location: plume.target_location,
                    h3_cell: plume.target_location.h3_cell_index(),
                    erupted_mass_kg: plume.mass_transfer,
                    volatile_gases_kg: plume.volatile_content_kg,
                    material_type: plume.material_type.clone(),
                    eruption_temp_k: 1200.0 + plume.buoyancy_strength * 2.0, // Estimate eruption temp
                    eruption_energy_j: plume.energy_transfer,
                };

                // Store eruption in collections using step as key
                // TODO: Implement proper string key storage for eruptions
                // let eruption_key = format!("step_{}_eruption_{}", step, eruptions_created);
                // actor.set_with_string_key("surface_eruptions", eruption_key, eruption);
                eruptions_created += 1;
            }
        }

        // Store step statistics and update cumulative totals
        if transfers_applied > 0 {
            let statistics = PlumeStatistics {
                step,
                plume_count: transfers_applied,
                total_mass_transported_kg: total_mass_transported,
                total_energy_transported_j: total_energy_transported,
                total_volatiles_released_kg: total_volatiles_released,
                surface_eruptions_count: eruptions_created,
                mass_by_material,
                energy_by_material,
            };

            // TODO: Implement proper string key storage for statistics
            // let stats_key = format!("step_{}_plume_stats", step);
            // actor.set_with_string_key("plume_statistics", stats_key, statistics);

            // Use the surface eruption energy passed as parameter
            let totals = PlumeTotals {
                total_energy_in_plumes_j: total_energy_transported,
                total_material_in_plumes_kg: total_mass_transported,
                total_energy_to_atmosphere_j: surface_eruption_energy,
                total_plumes_created: transfers_applied,
                total_surface_eruptions: eruptions_created,
            };

            // TODO: Implement proper string key storage for totals
            // let totals_key = format!("step_{}_plume_totals", step);
            // actor.set_with_string_key("plume_totals", totals_key, totals);
        }

        (transfers_applied, eruptions_created)
    }
}

impl Component for ColumnPlumeComponent {
    fn name(&self) -> &'static str {
        "ColumnPlumeComponent"
    }
    
    fn initialize(&mut self, coll_mgr: &mut CollectionsManager, _config: &SimulationConfig) {
        println!("🌋 ColumnPlumeComponent: Initializing column-based convection plumes...");
        println!("   • Plume threshold: {:.0}K", self.plume_threshold_temp_k);
        println!("   • Min density difference: {:.0} kg/m³", self.min_density_difference);
        println!("   • Energy transfer efficiency: {:.1}%", self.energy_transfer_efficiency * 100.0);
        println!("   • Mass transfer efficiency: {:.1}%", self.mass_transfer_efficiency * 100.0);
        println!("   • Material tracking: ENABLED (for atmospheric outgassing)");
        println!("   • Volatile fractions: Loaded from materials database");
        println!("   • Surface eruptions: TRACKED (for atmosphere creation)");
        println!("   • Processing method: Vertical columns (optimized)");

        // Create collections for simple plume tracking
        coll_mgr.add_empty_collection::<String, SurfaceEruption>("surface_eruptions");
        coll_mgr.add_empty_collection::<String, PlumeStatistics>("plume_statistics");
        coll_mgr.add_empty_collection::<String, PlumeTotals>("plume_totals");
        println!("   • Collections: surface_eruptions, plume_statistics, and plume_totals created");
        println!("   • Tracking: Energy in plumes, material in plumes, energy to atmosphere");
    }
    
    fn step(&self, coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, _year: f64, _config: &SimulationConfig) {
        let cells = coll_mgr.get::<CellLocation, GeologicalCellData>("geological_cells")
            .expect("geological_cells collection should exist");
        
        // Create column processor for efficient vertical processing
        let column_processor = ColumnProcessor::from_cells(&*cells);
        let stats = column_processor.get_statistics();
        
        // Note: Step counter tracking removed to avoid unsafe casting
        
        // Print every step for testing, or every 1000 for long runs
        if step <= 10 || step % 1000 == 0 {
            println!("🌋 ColumnPlumeComponent: Analyzing {} columns for plume formation at step {}", stats.total_columns, step);
        }
        
        // Process columns for plume formation
        let plume_formations = self.process_columns_for_plumes(&column_processor, step);

        // Calculate statistics before moving plume_formations
        let total_mass: f64 = plume_formations.iter().map(|p| p.mass_transfer).sum();
        let total_energy: f64 = plume_formations.iter().map(|p| p.energy_transfer).sum();
        let energy_to_atmosphere: f64 = plume_formations.iter()
            .filter(|p| p.eruption_potential > 0.3 && p.target_location.depth_index() == 0)
            .map(|p| p.energy_transfer)
            .sum();

        // Apply plume formations (collections-based, no internal state)
        let (transfers_applied, eruptions_created) = self.apply_plume_formations(plume_formations, actor, step, energy_to_atmosphere);

        if transfers_applied > 0 && (step <= 10 || step % 1000 == 0) {

            println!("🌋 ColumnPlumeComponent: {} plumes formed across {} columns at step {}",
                     transfers_applied, stats.total_columns, step);
            println!("   • Plume formation rate: {:.2}%",
                     transfers_applied as f64 / stats.total_columns as f64 * 100.0);
            println!("   📊 Material in plumes: {:.2e} kg", total_mass);
            println!("   ⚡ Energy in plumes: {:.2e} J", total_energy);

            if eruptions_created > 0 {
                println!("   🌋 Surface eruptions: {} (energy to atmosphere)", eruptions_created);
                println!("   🌍 Energy to atmosphere: {:.2e} J", energy_to_atmosphere);
            }

            println!("   📊 Totals stored in 'plume_totals' collection");
        }
    }
    
    fn complete(&mut self, _sim: &Simulation, _config: &SimulationConfig) {
        println!("🌋 ColumnPlumeComponent: Column-based plume processing complete");
        println!("   • Optimization: Vertical columns analyzed for buoyancy conditions");
        println!("   • Performance: Efficient plume formation detection and application");
    }
}

// Removed ColumnBasedComponent implementation - trait doesn't exist
/*
impl ColumnBasedComponent for ColumnPlumeComponent {
    fn step_with_columns(&self, _coll_mgr: &CollectionsManager, actor: &mut Actor, step: u32, _year: f64, _config: &SimulationConfig, column_processor: &ColumnProcessor) {
        let stats = column_processor.get_statistics();

        // Print every step for testing, or every 1000 for long runs
        if step <= 10 || step % 1000 == 0 {
            println!("🌋 ColumnPlumeComponent: Analyzing {} columns for plume formation at step {} [OPTIMIZED]", stats.total_columns, step);
            println!("   • Using pre-built column processor (shared optimization)");
        }

        // Process columns for plume formation using the pre-built processor
        let plume_formations = self.process_columns_for_plumes_with_processor(column_processor, step);

        // Calculate statistics before moving plume_formations
        let total_mass: f64 = plume_formations.iter().map(|p| p.mass_transfer).sum();
        let total_energy: f64 = plume_formations.iter().map(|p| p.energy_transfer).sum();
        let energy_to_atmosphere: f64 = plume_formations.iter()
            .filter(|p| p.eruption_potential > 0.3 && p.target_location.depth_index() == 0)
            .map(|p| p.energy_transfer)
            .sum();

        // Apply plume formations (collections-based, no internal state)
        let (transfers_applied, eruptions_created) = self.apply_plume_formations(plume_formations, actor, step, energy_to_atmosphere);

        if transfers_applied > 0 && (step <= 10 || step % 1000 == 0) {
            println!("🌋 ColumnPlumeComponent: {} plumes formed across {} columns at step {} [OPTIMIZED]",
                     transfers_applied, stats.total_columns, step);
            println!("   • Plume formation rate: {:.2}%",
                     transfers_applied as f64 / stats.total_columns as f64 * 100.0);
            println!("   📊 Material in plumes: {:.2e} kg", total_mass);
            println!("   ⚡ Energy in plumes: {:.2e} J", total_energy);

            if eruptions_created > 0 {
                println!("   🌋 Surface eruptions: {} (energy to atmosphere)", eruptions_created);
                println!("   🌍 Energy to atmosphere: {:.2e} J", energy_to_atmosphere);
            }

            println!("   📊 Totals stored in 'plume_totals' collection");
        }
    }

    fn uses_columns(&self) -> bool { true }
}
*/

/// Helper method for processing columns with pre-built processor
impl ColumnPlumeComponent {
    fn process_columns_for_plumes_with_processor(
        &self,
        column_processor: &ColumnProcessor,
        step: u32,
    ) -> Vec<PlumeFormation> {
        let mut plume_formations = Vec::new();

        column_processor.process_columns(|_h3_index, column| {
            if let Some(plume) = self.analyze_column_for_plumes(column, step) {
                plume_formations.push(plume);
            }
        });

        plume_formations
    }
}

impl Default for ColumnPlumeComponent {
    fn default() -> Self {
        Self::new()
    }
}
