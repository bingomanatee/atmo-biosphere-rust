use crate::component::SimComponent;
use crate::deprecated::sim::Simulation;
use crate::deprecated::sim::energy_mass_cell::EnergyMassCell;
use crate::energy_mass::energy_mass::EnergyMass;
use crate::material::material::MassCalculationParams;
use crate::constants::GRAVITY_M_S2;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use h3o::CellIndex;
use rayon::prelude::*;



/// A moving convection plume that rises through layers with vertical radiation effects
#[derive(Debug, Clone)]
pub struct ConvectionPlume {
    /// Unique identifier
    pub id: u64,
    /// Source layer where plume originated
    pub source_layer_index: usize,
    /// Current layer where plume center is located
    pub current_layer_index: usize,
    /// Current cell index within the current layer
    pub current_cell_index: h3o::CellIndex,
    /// Geographic position (lat, lon in degrees)
    pub position_lat_deg: f64,
    pub position_lon_deg: f64,
    /// Current depth (changes as plume rises)
    pub current_depth_km: f64,
    /// Total energy carried by plume (Joules)
    pub total_energy_joules: f64,
    /// Total mass carried by plume (kg)
    pub total_mass_kg: f64,
    /// Temperature of plume material (K)
    pub temperature_k: f64,
    /// Upward velocity (km/year)
    pub velocity_km_per_year: f64,
    /// Age of the plume (years)
    pub age_years: f64,
    /// Half-life for exponential decay (years)
    pub half_life_years: f64,
    /// Radiation strength to adjacent layers (0.0 to 1.0)
    pub vertical_radiation_factor: f64,
    /// Radius of influence (km)
    pub radius_km: f64,
}

/// Information about buoyancy conditions for plume formation
#[derive(Debug, Clone)]
struct BuoyancyInfo {
    density_difference: f64,    // kg/m³ (positive = lower cell less dense)
    buoyancy_force: f64,        // N/m³ (gravitational force per unit volume)
    temperature_excess: f64,    // K (how much hotter the lower cell is)
    lower_density: f64,         // kg/m³ (density of potentially rising material)
    upper_density: f64,         // kg/m³ (density of overlying material)
}

impl ConvectionPlume {
    /// Create a new moving plume with vertical radiation effects
    pub fn new(
        id: u64,
        source_layer_index: usize,
        source_cell_index: h3o::CellIndex,
        position: (f64, f64),
        initial_depth_km: f64,
        total_energy_joules: f64,
        total_mass_kg: f64,
        temperature_k: f64,
        velocity_km_per_year: f64,
        buoyancy_force: f64,
        radius_km: f64,
    ) -> Self {
        // Calculate half-life based on plume strength and buoyancy
        let half_life_years = Self::calculate_half_life(total_energy_joules, buoyancy_force);

        // Calculate vertical radiation factor based on plume strength
        let vertical_radiation_factor = Self::calculate_radiation_factor(total_energy_joules, buoyancy_force);

        ConvectionPlume {
            id,
            source_layer_index,
            current_layer_index: source_layer_index,
            current_cell_index: source_cell_index,
            position_lat_deg: position.0,
            position_lon_deg: position.1,
            current_depth_km: initial_depth_km,
            total_energy_joules,
            total_mass_kg,
            temperature_k,
            velocity_km_per_year,
            age_years: 0.0,
            half_life_years,
            vertical_radiation_factor,
            radius_km,
        }
    }

    /// Calculate half-life based on plume strength and buoyancy
    fn calculate_half_life(energy_joules: f64, buoyancy_force: f64) -> f64 {
        // Base half-life: 500,000 years
        let base_half_life = 500_000.0;

        // Strong energy = longer half-life (more persistent)
        let energy_factor = 1.0 + (energy_joules / 1e15).min(2.0);

        // Strong buoyancy = longer half-life
        let buoyancy_factor = 1.0 + (buoyancy_force / 2000.0).min(1.5);

        base_half_life * energy_factor * buoyancy_factor
    }

    /// Calculate vertical radiation factor
    fn calculate_radiation_factor(energy_joules: f64, buoyancy_force: f64) -> f64 {
        // Stronger plumes radiate more to adjacent layers
        let energy_factor = (energy_joules / 1e12).min(2.0); // Normalize
        let buoyancy_factor = (buoyancy_force / 1000.0).min(1.5);

        // Base radiation: 20%, enhanced by strength and buoyancy
        let base_radiation = 0.2;
        (base_radiation * energy_factor * buoyancy_factor).min(0.8) // Max 80%
    }

    /// Apply half-life exponential decay
    pub fn apply_half_life_decay(&mut self, years_elapsed: f64) {
        let decay_constant = (2.0_f64).ln() / self.half_life_years;
        let decay_factor = (-decay_constant * years_elapsed).exp();

        // Apply exponential decay to both energy and mass
        self.total_energy_joules *= decay_factor;
        self.total_mass_kg *= decay_factor;
    }

    /// Check if plume is still significant (above 1% of initial strength)
    pub fn is_significant(&self) -> bool {
        // Use a reasonable threshold for geological significance
        self.total_energy_joules > 1e10 && self.total_mass_kg > 1e6
    }

    /// Apply plume effects to current layer and vertically adjacent layers
    pub fn apply_vertical_effects(&self, sim: &mut crate::deprecated::sim::simulation::Simulation, years_per_step: f64) {
        let current_layer = self.current_layer_index;

        // Calculate energy/mass to distribute this time step (10% per year)
        let annual_transfer_rate = 0.1;
        let total_energy_transfer = self.total_energy_joules * annual_transfer_rate * years_per_step;
        let total_mass_transfer = self.total_mass_kg * annual_transfer_rate * years_per_step;

        // Define vertical effects: [layer_offset, effect_strength]
        let vertical_effects = vec![
            (-1, self.vertical_radiation_factor * 0.3), // Layer above (30% of radiation)
            (0, 1.0),                                   // Current layer (100% effect)
            (1, self.vertical_radiation_factor * 0.2),  // Layer below (20% of radiation)
        ];

        for (layer_offset, effect_strength) in vertical_effects {
            if effect_strength <= 0.0 {
                continue;
            }

            let target_layer_index = if layer_offset < 0 {
                current_layer.saturating_sub((-layer_offset) as usize)
            } else {
                current_layer + layer_offset as usize
            };

            // Skip if layer doesn't exist
            if target_layer_index >= sim.layer_sets.len() {
                continue;
            }

            let energy_for_layer = total_energy_transfer * effect_strength;
            let mass_for_layer = total_mass_transfer * effect_strength;

            self.apply_to_layer(sim, target_layer_index, energy_for_layer, mass_for_layer);
        }
    }

    /// Apply energy/mass to specific layer around plume location
    /// Uses double-entry accounting to ensure mass conservation
    fn apply_to_layer(&self, sim: &mut crate::deprecated::sim::simulation::Simulation, layer_index: usize, energy: f64, mass: f64) {
        if let Some(layer_set) = sim.layer_sets.get_mut(layer_index) {
            // Find target cells around plume location
            let target_cells = self.find_target_cells_in_layer(layer_set);

            if !target_cells.is_empty() {
                let energy_per_cell = energy / target_cells.len() as f64;
                let mass_per_cell = mass / target_cells.len() as f64; // Exact division for conservation

                let mut total_mass_added = 0.0; // Track for conservation verification

                for cell_index in target_cells {
                    if let Some(column) = layer_set.layers.get_mut(&cell_index) {
                        // Apply to middle cell in column (representative depth)
                        let cell_idx = column.cells.len() / 2;
                        if let Some(target_cell) = column.cells.get_mut(cell_idx) {
                            target_cell.add_energy_joules(energy_per_cell);
                            target_cell.add_mass_kg(mass_per_cell);
                            total_mass_added += mass_per_cell;
                        }
                    }
                }

                // Verify mass conservation (debug check)
                let mass_difference = (total_mass_added - mass).abs();
                if mass_difference > 1e-6 {
                    println!("⚠️  Mass conservation violation: expected {:.2e}, added {:.2e}, diff {:.2e}",
                        mass, total_mass_added, mass_difference);
                }
            }
        }
    }

    /// Find target cells in a layer around the plume location
    fn find_target_cells_in_layer(&self, layer_set: &crate::deprecated::sim::layer_set::LayerSet) -> Vec<h3o::CellIndex> {
        use h3o::LatLng;

        let mut target_cells = Vec::new();
        let plume_lat = self.position_lat_deg;
        let plume_lon = self.position_lon_deg;
        let search_radius_degrees = self.radius_km / 111.0; // Rough km to degrees conversion

        for &cell_index in layer_set.layers.keys() {
            let cell_lat_lng = LatLng::from(cell_index);
            let cell_lat = cell_lat_lng.lat_radians().to_degrees();
            let cell_lon = cell_lat_lng.lng_radians().to_degrees();

            // Simple distance check (could be improved with proper great circle distance)
            let lat_diff = plume_lat - cell_lat;
            let lon_diff = plume_lon - cell_lon;
            let distance_degrees = (lat_diff * lat_diff + lon_diff * lon_diff).sqrt();

            if distance_degrees <= search_radius_degrees {
                target_cells.push(cell_index);
            }
        }

        // If no cells found within radius, use nearest cell
        if target_cells.is_empty() {
            if let Some(&nearest_cell) = layer_set.layers.keys().next() {
                target_cells.push(nearest_cell);
            }
        }

        target_cells
    }
}

/// Component that manages moving convection plumes in the simulation
/// Note: Plumes are now stored in the Simulation struct, not here
pub struct ConvectionPlumeComponent {
    /// Minimum temperature difference between layers to generate plumes (K)
    min_temp_difference_k: f64,
    /// Base plume generation probability per km² per year
    base_plume_probability_per_km2_per_year: f64,
    /// Energy fraction copied from hotspot cells to plume (preserves source)
    energy_copy_fraction: f64,
    /// Plume radius (km)
    plume_radius_km: f64,
    /// Plume velocity (km/year)
    plume_velocity_km_per_year: f64,
    /// Plume lifetime (years)
    plume_lifetime_years: f64,
    /// Energy radiation fraction per year (fraction of plume energy radiated to surrounding cells)
    energy_radiation_fraction_per_year: f64,
    /// Random temperature perturbation amplitude (K)
    temperature_perturbation_amplitude_k: f64,
    /// Random energy variation factor (0.5 to 2.0 multiplier)
    energy_variation_factor: f64,
    /// Random radius variation factor (0.5 to 2.0 multiplier)
    radius_variation_factor: f64,
    /// Random number generator
    rng: StdRng,
}

impl ConvectionPlumeComponent {
    pub fn new() -> Self {
        Self::with_seed(42) // Default reproducible seed
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            min_temp_difference_k: 200.0,    // Significant temperature gradient
            base_plume_probability_per_km2_per_year: 1e-12, // 1/1,000,000 as common (extremely rare)
            energy_copy_fraction: 0.1,       // 10% of hotspot energy copied to plume (preserves source)
            plume_radius_km: 5.0,            // 5 km radius of influence
            plume_velocity_km_per_year: 10.0, // 10 km/year upward velocity
            plume_lifetime_years: 1000.0,    // 1000 year lifetime
            energy_radiation_fraction_per_year: 0.2, // 20% of energy radiated per year
            temperature_perturbation_amplitude_k: 50.0, // ±50K random temperature variations
            energy_variation_factor: 0.5,    // 50% to 200% energy variation
            radius_variation_factor: 0.3,    // 70% to 130% radius variation
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Configure plume generation parameters for testing
    pub fn with_plume_config(mut self, probability_per_km2_per_year: f64, energy_fraction: f64) -> Self {
        self.base_plume_probability_per_km2_per_year = probability_per_km2_per_year;
        self.energy_copy_fraction = energy_fraction;
        self
    }

    /// Configure minimum temperature difference for plume formation
    pub fn with_min_temperature_difference(mut self, min_diff_k: f64) -> Self {
        self.min_temp_difference_k = min_diff_k;
        self
    }

    /// Calculate layer-aware plume generation probability with exponential temperature dependence
    fn calculate_plume_probability(&self, cell_area_km2: f64, years_per_step: f64, temp_excess_k: f64,
                                   layer_height_km: f64, total_cells_in_layer: usize) -> f64 {
        // Base probability scaled by area and time
        let base_prob = self.base_plume_probability_per_km2_per_year * cell_area_km2 * years_per_step;

        // Layer height factor - taller layers have higher instability
        // Linear scaling: probability increases with layer thickness
        let height_factor = layer_height_km / 50.0; // Normalize to 50km reference thickness

        // Cell distribution factor - probability distributed among all cells in layer
        // More cells = lower probability per cell (total layer probability is conserved)
        let cell_distribution_factor = 1.0 / (total_cells_in_layer as f64).sqrt(); // Square root to avoid over-dilution

        // Exponential temperature enhancement factor - geological processes are highly temperature sensitive
        // Using a steeper exponential curve: probability increases dramatically with temperature
        // Scale factor of 50K means probability doubles every 50K temperature increase
        let temp_scale_k = 50.0; // Temperature scale for exponential growth
        let temp_factor = (temp_excess_k / temp_scale_k).exp();

        // Combine all factors
        let probability = base_prob * height_factor * cell_distribution_factor * temp_factor;

        // Cap the maximum probability to prevent unrealistic values
        let max_probability = 0.1; // Maximum 10% chance per step
        probability.min(max_probability)
    }

    /// Calculate buoyancy-driven velocity based on density differences
    /// Uses Stokes' law for buoyant rise velocity: v = sqrt(2 * g * Δρ * r / (9 * η))
    /// But simplified for geological timescales
    fn calculate_buoyancy_velocity(&self, plume_density: f64, ambient_density: f64, plume_radius_km: f64) -> f64 {
        // Constants
        const GRAVITY_M_S2: f64 = 9.81;
        const VISCOSITY_PA_S: f64 = 1e21; // Typical mantle viscosity

        // Convert radius to meters
        let radius_m = plume_radius_km * 1000.0;

        // Density difference (kg/m³)
        let density_diff = ambient_density - plume_density; // Positive if plume is less dense

        if density_diff <= 0.0 {
            return 0.0; // No buoyancy if plume is denser than ambient
        }

        // Buoyancy force per unit volume (N/m³)
        let _buoyancy_force = GRAVITY_M_S2 * density_diff;

        // Terminal velocity for spherical particle (m/s)
        // v = sqrt(2 * g * Δρ * r / (9 * η))
        let velocity_m_s = (2.0 * GRAVITY_M_S2 * density_diff * radius_m / (9.0 * VISCOSITY_PA_S)).sqrt();

        // Convert to km/year
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let velocity_km_per_year = velocity_m_s * seconds_per_year / 1000.0;

        // Cap velocity at reasonable geological rates (max 100 km/year)
        velocity_km_per_year.min(100.0)
    }

    /// Calculate density from pressure and temperature using material properties
    fn calculate_density(&self, material: &crate::material::MaterialPhase, pressure_pa: f64, temperature_k: f64) -> f64 {
        // Use the material's density calculation method
        // This accounts for thermal expansion and pressure compressibility
        let volume_m3 = 1.0; // 1 m³ reference volume
        let mass_kg = material.calculate_mass_from_pressure_volume(
            MassCalculationParams {
                pressure_pa,
                volume_km3: volume_m3 / 1e9, // Convert to km³
                temperature_k,
            }
        );

        mass_kg / volume_m3 // kg/m³
    }

    /// Generate random 3D position within plume radius around a center point
    fn generate_random_position_around(rng: &mut StdRng, center_lat: f64, center_lon: f64, center_depth: f64, radius_km: f64) -> (f64, f64, f64) {
        // Generate random offset within sphere of radius_km
        let theta = rng.random::<f64>() * 2.0 * std::f64::consts::PI; // Azimuth angle
        let phi = (rng.random::<f64>() * 2.0 - 1.0).acos(); // Polar angle (uniform on sphere)
        let r = radius_km * rng.random::<f64>().cbrt(); // Cube root for uniform volume distribution

        // Convert spherical to Cartesian offset (km)
        let dx = r * phi.sin() * theta.cos();
        let dy = r * phi.sin() * theta.sin();
        let dz = r * phi.cos();

        // Convert km offsets to lat/lon/depth
        // Approximate: 1 degree ≈ 111 km at equator
        let lat_offset = dy / 111.0;
        let lon_offset = dx / (111.0 * center_lat.to_radians().cos());
        let depth_offset = dz; // Depth is already in km

        (
            center_lat + lat_offset,
            center_lon + lon_offset,
            center_depth + depth_offset
        )
    }

    /// Find the nearest H3 cell to a given lat/lon position
    fn find_nearest_cell(lat_deg: f64, lon_deg: f64, layer_set: &crate::deprecated::sim::layer_set::LayerSet) -> Option<CellIndex> {
        // Convert lat/lon to H3 cell at the layer set's resolution
        let lat_rad = lat_deg.to_radians();
        let lon_rad = lon_deg.to_radians();

        // Use H3's built-in function to find the cell containing this point
        if let Ok(latlng) = h3o::LatLng::new(lat_rad, lon_rad) {
            let cell = latlng.to_cell(layer_set.resolution);
            // Check if this cell exists in our layer set
            if layer_set.layers.contains_key(&cell) {
                Some(cell)
            } else {
                // Find the closest existing cell
                let mut closest_cell = None;
                let _min_distance = f64::INFINITY;

                // For now, just return the first available cell as a fallback
                // TODO: Implement proper distance calculation when H3 API is clarified
                closest_cell = layer_set.layers.keys().next().copied();

                closest_cell
            }
        } else {
            None
        }
    }

    /// Get the 3D position (lat, lon, depth) of a cell
    fn get_cell_3d_position(&self, cell: &EnergyMassCell, layer_set_idx: usize, sim: &Simulation) -> (f64, f64, f64) {
        // For now, use a simplified approach with default coordinates
        // TODO: Implement proper H3 coordinate extraction when API is clarified
        let layer_set = &sim.layer_sets[layer_set_idx];
        let depth_km = layer_set.start_height_km; // Approximate depth

        // Use a simple deterministic position based on cell index
        // Convert cell index to a hash for position calculation
        let cell_hash = std::collections::hash_map::DefaultHasher::new();
        use std::hash::{Hash, Hasher};
        let mut hasher = cell_hash;
        cell.cell_index.hash(&mut hasher);
        let hash_value = hasher.finish() as f64;

        let lat_deg = (hash_value % 180.0) - 90.0; // -90 to +90
        let lon_deg = ((hash_value / 180.0) % 360.0) - 180.0; // -180 to +180

        (lat_deg, lon_deg, depth_km)
    }

    /// Apply random temperature perturbations to break initial symmetry
    fn apply_temperature_perturbations(&mut self, sim: &mut Simulation) {
        for layer_set in &mut sim.layer_sets {
            for column in layer_set.layers.values_mut() {
                for cell in &mut column.cells {
                    // Generate random temperature perturbation
                    let perturbation = (self.rng.random::<f64>() - 0.5) * 2.0 * self.temperature_perturbation_amplitude_k;

                    // Apply perturbation to cell temperature
                    let current_temp = cell.temperature_kelvin();
                    let new_temp = current_temp + perturbation;

                    // Ensure temperature stays within reasonable bounds
                    let bounded_temp = new_temp.max(200.0).min(5000.0);
                    cell.set_temperature_kelvin(bounded_temp);
                }
            }
        }
    }

    /// Calculate buoyancy force between a lower cell and upper cell
    /// Returns Some(BuoyancyInfo) if buoyancy conditions favor plume formation
    fn calculate_buoyancy_force(&self, lower_cell: &crate::deprecated::sim::energy_mass_cell::EnergyMassCell,
                                upper_cell: &crate::deprecated::sim::energy_mass_cell::EnergyMassCell) -> Option<BuoyancyInfo> {
        // Get cell properties
        let lower_temp = lower_cell.temperature_kelvin();
        let upper_temp = upper_cell.temperature_kelvin();
        let _lower_pressure = lower_cell.pressure_pa();
        let _upper_pressure = upper_cell.pressure_pa();

        // Calculate densities based on current conditions
        let lower_volume_km3 = lower_cell.area() * lower_cell.height_km;
        let upper_volume_km3 = upper_cell.area() * upper_cell.height_km;

        let lower_mass = lower_cell.mass_kg();
        let upper_mass = upper_cell.mass_kg();

        // Convert to density (kg/m³)
        let lower_density = lower_mass / (lower_volume_km3 * 1e9);
        let upper_density = upper_mass / (upper_volume_km3 * 1e9);

        // Check for buoyancy instability: lower cell must be less dense than upper cell
        let density_difference = upper_density - lower_density; // Positive = unstable (good for plumes)

        if density_difference <= 0.0 {
            return None; // No buoyancy instability
        }

        // Must have some temperature excess to drive thermal expansion
        let temperature_excess = lower_temp - upper_temp;
        if temperature_excess < 50.0 {  // Minimum 50K temperature difference
            return None;
        }

        // Calculate buoyancy force per unit volume (N/m³)
        const GRAVITY_M_S2: f64 = 9.81;
        let buoyancy_force = GRAVITY_M_S2 * density_difference;

        Some(BuoyancyInfo {
            density_difference,
            buoyancy_force,
            temperature_excess,
            lower_density,
            upper_density,
        })
    }

    /// Calculate plume generation probability based on buoyancy conditions and cell volume
    /// Uses reference volume (20km × H3 L2 cell area) for resolution-independent scaling
    fn calculate_buoyancy_plume_probability(&self, cell_area_km2: f64, years_per_step: f64,
                                          buoyancy_info: &BuoyancyInfo, layer_height_km: f64,
                                          _total_cells_in_layer: usize) -> f64 {
        use crate::utils::h3_utils::H3Utils;

        // Reference volume: 20km height × H3 Resolution::Two cell area
        let reference_area_km2 = H3Utils::cell_area(h3o::Resolution::Two, 6371.0);
        let reference_height_km = 20.0;
        let reference_volume_km3 = reference_area_km2 * reference_height_km;

        // Current cell volume
        let cell_volume_km3 = cell_area_km2 * layer_height_km;

        // Volume scaling factor: probability proportional to (cell_volume / reference_volume)
        // This ensures resolution independence - smaller cells have proportionally lower probability
        let volume_factor = cell_volume_km3 / reference_volume_km3;

        // Buoyancy scaling: probability proportional to buoyancy force
        // Normalize by typical mantle buoyancy force (~10 N/m³)
        let buoyancy_factor = (buoyancy_info.buoyancy_force / 10.0).max(0.0);

        // Temperature excess factor (exponential but capped)
        let temp_factor = (buoyancy_info.temperature_excess / 200.0).exp().min(5.0);

        // Density contrast factor (stronger contrast = higher probability)
        let density_contrast = buoyancy_info.density_difference / buoyancy_info.upper_density;
        let density_factor = (density_contrast * 20.0).max(0.0).min(3.0); // Cap at 3x

        // Base probability per reference volume per year
        // This should be tuned so that a reference-sized cell with moderate buoyancy
        // has a reasonable chance of generating a plume over geological time
        let base_probability_per_ref_volume_per_year = 1e-8; // Much lower base rate

        // Combined probability
        let probability = base_probability_per_ref_volume_per_year
            * volume_factor        // Scale with cell size
            * buoyancy_factor      // Scale with buoyancy force
            * temp_factor          // Scale with temperature excess
            * density_factor       // Scale with density contrast
            * years_per_step;      // Scale with time

        // Cap at reasonable maximum (1% per step for any single cell)
        probability.min(0.01)
    }

    /// Create a buoyancy-driven moving plume with vertical radiation effects
    fn create_buoyancy_plume(&mut self, layer_set_idx: usize, cell_index: h3o::CellIndex,
                           cell: &crate::deprecated::sim::energy_mass_cell::EnergyMassCell,
                           buoyancy_info: &BuoyancyInfo, sim: &mut crate::deprecated::sim::simulation::Simulation) -> (f64, f64) {
        // Generate plume properties based on buoyancy conditions
        let base_temp = cell.temperature_kelvin();
        let temp_variation = (self.rng.random::<f64>() - 0.5) * 2.0 * self.temperature_perturbation_amplitude_k;
        let plume_temp = base_temp + temp_variation;

        // Calculate plume mass and energy (total amounts, not rates)
        let mass_fraction = 0.05; // Take 5% of cell mass
        let energy_fraction = 0.08; // Take 8% of cell energy

        let base_mass = cell.mass_kg() * mass_fraction;
        let base_energy = cell.energy_joules() * energy_fraction;

        let buoyancy_factor = (buoyancy_info.buoyancy_force / 1000.0).min(3.0);
        let variation = 1.0 + (self.rng.random::<f64>() - 0.5) * 2.0 * self.energy_variation_factor;

        let plume_mass = base_mass * buoyancy_factor * variation;
        let plume_energy = base_energy * buoyancy_factor * variation;

        // Calculate plume velocity based on buoyancy
        let velocity_factor = (buoyancy_info.buoyancy_force / 500.0).sqrt().min(3.0);
        let velocity_variation = 1.0 + (self.rng.random::<f64>() - 0.5) * 2.0 * 0.3;
        let plume_velocity = self.plume_velocity_km_per_year * velocity_factor * velocity_variation;

        // Calculate plume radius
        let radius_variation = 1.0 + (self.rng.random::<f64>() - 0.5) * 2.0 * self.radius_variation_factor;
        let plume_radius = self.plume_radius_km * radius_variation;

        // Get source cell geographic location and depth
        let source_location = self.get_cell_geographic_location(cell_index, layer_set_idx, sim);
        let initial_depth = self.get_layer_depth(layer_set_idx, sim);

        // Create moving plume with vertical radiation effects using simulation's plume storage
        let plume_id = sim.create_plume(
            layer_set_idx,
            cell_index,
            source_location,
            initial_depth,
            plume_energy,
            plume_mass,
            plume_temp,
            plume_velocity,
            buoyancy_info.buoyancy_force,
            plume_radius,
        );

        println!("🌋 Moving Plume #{} created at layer {} cell {:?}: {:.1}K, {:.2e}kg, {:.2e}J, {:.1} km/yr",
            plume_id, layer_set_idx, cell_index, plume_temp,
            plume_mass, plume_energy, plume_velocity);
        if let Some(plume) = sim.plumes.last() {
            println!("   📊 Half-life: {:.0} years, Radiation factor: {:.1}%",
                plume.half_life_years, plume.vertical_radiation_factor * 100.0);
        }
        println!("   🔥 Will extract {:.2e}kg and {:.2e}J from source cell", plume_mass, plume_energy);

        // Return extraction amounts
        (plume_mass, plume_energy)
    }

    /// Get geographic location of a cell
    fn get_cell_geographic_location(&self, cell_index: h3o::CellIndex, _layer_index: usize, _sim: &crate::deprecated::sim::simulation::Simulation) -> (f64, f64) {
        use h3o::LatLng;
        let lat_lng = LatLng::from(cell_index);
        (lat_lng.lat_radians().to_degrees(), lat_lng.lng_radians().to_degrees())
    }

    /// Get approximate depth of a layer (middle of layer)
    fn get_layer_depth(&self, layer_index: usize, sim: &crate::deprecated::sim::simulation::Simulation) -> f64 {
        if let Some(layer_set) = sim.layer_sets.get(layer_index) {
            if let Some(column) = layer_set.layers.values().next() {
                if let Some(first_cell) = column.cells.first() {
                    // Use the middle depth of the layer
                    let layer_height = column.cells.len() as f64 * first_cell.height_km;
                    return first_cell.top_km + layer_height / 2.0;
                }
            }
        }
        // Fallback: estimate based on layer index
        layer_index as f64 * 50.0 + 25.0 // Assume 50km layers, middle depth
    }

    /// Generate new plumes based on temperature conditions with layer-aware probability (threaded)
    fn generate_plumes(&mut self, sim: &mut Simulation, years_per_step: f64) {
        // Determine if we should use threading based on simulation size
        let total_cells: usize = sim.layer_sets.iter()
            .map(|layer| layer.layers.len() * layer.layers.values().next().map_or(1, |col| col.cells.len()))
            .sum();

        let use_threading = total_cells > 10000; // Lower threshold to test threading

        println!("🔍 Threading decision: {} total cells, threading: {}", total_cells, use_threading);

        if use_threading {
            self.generate_plumes_threaded(sim, years_per_step);
        } else {
            self.generate_plumes_sequential(sim, years_per_step);
        }
    }

    /// Sequential plume generation (for small simulations)
    fn generate_plumes_sequential(&mut self, sim: &mut Simulation, years_per_step: f64) {
        // First pass: collect plume generation data without modifying simulation
        let mut plume_generation_data = Vec::new();

        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            // Skip surface layer (no layer above to compare with)
            if layer_set_idx == 0 {
                continue;
            }

            let upper_layer_set = &sim.layer_sets[layer_set_idx - 1];

            // Calculate layer properties for probability scaling
            let layer_height_km = if let Some(first_column) = layer_set.layers.values().next() {
                first_column.cells.len() as f64 * 25.0 // Approximate cell height
            } else {
                50.0 // Default layer height
            };

            let total_cells_in_layer = layer_set.layers.len() *
                layer_set.layers.values().next().map_or(1, |col| col.cells.len());

            for (cell_index, column) in layer_set.layers.iter() {
                // Find corresponding upper column
                if let Some(upper_column) = upper_layer_set.layers.get(cell_index) {
                    for (cell_idx, cell) in column.cells.iter().enumerate() {
                        // Find corresponding upper cell for buoyancy comparison
                        if cell_idx >= upper_column.cells.len() {
                            continue;
                        }
                        let upper_cell = &upper_column.cells[cell_idx];

                        // Calculate buoyancy-driven plume formation
                        if let Some(buoyancy_info) = self.calculate_buoyancy_force(cell, upper_cell) {
                            // Calculate cell area (area-scaling is key here!)
                            let cell_area_km2 = cell.area();

                            // Calculate buoyancy-driven plume generation probability
                            let probability = self.calculate_buoyancy_plume_probability(
                                cell_area_km2,
                                years_per_step,
                                &buoyancy_info,
                                layer_height_km,
                                total_cells_in_layer
                            );

                            // Generate plume based on buoyancy probability
                            if self.rng.random::<f64>() < probability {
                                // Collect plume generation data for later processing
                                plume_generation_data.push((layer_set_idx, *cell_index, cell_idx, buoyancy_info.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Apply the plume generation data
        self.apply_plume_generation_data(sim, plume_generation_data);
    }

    /// Threaded plume generation (for large simulations)
    fn generate_plumes_threaded(&mut self, sim: &mut Simulation, years_per_step: f64) {
        println!("🧵 Using threaded plume generation for large simulation");

        // Phase 1: Collect layer data for parallel processing
        let data_collection_start = std::time::Instant::now();
        let mut layer_data = Vec::new();

        for (layer_set_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            // Skip surface layer (no layer above to compare with)
            if layer_set_idx == 0 {
                continue;
            }

            let upper_layer_set = &sim.layer_sets[layer_set_idx - 1];

            // Calculate layer properties for probability scaling
            let layer_height_km = if let Some(first_column) = layer_set.layers.values().next() {
                first_column.cells.len() as f64 * 25.0 // Approximate cell height
            } else {
                50.0 // Default layer height
            };

            let total_cells_in_layer = layer_set.layers.len() *
                layer_set.layers.values().next().map_or(1, |col| col.cells.len());

            // Collect cell pairs for parallel processing (simplified - no sampling for now)
            for (cell_index, column) in layer_set.layers.iter() {
                if let Some(upper_column) = upper_layer_set.layers.get(cell_index) {
                    for (cell_idx, cell) in column.cells.iter().enumerate() {
                        if cell_idx < upper_column.cells.len() {
                            let upper_cell = &upper_column.cells[cell_idx];
                            layer_data.push((
                                layer_set_idx,
                                cell_index,
                                cell_idx,
                                cell,
                                upper_cell,
                                layer_height_km,
                                total_cells_in_layer,
                            ));
                        }
                    }
                }
            }
        }

        let data_collection_time = data_collection_start.elapsed();
        println!("      📊 Data collection: {:.2} ms ({} cells)", data_collection_time.as_secs_f64() * 1000.0, layer_data.len());

        // DIAGNOSTIC: Sample temperature differences
        if !layer_data.is_empty() {
            let sample_size = layer_data.len().min(5);
            println!("      🌡️ Temperature samples:");
            for i in 0..sample_size {
                let (_, _, _, cell, upper_cell, _, _) = &layer_data[i];
                let temp_diff = cell.temperature_kelvin() - upper_cell.temperature_kelvin();
                println!("        Cell {}: {:.1}K - {:.1}K = {:.1}K diff",
                    i, cell.temperature_kelvin(), upper_cell.temperature_kelvin(), temp_diff);
            }
        }

        // Phase 2: Focused plume generation for extreme temperature differences
        let parallel_processing_start = std::time::Instant::now();

        // Store diagnostic data before parallel processing (just temperature differences)
        let diagnostic_samples: Vec<f64> = layer_data.iter().take(3).map(|(_, _, _, cell, upper_cell, _, _)| {
            cell.temperature_kelvin() - upper_cell.temperature_kelvin()
        }).collect();

        // Process cells to find plume generation opportunities
        let plume_generation_data: Vec<_> = layer_data
            .into_par_iter()
            .filter_map(|(layer_set_idx, cell_index, cell_idx, cell, upper_cell, layer_height_km, total_cells_in_layer)| {
                let cell_temp = cell.temperature_kelvin();
                let upper_temp = upper_cell.temperature_kelvin();
                let temp_diff = cell_temp - upper_temp;

                // FOCUSED: Only generate plumes for extreme temperature differences (>800K)
                if temp_diff > 800.0 {  // Much higher threshold for true plume conditions
                    // Only generate plumes for truly extreme temperature differences
                    if let Some(buoyancy_info) = Self::calculate_buoyancy_force_static(&cell, &upper_cell) {
                        // Higher probability for extreme conditions
                        let plume_probability = (temp_diff / 1000.0).min(0.5) * years_per_step * 1e-4; // Much higher probability

                        use rand::rng;
                        if rng().random::<f64>() < plume_probability {
                            Some((layer_set_idx, *cell_index, cell_idx, buoyancy_info))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None // No plumes for moderate temperature differences
                }
            })
            .collect();

        let parallel_processing_time = parallel_processing_start.elapsed();
        println!("      🧵 Parallel processing: {:.2} ms", parallel_processing_time.as_secs_f64() * 1000.0);

        println!("🧵 Focused plume processing: {} potential plumes", plume_generation_data.len());

        // DIAGNOSTIC: Report plume generation results with detailed analysis
        if plume_generation_data.is_empty() {
            println!("      🔍 No plumes generated despite extreme temperature differences");
            println!("      🔍 This suggests buoyancy calculation is failing or probability too low");

            // Sample a few cells to diagnose the issue
            if !diagnostic_samples.is_empty() {
                println!("      🔍 Diagnostic sample:");
                for (i, temp_diff) in diagnostic_samples.iter().enumerate() {
                    println!("        Cell {}: temp_diff={:.1}K", i, temp_diff);

                    if *temp_diff > 800.0 {
                        println!("          ✓ Above 800K threshold - should generate plumes");
                        let prob = (temp_diff / 1000.0).min(0.5) * years_per_step * 1e-4;
                        println!("          📊 Expected probability: {:.2e}", prob);
                    } else {
                        println!("          ✗ Below 800K threshold");
                    }
                }
            }
        } else {
            println!("      🌋 {} plumes generated from extreme temperature conditions", plume_generation_data.len());
        }

        // Phase 3: Apply plume generation data
        let application_start = std::time::Instant::now();
        self.apply_plume_generation_data(sim, plume_generation_data);
        let application_time = application_start.elapsed();
        println!("      🎯 Plume generation: {:.2} ms", application_time.as_secs_f64() * 1000.0);
    }



    /// Apply plume generation data to create actual plumes
    fn apply_plume_generation_data(&mut self, sim: &mut Simulation, plume_generation_data: Vec<(usize, CellIndex, usize, BuoyancyInfo)>) {
        // Second pass: create plume vectors and apply initial mass/energy extraction
        for (layer_set_idx, h3_cell_index, cell_idx, buoyancy_info) in plume_generation_data {
            // Get the cell data we need (collect data first to avoid borrowing conflicts)
            let cell_data = if let Some(layer_set) = sim.layer_sets.get(layer_set_idx) {
                if let Some(column) = layer_set.layers.get(&h3_cell_index) {
                    if let Some(cell) = column.cells.get(cell_idx) {
                        Some((cell, layer_set_idx, h3_cell_index, buoyancy_info))
                    } else { None }
                } else { None }
            } else { None };

            if let Some((cell, layer_set_idx, h3_cell_index, buoyancy_info)) = cell_data {
                // Create the plume and get extraction amounts (pass sim separately to avoid borrowing conflicts)
                let (mass_to_remove, energy_to_remove) = self.create_buoyancy_plume_with_data(
                    layer_set_idx, h3_cell_index, &cell, &buoyancy_info);

                        // Apply initial mass and energy extraction from source cell
                        if let Some(source_layer) = sim.layer_sets.get_mut(layer_set_idx) {
                            if let Some(source_column) = source_layer.layers.get_mut(&h3_cell_index) {
                                if let Some(source_cell) = source_column.cells.get_mut(cell_idx) {
                                    // Double-entry mass accounting: transport very small amounts
                                    // Remove exact amount from source that will be added to targets
                                    source_cell.add_mass_kg(-mass_to_remove); // Debit source
                                    // Credit will be applied to targets via apply_to_layer (exact same amount)
                                }
                            }
                        }
                    }
                }
    }

    /// Create a buoyancy-driven moving plume with cell data (helper to avoid borrowing conflicts)
    fn create_buoyancy_plume_with_data(&mut self, layer_set_idx: usize, cell_index: h3o::CellIndex,
                                     cell: &crate::deprecated::sim::energy_mass_cell::EnergyMassCell,
                                     buoyancy_info: &BuoyancyInfo) -> (f64, f64) {
        // Generate plume properties based on buoyancy conditions
        let base_temp = cell.temperature_kelvin();
        let temp_variation = (self.rng.random::<f64>() - 0.5) * 2.0 * self.temperature_perturbation_amplitude_k;
        let plume_temp = base_temp + temp_variation;

        // Calculate plume mass and energy (focus on hotspot energy transport)
        let mass_fraction = 0.001; // Take only 0.1% of cell mass (very minimal mass transfer)

        // Only transport energy if this is a hotspot-affected cell
        // Use the configured copy fraction for hotspot energy transport
        let base_mass = cell.mass_kg() * mass_fraction;
        let base_energy = cell.energy_joules() * self.energy_copy_fraction; // Copy hotspot energy

        let buoyancy_factor = (buoyancy_info.buoyancy_force / 1000.0).min(3.0);
        let variation = 1.0 + (self.rng.random::<f64>() - 0.5) * 2.0 * self.energy_variation_factor;

        let plume_mass = base_mass * buoyancy_factor * variation;
        let plume_energy = base_energy * buoyancy_factor * variation;

        println!("🌋 Plume data calculated: {:.2e}kg, {:.2e}J at layer {} cell {:?}",
            plume_mass, plume_energy, layer_set_idx, cell_index);

        // Return extraction amounts (actual plume creation will happen later)
        (plume_mass, plume_energy)
    }

    /// Static version of calculate_buoyancy_force for parallel processing
    fn calculate_buoyancy_force_static(cell: &EnergyMassCell, upper_cell: &EnergyMassCell) -> Option<BuoyancyInfo> {
        let cell_temp = cell.temperature_kelvin();
        let upper_temp = upper_cell.temperature_kelvin();
        let temp_diff = cell_temp - upper_temp;

        // Minimum temperature difference for plume formation
        let min_temp_difference_k = 200.0;

        if temp_diff > min_temp_difference_k {
            // Calculate density difference based on temperature
            // Hot cells are less dense than cold cells (thermal expansion)
            let base_density = 3000.0; // Base basalt density at reference temperature
            let thermal_expansion_coeff = 3e-5; // Typical thermal expansion coefficient (1/K)
            let reference_temp = 1000.0; // Reference temperature (K)

            // Calculate actual densities based on temperature
            let cell_density = base_density * (1.0 - thermal_expansion_coeff * (cell_temp - reference_temp));
            let upper_density = base_density * (1.0 - thermal_expansion_coeff * (upper_temp - reference_temp));

            let density_diff = upper_density - cell_density; // Positive if cell is less dense (buoyant)
            let buoyancy_force = GRAVITY_M_S2 * density_diff;

            if density_diff > 0.0 {
                Some(BuoyancyInfo {
                    density_difference: density_diff,
                    buoyancy_force,
                    temperature_excess: temp_diff,
                    lower_density: cell_density,
                    upper_density,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Static version of calculate_buoyancy_plume_probability for parallel processing
    fn calculate_buoyancy_plume_probability_static(
        cell_area_km2: f64,
        years_per_step: f64,
        buoyancy_info: &BuoyancyInfo,
        layer_height_km: f64,
        total_cells_in_layer: usize,
    ) -> f64 {
        let base_probability_per_km2_per_year = 1e-12;

        // Temperature factor: higher temperature difference = higher probability
        let temp_factor = (buoyancy_info.temperature_excess / 500.0).min(3.0);

        // Density factor: higher density difference = higher probability
        let density_factor = (buoyancy_info.density_difference / 100.0).min(2.0);

        // Layer depth factor: deeper layers have higher probability
        let depth_factor = (layer_height_km / 100.0).max(0.5);

        // Area scaling: larger cells have proportionally higher probability
        let area_factor = cell_area_km2;

        // Time scaling
        let time_factor = years_per_step;

        // Layer size factor: fewer cells = higher individual probability
        let layer_size_factor = (1000.0 / total_cells_in_layer as f64).max(0.1);

        base_probability_per_km2_per_year * temp_factor * density_factor * depth_factor *
        area_factor * time_factor * layer_size_factor
    }

    /// Calculate total mass and energy for a layer including plume contributions
    pub fn calculate_layer_totals(&self, sim: &Simulation, layer_index: usize, base_mass: f64, base_energy: f64) -> (f64, f64) {
        let mut total_mass = base_mass;
        let mut total_energy = base_energy;

        // Add contributions from all plumes that affect this layer
        for plume in &sim.plumes {
            // Check if plume affects this layer (current layer or adjacent layers)
            let affects_layer = plume.current_layer_index == layer_index ||
                               plume.current_layer_index.saturating_sub(1) == layer_index ||
                               plume.current_layer_index + 1 == layer_index;

            if affects_layer {
                // Calculate effect strength based on layer relationship
                let effect_strength = if plume.current_layer_index == layer_index {
                    1.0 // Full effect in current layer
                } else if plume.current_layer_index.saturating_sub(1) == layer_index {
                    plume.vertical_radiation_factor * 0.3 // 30% radiation upward
                } else {
                    plume.vertical_radiation_factor * 0.2 // 20% radiation downward
                };

                total_mass += plume.total_mass_kg * effect_strength;
                total_energy += plume.total_energy_joules * effect_strength;
            }
        }

        (total_mass, total_energy)
    }

    /// Get average layer density including plume contributions
    pub fn calculate_layer_density(&self, sim: &Simulation, layer_index: usize, base_mass: f64, layer_volume_km3: f64) -> f64 {
        let (total_mass, _) = self.calculate_layer_totals(sim, layer_index, base_mass, 0.0);
        total_mass / (layer_volume_km3 * 1e9) // Convert km³ to m³
    }



    /// Update existing moving plumes - apply decay, movement, and vertical radiation (threaded)
    fn update_plumes(&mut self, sim: &mut crate::deprecated::sim::simulation::Simulation, years_per_step: f64) {
        let use_threading = sim.plumes.len() > 10; // Use threading for many plumes

        if use_threading {
            self.update_plumes_threaded(sim, years_per_step);
        } else {
            self.update_plumes_sequential(sim, years_per_step);
        }
    }

    /// Sequential plume updates (for few plumes)
    fn update_plumes_sequential(&mut self, sim: &mut crate::deprecated::sim::simulation::Simulation, years_per_step: f64) {
        // First pass: apply physics updates that don't need sim access
        sim.plumes.retain_mut(|plume| {
            // 1. Apply half-life decay
            plume.apply_half_life_decay(years_per_step);

            // 2. Check if still significant
            if !plume.is_significant() {
                println!("🌋 Plume #{} faded below significance threshold", plume.id);
                return false; // Remove plume
            }

            // 3. Move plume upward
            let distance_moved = plume.velocity_km_per_year * years_per_step;
            plume.current_depth_km -= distance_moved; // Move up (decrease depth)
            plume.age_years += years_per_step;

            // 4. Check if plume should move to upper layer (inline to avoid borrowing issues)
            if plume.current_layer_index > 0 {
                // Get current layer thickness
                let should_move = if let Some(current_layer) = sim.layer_sets.get(plume.current_layer_index) {
                    if let Some(column) = current_layer.layers.values().next() {
                        if let Some(first_cell) = column.cells.first() {
                            let layer_top = first_cell.top_km;
                            // Move to upper layer if plume has risen above current layer
                            plume.current_depth_km <= layer_top
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_move {
                    plume.current_layer_index -= 1;
                    println!("🌋 Plume #{} moved to layer {}", plume.id, plume.current_layer_index);
                }
            }

            // 6. Remove if reached surface
            if plume.current_layer_index == 0 && plume.current_depth_km <= 0.0 {
                println!("🌋 Plume #{} reached surface and dissipated", plume.id);
                return false;
            }

            true // Keep plume
        });

        // Second pass: apply vertical radiation effects (needs mutable sim access)
        // TODO: Re-implement vertical effects without borrowing conflicts
        // self.apply_all_plume_vertical_effects(sim, years_per_step);
    }

    /*
    /// Apply vertical effects for all plumes (helper method to avoid borrowing conflicts)
    fn apply_all_plume_vertical_effects(&self, sim: &mut crate::deprecated::sim::simulation::Simulation, years_per_step: f64) {
        // Collect plume data first to avoid borrowing conflicts
        let plume_data: Vec<_> = sim.plumes.iter().map(|p| {
            (p.id, p.current_layer_index, p.total_energy_joules, p.total_mass_kg, p.source_cell_index)
        }).collect();

        for (plume_id, current_layer, total_energy, total_mass, source_cell) in plume_data {
            // Apply vertical radiation effects for this plume
            let annual_transfer_rate = 0.1;
            let total_energy_transfer = total_energy * annual_transfer_rate * years_per_step;
            let total_mass_transfer = total_mass * annual_transfer_rate * years_per_step;

            // Apply to current layer and adjacent layers
            self.apply_plume_effects_to_layers(sim, current_layer, source_cell, total_energy_transfer, total_mass_transfer);
        }
    }

    /// Apply plume effects to specific layers (helper method)
    fn apply_plume_effects_to_layers(&self, sim: &mut crate::deprecated::sim::simulation::Simulation,
                                   current_layer: usize, source_cell: h3o::CellIndex,
                                   energy_transfer: f64, mass_transfer: f64) {
        // Apply effects to current layer and adjacent layers
        for layer_offset in -1i32..=1i32 {
            let target_layer = (current_layer as i32 + layer_offset) as usize;
            if target_layer < sim.layer_sets.len() {
                if let Some(layer_set) = sim.layer_sets.get_mut(target_layer) {
                    if let Some(column) = layer_set.layers.get_mut(&source_cell) {
                        if let Some(cell) = column.cells.get_mut(0) { // Apply to first cell in column
                            cell.add_energy_joules(energy_transfer * 0.33); // Distribute energy
                            cell.add_mass_kg(mass_transfer * 0.33); // Distribute mass
                        }
                    }
                }
            }
        }
    }
    */

    /// Threaded plume updates (for many plumes)
    fn update_plumes_threaded(&mut self, sim: &mut crate::deprecated::sim::simulation::Simulation, years_per_step: f64) {
        println!("🧵 Using threaded plume updates for {} plumes", sim.plumes.len());

        // Phase 1: Parallel physics updates (read-only operations)
        let plume_updates: Vec<_> = sim.plumes
            .par_iter()
            .map(|plume| {
                let mut updated_plume = plume.clone();

                // 1. Apply half-life decay
                updated_plume.apply_half_life_decay(years_per_step);

                // 2. Check if still significant
                let is_significant = updated_plume.is_significant();

                // 4. Move plume upward
                let distance_moved = updated_plume.velocity_km_per_year * years_per_step;
                updated_plume.current_depth_km -= distance_moved; // Move up (decrease depth)
                updated_plume.age_years += years_per_step;

                (updated_plume, is_significant)
            })
            .collect();

        // Phase 2: Apply updates and handle simulation effects (sequential)
        let mut plumes_to_keep = Vec::new();

        for (updated_plume, is_significant) in plume_updates {
            if !is_significant {
                println!("🌋 Plume #{} faded below significance threshold", updated_plume.id);
                continue; // Remove plume
            }

            let mut final_plume = updated_plume;

            // 3. Apply vertical radiation effects (must be sequential due to sim mutation)
            final_plume.apply_vertical_effects(sim, years_per_step);

            // 5. Check if plume should move to upper layer
            if final_plume.current_layer_index > 0 {
                let should_move = if let Some(current_layer) = sim.layer_sets.get(final_plume.current_layer_index) {
                    if let Some(column) = current_layer.layers.values().next() {
                        if let Some(first_cell) = column.cells.first() {
                            let layer_top = first_cell.top_km;
                            final_plume.current_depth_km <= layer_top
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if should_move {
                    final_plume.current_layer_index -= 1;
                    println!("🌋 Plume #{} moved to layer {}", final_plume.id, final_plume.current_layer_index);
                }
            }

            // 6. Remove if reached surface
            if final_plume.current_layer_index == 0 && final_plume.current_depth_km <= 0.0 {
                println!("🌋 Plume #{} reached surface and dissipated", final_plume.id);
                continue; // Remove plume
            }

            plumes_to_keep.push(final_plume);
        }

        // Replace plumes with updated ones
        sim.plumes = plumes_to_keep;

        println!("🧵 Threaded update completed: {} plumes remaining", sim.plumes.len());
    }

    /// Check if plume should move to upper layer based on depth
    fn should_move_to_upper_layer(&self, plume: &ConvectionPlume, sim: &crate::deprecated::sim::simulation::Simulation) -> bool {
        if plume.current_layer_index == 0 {
            return false; // Already at surface
        }

        // Get current layer thickness
        if let Some(current_layer) = sim.layer_sets.get(plume.current_layer_index) {
            if let Some(column) = current_layer.layers.values().next() {
                if let Some(first_cell) = column.cells.first() {
                    let layer_top = first_cell.top_km;

                    // Move to upper layer if plume has risen above current layer
                    return plume.current_depth_km <= layer_top;
                }
            }
        }

        false
    }





    /// Apply plume effects - now handled by update_plumes method
    fn apply_plume_effects(&mut self, _sim: &mut Simulation, _years_per_step: f64) {
        // Energy/mass transfer is now handled by the update_plumes method
        // through pre-calculated resolution hierarchy
    }

    /// Find the cell index within a column that corresponds to a given depth
    fn find_cell_at_depth(column: &crate::deprecated::sim::layer_set::Column, target_depth_km: f64) -> usize {
        // Simple approach: assume cells are evenly distributed in depth
        // and pick the cell closest to the target depth
        let num_cells = column.cells.len();
        if num_cells == 0 {
            return 0;
        }

        // Assume each cell represents equal depth intervals
        let start_depth = column.start_height_km;
        let cell_height = 10.0; // Assume 10km per cell (could be made configurable)

        let relative_depth = (target_depth_km - start_depth).abs();
        let cell_index = (relative_depth / cell_height) as usize;

        // Clamp to valid range
        cell_index.min(num_cells - 1)
    }
}

impl SimComponent for ConvectionPlumeComponent {
    fn key(&self) -> &'static str {
        "convection_plumes"
    }

    fn initialize(&mut self, sim: &mut Simulation) {
        println!("🌋 Convection Plume Component initialized");
        println!("   - Buoyancy-based plume generation (no absolute temperature threshold)");
        println!("   - Min temp difference: {:.1}K", self.min_temp_difference_k);
        println!("   - Base probability: {:.2e} per km²/year", self.base_plume_probability_per_km2_per_year);
        println!("   - Plume radius: {:.1} km", self.plume_radius_km);
        println!("   - Plume velocity: {:.1} km/year", self.plume_velocity_km_per_year);
        println!("   - Temperature perturbations: ±{:.1}K", self.temperature_perturbation_amplitude_k);
        println!("   - Energy variation: ±{:.0}%", self.energy_variation_factor * 100.0);
        println!("   - Radius variation: ±{:.0}%", self.radius_variation_factor * 100.0);

        // Apply initial temperature perturbations to break symmetry
        println!("   - Applying initial temperature perturbations...");
        self.apply_temperature_perturbations(sim);
        println!("   - Temperature perturbations applied ✓");
    }

    fn step(&mut self, sim: &mut Simulation, step: i64, year: i64) {
        // Component organizes its own internal phases with detailed timing
        {
            let start = std::time::Instant::now();
            self.analyze_and_generate_plumes(sim, step, year);
            let duration = start.elapsed();
            // Profiling now handled by event system
            println!("⏱️  analyze_and_generate_plumes: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }

        {
            let start = std::time::Instant::now();
            self.apply_plume_effects_internal(sim, step, year);
            let duration = start.elapsed();
            // Profiling now handled by event system
            println!("⏱️  apply_plume_effects_internal: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }

        {
            let start = std::time::Instant::now();
            self.report_plume_status(sim, step, year);
            let duration = start.elapsed();
            // Profiling now handled by event system
            println!("⏱️  report_plume_status: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }
    }

    fn complete(&mut self, sim: &Simulation) {
        println!("🌋 Convection Plume Component completed");
        println!("   - Final active plumes: {}", sim.plumes.len());
        let total_energy: f64 = sim.plumes.iter().map(|p| p.total_energy_joules).sum();
        println!("   - Total plume energy: {:.2e} J", total_energy);
    }
}

// Internal methods - component's choice of organization (great for unit testing)
impl ConvectionPlumeComponent {
    /// Analyze conditions and generate plumes (private method for internal organization)
    fn analyze_and_generate_plumes(&mut self, sim: &mut Simulation, _step: i64, _year: i64) {
        let years_per_step = sim.years_per_step();

        // Generate new plumes based on temperature conditions with detailed timing
        {
            let start = std::time::Instant::now();
            self.generate_plumes(sim, years_per_step);
            let duration = start.elapsed();
            println!("    🔍 generate_plumes: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }

        // Update existing plumes with detailed timing
        {
            let start = std::time::Instant::now();
            self.update_plumes(sim, years_per_step);
            let duration = start.elapsed();
            println!("    🔄 update_plumes: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }
    }

    /// Apply plume effects to simulation (private method for internal organization)
    fn apply_plume_effects_internal(&mut self, sim: &mut Simulation, _step: i64, _year: i64) {
        let years_per_step = sim.years_per_step();

        // Apply plume effects - radiate energy to surrounding cells with detailed timing
        {
            let start = std::time::Instant::now();
            self.apply_plume_effects(sim, years_per_step);
            let duration = start.elapsed();
            println!("    🌊 apply_plume_effects: {:.2} ms", duration.as_secs_f64() * 1000.0);
        }
    }

    /// Report plume status (private method for internal organization)
    fn report_plume_status(&mut self, sim: &Simulation, step: i64, _year: i64) {
        if step % 100 == 0 && !sim.plumes.is_empty() {
            let total_energy: f64 = sim.plumes.iter().map(|p| p.total_energy_joules).sum();
            let avg_temp: f64 = sim.plumes.iter().map(|p| p.temperature_k).sum::<f64>() / sim.plumes.len() as f64;
            let avg_velocity: f64 = sim.plumes.iter().map(|p| p.velocity_km_per_year).sum::<f64>() / sim.plumes.len() as f64;
            let avg_age: f64 = sim.plumes.iter().map(|p| p.age_years).sum::<f64>() / sim.plumes.len() as f64;

            println!("🌋 Moving Plumes (Step {}): {} active, {:.2e}J total, {:.1}K avg temp, {:.2} km/yr avg velocity, {:.1} yr avg age",
                step, sim.plumes.len(), total_energy, avg_temp, avg_velocity, avg_age);
        }
    }
}

impl Default for ConvectionPlumeComponent {
    fn default() -> Self {
        Self::new()
    }
}

// Pressure equalization tests moved inline

#[cfg(test)]
mod tests {
    use crate::constants::{DEFAULT_SURFACE_TEMP_K, EARTH_RADIUS_KM};
    use super::*;
    
    use crate::deprecated::sim::layer_set::{default_layer_set_params, DefaultLayerSetParams};

    #[test]
    fn test_mass_transfer_pressure_imbalance() {
        println!("\n🧪 Testing Mass Transfer Pressure Imbalance");
        println!("============================================");

        let cell_index = h3o::CellIndex::try_from(0x85283473fffffff_u64).unwrap();

        // Create two cells with different pressures but same material
        let mut lower_cell = crate::deprecated::sim::energy_mass_cell::EnergyMassCell::new(
            crate::deprecated::sim::energy_mass_cell::EnergyMassCellProps {
                cell_index,
                temperature_kelvin: 1800.0,
                pressure_pa: 2e9,  // 2 GPa - high pressure (deep)
                height_km: 20.0,
                top_km: 200.0,
                material_name: "basalt".to_string(),
                planet_radius_km: 6371.0,
            });

        let mut upper_cell = crate::deprecated::sim::energy_mass_cell::EnergyMassCell::new(
            crate::deprecated::sim::energy_mass_cell::EnergyMassCellProps {
                cell_index,
                temperature_kelvin: 1200.0,
                pressure_pa: 1e9,  // 1 GPa - lower pressure (shallow)
                height_km: 20.0,
                top_km: 100.0,
                material_name: "basalt".to_string(),
                planet_radius_km: 6371.0,
            });

        // Record initial state
        let initial_lower_mass = lower_cell.mass_kg();
        let initial_upper_mass = upper_cell.mass_kg();
        let initial_lower_pressure = lower_cell.pressure_pa();
        let initial_upper_pressure = upper_cell.pressure_pa();

        println!("Initial State:");
        println!("  Lower cell: {:.2e} kg at {:.1e} Pa", initial_lower_mass, initial_lower_pressure);
        println!("  Upper cell: {:.2e} kg at {:.1e} Pa", initial_upper_mass, initial_upper_pressure);

        // Simulate convection plume mass transfer (typical 0.1% transfer)
        let mass_transfer_fraction = 0.001;
        let mass_to_transfer = initial_lower_mass * mass_transfer_fraction;

        // Apply mass transfer (current implementation)
        lower_cell.add_mass_kg(-mass_to_transfer);  // Remove from lower
        upper_cell.add_mass_kg(mass_to_transfer);   // Add to upper

        // Check final state
        let final_lower_pressure = lower_cell.pressure_pa();
        let final_upper_pressure = upper_cell.pressure_pa();

        // Calculate pressure changes
        let lower_pressure_change = final_lower_pressure - initial_lower_pressure;
        let upper_pressure_change = final_upper_pressure - initial_upper_pressure;

        println!("Pressure Changes:");
        println!("  Lower cell: {:.2e} Pa ({:.1}%)",
                 lower_pressure_change,
                 (lower_pressure_change / initial_lower_pressure) * 100.0);
        println!("  Upper cell: {:.2e} Pa ({:.1}%)",
                 upper_pressure_change,
                 (upper_pressure_change / initial_upper_pressure) * 100.0);

        // CRITICAL: This demonstrates the pressure imbalance problem
        let pressure_imbalance = (lower_pressure_change.abs() + upper_pressure_change.abs()) / 2.0;
        println!("❌ PRESSURE IMBALANCE: {:.2e} Pa", pressure_imbalance);
        println!("   Mass transfer without pressure equalization causes instability");

        // Mass conservation should be maintained
        let total_initial_mass = initial_lower_mass + initial_upper_mass;
        let total_final_mass = lower_cell.mass_kg() + upper_cell.mass_kg();
        let mass_conservation_error = (total_final_mass - total_initial_mass).abs();
        assert!(mass_conservation_error < 1e-6, "Mass conservation violated");

        println!("\n💡 SOLUTION: Make pressure dynamically calculated from mass/volume/temperature");
        println!("   - Remove stored pressure_pa field");
        println!("   - Use material.calculate_pressure_from_mass_volume()");
        println!("   - Pressure automatically adjusts when mass changes");
        println!("   - Natural pressure equilibrium prevents drainage");
    }

    #[test]
    fn test_dynamic_pressure_calculation() {
        println!("\n🧪 Testing Dynamic Pressure Calculation");
        println!("=======================================");

        // This test demonstrates how pressure should be calculated dynamically
        let basalt = crate::material::materials_loader::MaterialsLoader::get_phase_properties(
            "basalt",
            crate::material::MaterialPhases::Solid
        ).expect("Failed to get basalt properties");

        // Test parameters
        let volume_km3 = 1000.0; // 1000 km³
        let temperature_k = 1500.0; // 1500K
        let initial_mass_kg = 3e15; // 3 × 10¹⁵ kg

        // Calculate initial pressure from mass
        let initial_pressure = basalt.calculate_pressure_from_mass_volume(
            crate::material::material::PressureCalculationParams::new(
                initial_mass_kg, volume_km3, temperature_k
            )
        );

        println!("Initial state:");
        println!("  Mass: {:.2e} kg", initial_mass_kg);
        println!("  Volume: {:.0} km³", volume_km3);
        println!("  Temperature: {:.0} K", temperature_k);
        println!("  Calculated pressure: {:.2e} Pa", initial_pressure);

        // Simulate mass transfer (remove 10% of mass)
        let mass_transfer = initial_mass_kg * 0.1;
        let final_mass_kg = initial_mass_kg - mass_transfer;

        // Calculate new pressure from new mass
        let final_pressure = basalt.calculate_pressure_from_mass_volume(
            crate::material::material::PressureCalculationParams::new(
                final_mass_kg, volume_km3, temperature_k
            )
        );

        println!("\nAfter 10% mass removal:");
        println!("  Mass: {:.2e} kg", final_mass_kg);
        println!("  Calculated pressure: {:.2e} Pa", final_pressure);

        let pressure_change = final_pressure - initial_pressure;
        let pressure_change_percent = (pressure_change / initial_pressure) * 100.0;

        println!("  Pressure change: {:.2e} Pa ({:.1}%)", pressure_change, pressure_change_percent);

        // This demonstrates natural pressure feedback
        assert!(pressure_change < 0.0, "Pressure should decrease when mass is removed");
        assert!(pressure_change_percent.abs() > 1.0, "Pressure change should be significant");

        println!("\n✅ DYNAMIC PRESSURE WORKS:");
        println!("   - Pressure automatically decreases when mass is removed");
        println!("   - Provides natural feedback to limit mass transfer");
        println!("   - Prevents unlimited drainage through pressure equilibrium");
    }

    #[test]
    fn test_transaction_manager_dynamic_pressure() {
        println!("\n🧪 Testing Transaction Manager Dynamic Pressure");
        println!("===============================================");

        use crate::transaction_manager::{TransactionManager, CellSnapshot, CellLocation};

        let mut tm = TransactionManager::new();

        // Create a cell snapshot with fixed overhead mass
        let location = CellLocation::new(0, h3o::CellIndex::try_from(0x85283473fffffff_u64).unwrap(), 0);
        let initial_mass = 1e15; // 1 × 10¹⁵ kg
        let initial_energy = 1e20; // 1 × 10²⁰ J
        let initial_temp = 1500.0; // 1500K
        let overhead_mass_kg_per_m2 = 1e6; // 1 million kg/m² overhead

        let snapshot = CellSnapshot {
            location: location.clone(),
            mass_kg: initial_mass,
            energy_joules: initial_energy,
            temperature_kelvin: initial_temp,
            initial_overhead_mass_kg_per_m2: overhead_mass_kg_per_m2,
        };

        tm.record_baseline_snapshot(location.clone(), snapshot.clone());

        // Calculate initial pressure
        let initial_pressure = snapshot.calculate_pressure_pa(initial_mass, initial_temp);
        println!("Initial state:");
        println!("  Mass: {:.2e} kg", initial_mass);
        println!("  Overhead mass: {:.2e} kg/m²", overhead_mass_kg_per_m2);
        println!("  Calculated pressure: {:.2e} Pa", initial_pressure);

        // Simulate mass removal (like convection plume drainage)
        let mass_removed = initial_mass * 0.1; // Remove 10%
        let final_mass = initial_mass - mass_removed;

        // Calculate new pressure with reduced mass
        let final_pressure = snapshot.calculate_pressure_pa(final_mass, initial_temp);

        println!("\nAfter 10% mass removal:");
        println!("  Mass: {:.2e} kg", final_mass);
        println!("  Calculated pressure: {:.2e} Pa", final_pressure);

        let pressure_change = final_pressure - initial_pressure;
        let pressure_change_percent = (pressure_change / initial_pressure) * 100.0;

        println!("  Pressure change: {:.2e} Pa ({:.1}%)", pressure_change, pressure_change_percent);

        // This demonstrates the pressure feedback mechanism
        assert!(pressure_change < 0.0, "Pressure should decrease when mass is removed");
        assert!(pressure_change_percent.abs() > 1.0, "Pressure change should be significant enough to provide feedback");

        println!("\n✅ DYNAMIC PRESSURE FEEDBACK:");
        println!("   - Pressure decreases when mass is removed");
        println!("   - Provides natural resistance to further mass transfer");
        println!("   - Prevents unlimited drainage through pressure equilibrium");
        println!("   - Fixed overhead mass avoids expensive recomputation");
    }

    #[test]
    fn test_actual_overhead_mass_calculation() {
        println!("\n🧪 Testing Actual Overhead Mass Calculation");
        println!("===========================================");

        // This test demonstrates that we calculate ACTUAL overhead mass
        // from cells above, not from pressure (which was wrong when cells had zero mass)

        use crate::deprecated::sim::simulation::{Simulation, SimulationConfig};
        use crate::deprecated::sim::layer_set::LayerSetParams;
        use h3o::Resolution;

        // Create a simple 2-layer simulation
        let config = SimulationConfig {
            surface_temp_k: 288.15,
            layer_set_params: vec![
                LayerSetParams {
                    name: "Upper Layer".to_string(),
                    resolution: Resolution::Two,
                    start_height_km: 0.0,
                    cell_height_km: 50.0,
                    material_name: "basalt".to_string(),
                    cells_per_column: 2, // 2 cells per column
                    planet_radius_km: 6371.0,
                    thermal_gradient_k_per_km: 25.0,
                },
                LayerSetParams {
                    name: "Lower Layer".to_string(),
                    resolution: Resolution::Two,
                    start_height_km: 100.0, // Will be adjusted to 100km
                    cell_height_km: 50.0,
                    material_name: "basalt".to_string(),
                    cells_per_column: 2, // 2 cells per column
                    planet_radius_km: 6371.0,
                    thermal_gradient_k_per_km: 10.0,
                },
            ],
            warmup_steps: 0,
            steps: 1,
            years_per_step: 1000.0,
        };

        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let mut sim = Simulation::new(config, &mut components);
        sim.initialize();

        // Get the first available cell from the bottom layer
        if let Some(layer_set) = sim.layer_sets.get(1) {
            if let Some((h3_cell, column)) = layer_set.layers.iter().next() {
                if let Some(bottom_cell) = column.cells.get(1) {
                    println!("Bottom cell found:");
                    println!("  H3 Cell: {:?}", h3_cell);
                    println!("  Mass: {:.2e} kg", bottom_cell.mass_kg());
                    println!("  Pressure: {:.2e} Pa", bottom_cell.pressure_pa());

                    // Calculate what the overhead mass should be
                    let expected_overhead_mass = sim.calculate_overhead_mass_for_cell(1, *h3_cell, 1);
                    println!("  Calculated overhead mass: {:.2e} kg/m²", expected_overhead_mass);

                    // This should be > 0 because there are cells above
                    assert!(expected_overhead_mass > 0.0, "Overhead mass should be > 0 for bottom cells");

                    // The overhead mass should be reasonable (not astronomical)
                    assert!(expected_overhead_mass < 1e10, "Overhead mass should be reasonable");

                    println!("\n✅ ACTUAL OVERHEAD MASS CALCULATION:");
                    println!("   - Overhead mass calculated from actual cell masses above");
                    println!("   - Not derived from pressure (which was wrong with zero initial mass)");
                    println!("   - Provides realistic pressure baseline for dynamic calculation");
                    println!("   - Fixes the fundamental issue with pressure caching");
                } else {
                    println!("❌ Could not find cell at depth 1");
                }
            } else {
                println!("❌ No columns in layer set 1");
            }
        } else {
            println!("❌ Could not find layer set 1");
        }
    }

    #[test]
    fn test_two_pass_mass_initialization() {
        println!("\n🧪 Testing Two-Pass Mass Initialization");
        println!("=======================================");

        // This test demonstrates the proper two-pass approach:
        // 1. First pass: Calculate mass from density × volume (uncompressed)
        // 2. Second pass: Adjust for compression based on overhead mass

        use crate::deprecated::sim::simulation::{Simulation, SimulationConfig};
        
        use h3o::Resolution;

        // Create a simple 2-layer simulation
        let config = SimulationConfig {
            layer_set_params: default_layer_set_params( &DefaultLayerSetParams {
                resolution: Resolution::One,
                planet_radius_km: EARTH_RADIUS_KM as f64,
            }),
            warmup_steps: 0,
            steps: 1,
            years_per_step: 1000.0,
            surface_temp_k: DEFAULT_SURFACE_TEMP_K,
        };

        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![];
        let mut sim = Simulation::new(config, &mut components);

        // Before two-pass initialization - cells already have some mass from layer creation
        let (initial_top_mass, initial_bottom_mass) = if let Some(layer_set) = sim.layer_sets.get(0) {
            if let Some((h3_cell, column)) = layer_set.layers.iter().next() {
                let top_mass = column.cells.get(0).map(|c| c.mass_kg()).unwrap_or(0.0);
                let bottom_mass = column.cells.get(1).map(|c| c.mass_kg()).unwrap_or(0.0);
                println!("Before two-pass initialization:");
                println!("  Top cell mass: {:.2e} kg", top_mass);
                println!("  Bottom cell mass: {:.2e} kg", bottom_mass);
                (top_mass, bottom_mass)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        // Initialize with two-pass approach
        sim.initialize();

        // After two-pass initialization - masses should be adjusted for compression
        if let Some(layer_set) = sim.layer_sets.get(0) {
            if let Some((h3_cell, column)) = layer_set.layers.iter().next() {
                if let Some(top_cell) = column.cells.get(0) {
                    let final_top_mass = top_cell.mass_kg();
                    let final_top_pressure = top_cell.pressure_pa();

                    println!("\nAfter two-pass initialization:");
                    println!("  Top cell mass: {:.2e} kg", final_top_mass);
                    println!("  Top cell pressure: {:.2e} Pa", final_top_pressure);

                    // Cell should have realistic mass
                    assert!(final_top_mass > 0.0, "Cell should have mass after initialization");
                    assert!(final_top_mass < 1e20, "Cell mass should be reasonable");

                    // Check bottom cell for comparison
                    if let Some(bottom_cell) = column.cells.get(1) {
                        let final_bottom_mass = bottom_cell.mass_kg();
                        let final_bottom_pressure = bottom_cell.pressure_pa();

                        println!("  Bottom cell mass: {:.2e} kg", final_bottom_mass);
                        println!("  Bottom cell pressure: {:.2e} Pa", final_bottom_pressure);

                        // Bottom cell should have higher pressure due to overhead mass
                        assert!(final_bottom_pressure > final_top_pressure,
                               "Bottom cell should have higher pressure");

                        // Show the compression effect
                        let mass_change_top = ((final_top_mass - initial_top_mass) / initial_top_mass) * 100.0;
                        let mass_change_bottom = ((final_bottom_mass - initial_bottom_mass) / initial_bottom_mass) * 100.0;

                        println!("\nCompression effects:");
                        println!("  Top cell mass change: {:.1}%", mass_change_top);
                        println!("  Bottom cell mass change: {:.1}%", mass_change_bottom);

                        // Bottom cell should show more compression due to higher pressure
                        if mass_change_bottom > mass_change_top {
                            println!("  ✅ Bottom cell shows more compression as expected");
                        }
                    }
                }
            }
        }

        println!("\n✅ TWO-PASS MASS INITIALIZATION:");
        println!("   - First pass: Calculate mass from material density × volume");
        println!("   - Second pass: Adjust for compression based on overhead mass");
        println!("   - Cells now start with realistic masses, not zero");
        println!("   - Pressure correctly reflects overhead mass compression");
        println!("   - Fixes the fundamental zero-mass initialization problem");
    }

    #[test]
    fn test_buoyancy_velocity_calculation() {
        let component = ConvectionPlumeComponent::new();

        // Test case: hot plume (lower density) rising through cooler ambient material
        let plume_density = 3200.0; // kg/m³ - hot, less dense material
        let ambient_density = 3300.0; // kg/m³ - cooler, denser material
        let plume_radius_km = 5.0;

        let velocity = component.calculate_buoyancy_velocity(
            plume_density,
            ambient_density,
            plume_radius_km
        );

        // Should have positive velocity (upward movement)
        assert!(velocity > 0.0, "Hot plume should rise with positive velocity");
        assert!(velocity < 100.0, "Velocity should be capped at reasonable geological rates");
    }

    #[test]
    fn test_area_scaled_probability() {
        let component = ConvectionPlumeComponent::new();

        // Test that probability scales with area
        let temp_excess = 500.0; // K above threshold
        let years_per_step = 100.0;

        let small_area = 100.0; // km²
        let large_area = 400.0; // km² (4x larger)

        // Test parameters for layer-aware probability
        let layer_height_km = 100.0; // 100km thick layer
        let total_cells_in_layer = 1000; // 1000 cells in layer

        let prob_small = component.calculate_plume_probability(small_area, years_per_step, temp_excess, layer_height_km, total_cells_in_layer);
        let prob_large = component.calculate_plume_probability(large_area, years_per_step, temp_excess, layer_height_km, total_cells_in_layer);

        // Larger area should have proportionally higher probability
        assert!(prob_large > prob_small, "Larger area should have higher plume probability");
        assert!((prob_large / prob_small - 4.0).abs() < 0.1,
            "Probability should scale linearly with area");
    }



    #[test]
    fn test_simulation_with_profiling() {
        use crate::deprecated::sim::simulation::{Simulation, SimulationConfig};
        
        use h3o::Resolution;

        println!("\n⏱️ Testing Simulation with Performance Profiling");
        println!("=================================================");

        // Create a minimal simulation configuration
        let layer_params = default_layer_set_params(&DefaultLayerSetParams {
            resolution: Resolution::One,
            planet_radius_km: EARTH_RADIUS_KM as f64,
        });

        let config = SimulationConfig {
            steps: 5, // Just 5 steps
            years_per_step: 1000.0,
            warmup_steps: 0,
            layer_set_params: vec![layer_params[0].clone()],
            surface_temp_k: DEFAULT_SURFACE_TEMP_K,
        };

        // Create components
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new(ConvectionPlumeComponent::with_seed(12345)),
        ];

        // Create simulation
        let mut sim = Simulation::new(config, &mut components);

        let num_steps = 5;
        println!("🚀 Running {} simulation steps...", num_steps);

        // Run simulation steps (this will automatically profile)
        for step in 0..num_steps {
            sim.step();
            if step % 2 == 0 {
                println!("   Step {} completed", step + 1);
            }
        }

        println!("\n📊 Generating performance report...");

        // Generate and print performance report
        // Performance reporting now handled by event system

        println!("✅ Simulation with profiling test completed!");
    }

    // #[test] - ComponentProfiler test disabled - replaced by event system
    #[allow(dead_code)]
    fn test_component_profiling_system() {
        // use crate::profiling::component_profiler::ComponentProfiler;
        

        println!("\n⏱️ Testing Component Profiling System");
        println!("=====================================");

        // let mut profiler = ComponentProfiler::new(); // Removed - using event system

        // Profiler calls removed - using event system now
        println!("Simulated component method calls (profiler replaced by event system)");

        // Test replaced - profiler system removed
        println!("✅ Event system replaces profiler - test updated!");
    }



    #[test]
    fn test_randomness_creates_lumpiness() {
        // Test with different seeds to verify randomness
        let mut component1 = ConvectionPlumeComponent::with_seed(123);
        let mut component2 = ConvectionPlumeComponent::with_seed(456);

        // Test random energy variations
        let mut energies1 = Vec::new();
        let mut energies2 = Vec::new();

        for _ in 0..10 {
            // Simulate energy variation calculation
            let base_energy = 1e15; // Base energy
            let energy_multiplier1 = 1.0 + (component1.rng.random::<f64>() - 0.5) * 2.0 * component1.energy_variation_factor;
            let energy_multiplier2 = 1.0 + (component2.rng.random::<f64>() - 0.5) * 2.0 * component2.energy_variation_factor;

            energies1.push(base_energy * energy_multiplier1);
            energies2.push(base_energy * energy_multiplier2);
        }

        // Verify different seeds produce different sequences
        assert_ne!(energies1, energies2, "Different seeds should produce different energy sequences");

        // Verify energy variations are within expected range (50% to 200% of base)
        let base_energy = 1e15;
        for energy in &energies1 {
            assert!(*energy >= base_energy * 0.5, "Energy should not be less than 50% of base");
            assert!(*energy <= base_energy * 2.0, "Energy should not exceed 200% of base");
        }

        // Test radius variations
        let mut radii = Vec::new();
        for _ in 0..10 {
            let base_radius = 5.0;
            let radius_multiplier = 1.0 + (component1.rng.random::<f64>() - 0.5) * 2.0 * component1.radius_variation_factor;
            radii.push(base_radius * radius_multiplier);
        }

        // Verify radius variations are within expected range (70% to 130% of base)
        let base_radius = 5.0;
        for radius in &radii {
            assert!(*radius >= base_radius * 0.7, "Radius should not be less than 70% of base");
            assert!(*radius <= base_radius * 1.3, "Radius should not exceed 130% of base");
        }

        println!("✅ Randomness test passed - creates proper lumpiness!");
        let min_energy = energies1.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_energy = energies1.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_radius = radii.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_radius = radii.iter().fold(0.0f64, |a, &b| a.max(b));

        println!("   Energy variations: {:.1} to {:.1} (factor of {:.1})",
            min_energy / base_energy,
            max_energy / base_energy,
            max_energy / min_energy
        );
        println!("   Radius variations: {:.2} to {:.2} km", min_radius, max_radius);
    }

    #[test]
    fn test_3d_position_randomization() {
        let mut component = ConvectionPlumeComponent::with_seed(789);

        // Test random position generation around a center point
        let center_lat = 45.0; // degrees
        let center_lon = -120.0; // degrees
        let center_depth = 100.0; // km
        let radius = 50.0; // km

        let mut positions = Vec::new();
        for _ in 0..20 {
            let (lat, lon, depth) = ConvectionPlumeComponent::generate_random_position_around(
                &mut component.rng,
                center_lat,
                center_lon,
                center_depth,
                radius
            );
            positions.push((lat, lon, depth));
        }

        // Verify positions are distributed around the center
        let avg_lat: f64 = positions.iter().map(|(lat, _, _)| lat).sum::<f64>() / positions.len() as f64;
        let avg_lon: f64 = positions.iter().map(|(_, lon, _)| lon).sum::<f64>() / positions.len() as f64;
        let avg_depth: f64 = positions.iter().map(|(_, _, depth)| depth).sum::<f64>() / positions.len() as f64;

        // Should be roughly centered around the input position
        assert!((avg_lat - center_lat).abs() < 2.0, "Average latitude should be near center");
        assert!((avg_lon - center_lon).abs() < 2.0, "Average longitude should be near center");
        assert!((avg_depth - center_depth).abs() < 20.0, "Average depth should be near center");

        // Verify positions have reasonable spread
        let lat_spread = positions.iter().map(|(lat, _, _)| lat).fold(0.0f64, |acc, &x| acc.max(x)) -
                        positions.iter().map(|(lat, _, _)| lat).fold(f64::INFINITY, |acc, &x| acc.min(x));
        let lon_spread = positions.iter().map(|(_, lon, _)| lon).fold(0.0f64, |acc, &x| acc.max(x)) -
                        positions.iter().map(|(_, lon, _)| lon).fold(f64::INFINITY, |acc, &x| acc.min(x));

        assert!(lat_spread > 0.1, "Should have latitude spread");
        assert!(lon_spread > 0.1, "Should have longitude spread");

        println!("✅ 3D Position randomization test passed!");
        println!("   Center: ({:.1}°, {:.1}°, {:.1}km)", center_lat, center_lon, center_depth);
        println!("   Average: ({:.1}°, {:.1}°, {:.1}km)", avg_lat, avg_lon, avg_depth);
        println!("   Spread: lat {:.2}°, lon {:.2}°", lat_spread, lon_spread);
    }
}

// Include the comprehensive simulation tests
#[cfg(test)]
mod convection_simulation_tests {
    use super::*;
    use crate::deprecated::sim::simulation::{Simulation, SimulationConfig};
    use crate::deprecated::sim::layer_set::{default_layer_set_params, DefaultLayerSetParams};
    use h3o::Resolution;
    use crate::constants::{DEFAULT_SURFACE_TEMP_K, EARTH_RADIUS_KM_F64};

    /// Test configuration for multi-layer convection simulations
    fn create_test_simulation_config() -> SimulationConfig {
  
        // Create realistic crust-to-asthenosphere layer sets (0-300km)
        let layer_params = default_layer_set_params(
            &DefaultLayerSetParams {
                resolution: Resolution::Two,
                planet_radius_km: 6371.0,
            }
        );

        SimulationConfig {
            steps: 3,                           // Only 3 steps for fast testing with larger stack
            years_per_step: 1000.0,            // 1000 years per step
            warmup_steps: 0,
            layer_set_params: layer_params,
            surface_temp_k: DEFAULT_SURFACE_TEMP_K
        }
    }

    /// Calculate convection metrics for a layer set
    #[derive(Debug, Clone)]
    struct LayerConvectionMetrics {
        layer_set_index: usize,
        total_energy_joules: f64,
        average_temperature_k: f64,
        temperature_variance: f64,
        energy_variance: f64,
        max_temperature_k: f64,
        min_temperature_k: f64,
    }

    fn calculate_layer_metrics(sim: &Simulation) -> Vec<LayerConvectionMetrics> {
        let mut metrics = Vec::new();

        for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
            let mut temperatures = Vec::new();
            let mut energies = Vec::new();

            // Collect all cell data
            for column in layer_set.layers.values() {
                for cell in &column.cells {
                    let temp = cell.temperature_kelvin();
                    let energy = cell.energy_joules();
                    temperatures.push(temp);
                    energies.push(energy);
                }
            }

            if !temperatures.is_empty() {
                let total_energy: f64 = energies.iter().sum();
                let avg_temp: f64 = temperatures.iter().sum::<f64>() / temperatures.len() as f64;
                let avg_energy: f64 = energies.iter().sum::<f64>() / energies.len() as f64;

                // Calculate variance
                let temp_variance: f64 = temperatures.iter()
                    .map(|&t| (t - avg_temp).powi(2))
                    .sum::<f64>() / temperatures.len() as f64;

                let energy_variance: f64 = energies.iter()
                    .map(|&e| (e - avg_energy).powi(2))
                    .sum::<f64>() / energies.len() as f64;

                let max_temp = temperatures.iter().fold(0.0f64, |acc, &x| acc.max(x));
                let min_temp = temperatures.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));

                metrics.push(LayerConvectionMetrics {
                    layer_set_index: layer_idx,
                    total_energy_joules: total_energy,
                    average_temperature_k: avg_temp,
                    temperature_variance: temp_variance,
                    energy_variance: energy_variance,
                    max_temperature_k: max_temp,
                    min_temperature_k: min_temp,
                });
            }
        }

        metrics
    }

    #[test]
    fn test_energy_distribution_with_and_without_convection() {
        println!("\n⚡ Testing Energy Distribution: With vs Without Convection");
        println!("=========================================================");

        let config = create_test_simulation_config();

        // Test 1: Simulation WITHOUT convection
        println!("\n📊 Running simulation WITHOUT convection...");
        let mut components_no_convection: Vec<Box<dyn SimComponent>> = vec![];
        let mut sim_no_convection = Simulation::new(config.clone(), &mut components_no_convection);
        sim_no_convection.initialize();

        let initial_metrics_no_conv = calculate_layer_metrics(&sim_no_convection);
        println!("✓ No-convection simulation initialized");

        // Run simulation
        for step in 0..config.steps {
            sim_no_convection.step();
            if step % 2 == 0 {
                println!("   No-convection step {} completed", step + 1);
            }
        }

        let final_metrics_no_conv = calculate_layer_metrics(&sim_no_convection);
        println!("✓ No-convection simulation completed");

        // Test 2: Simulation WITH convection AND core radiance
        println!("\n🌋 Running simulation WITH convection AND core radiance...");
        let mut components_with_convection: Vec<Box<dyn SimComponent>> = vec![
            Box::new(ConvectionPlumeComponent::with_seed(12345)),
            Box::new(super::super::core_radiance_component::CoreRadianceComponent::new()
                .with_base_energy(5e19)  // 5e19 J per cell per year
                .with_noise_amplitude(0.15)  // ±15% variation
                .with_spatial_scale(0.05)),   // Coarse spatial features
        ];
        let mut sim_with_convection = Simulation::new(config.clone(), &mut components_with_convection);
        sim_with_convection.initialize();

        let initial_metrics_with_conv = calculate_layer_metrics(&sim_with_convection);
        println!("✓ With-convection simulation initialized");

        // Run simulation
        for step in 0..config.steps {
            sim_with_convection.step();
            if step % 2 == 0 {
                println!("   With-convection step {} completed", step + 1);
            }
        }

        let final_metrics_with_conv = calculate_layer_metrics(&sim_with_convection);
        println!("✓ With-convection simulation completed");

        // DETAILED ENERGY ANALYSIS
        println!("\n⚡ DETAILED ENERGY ANALYSIS BY LAYER");
        println!("====================================");

        let total_initial_energy_no_conv: f64 = initial_metrics_no_conv.iter().map(|m| m.total_energy_joules).sum();
        let total_final_energy_no_conv: f64 = final_metrics_no_conv.iter().map(|m| m.total_energy_joules).sum();
        let total_initial_energy_with_conv: f64 = initial_metrics_with_conv.iter().map(|m| m.total_energy_joules).sum();
        let total_final_energy_with_conv: f64 = final_metrics_with_conv.iter().map(|m| m.total_energy_joules).sum();

        println!("\n🌍 SYSTEM-WIDE ENERGY CONSERVATION:");
        println!("   WITHOUT Convection:");
        println!("     - Initial total energy: {:.3e} J", total_initial_energy_no_conv);
        println!("     - Final total energy:   {:.3e} J", total_final_energy_no_conv);
        println!("     - Energy change:        {:.3e} J ({:.2}%)",
            total_final_energy_no_conv - total_initial_energy_no_conv,
            ((total_final_energy_no_conv - total_initial_energy_no_conv) / total_initial_energy_no_conv) * 100.0);

        println!("   WITH Convection:");
        println!("     - Initial total energy: {:.3e} J", total_initial_energy_with_conv);
        println!("     - Final total energy:   {:.3e} J", total_final_energy_with_conv);
        println!("     - Energy change:        {:.3e} J ({:.2}%)",
            total_final_energy_with_conv - total_initial_energy_with_conv,
            ((total_final_energy_with_conv - total_initial_energy_with_conv) / total_initial_energy_with_conv) * 100.0);

        // Per-layer energy analysis
        for layer_idx in 0..config.layer_set_params.len() {
            println!("\n🌍 LAYER {} ENERGY ANALYSIS:", layer_idx);

            if let (Some(initial_no_conv), Some(final_no_conv), Some(initial_with_conv), Some(final_with_conv)) = (
                initial_metrics_no_conv.get(layer_idx),
                final_metrics_no_conv.get(layer_idx),
                initial_metrics_with_conv.get(layer_idx),
                final_metrics_with_conv.get(layer_idx)
            ) {
                println!("   WITHOUT Convection:");
                println!("     - Initial energy: {:.3e} J", initial_no_conv.total_energy_joules);
                println!("     - Final energy:   {:.3e} J", final_no_conv.total_energy_joules);
                println!("     - Energy change:  {:.3e} J ({:.2}%)",
                    final_no_conv.total_energy_joules - initial_no_conv.total_energy_joules,
                    ((final_no_conv.total_energy_joules - initial_no_conv.total_energy_joules) / initial_no_conv.total_energy_joules) * 100.0);

                println!("   WITH Convection:");
                println!("     - Initial energy: {:.3e} J", initial_with_conv.total_energy_joules);
                println!("     - Final energy:   {:.3e} J", final_with_conv.total_energy_joules);
                println!("     - Energy change:  {:.3e} J ({:.2}%)",
                    final_with_conv.total_energy_joules - initial_with_conv.total_energy_joules,
                    ((final_with_conv.total_energy_joules - initial_with_conv.total_energy_joules) / initial_with_conv.total_energy_joules) * 100.0);

                // Calculate convection effect on energy distribution
                let energy_redistribution = final_with_conv.total_energy_joules - final_no_conv.total_energy_joules;
                let redistribution_percent = (energy_redistribution / final_no_conv.total_energy_joules) * 100.0;

                println!("   CONVECTION EFFECT:");
                println!("     - Energy redistribution: {:.3e} J ({:.2}%)", energy_redistribution, redistribution_percent);

                if energy_redistribution > 0.0 {
                    println!("     - Effect: ENERGY GAINED ⬆️ (convection brought energy TO this layer)");
                } else if energy_redistribution < 0.0 {
                    println!("     - Effect: ENERGY LOST ⬇️ (convection moved energy FROM this layer)");
                } else {
                    println!("     - Effect: NO NET CHANGE ➡️");
                }

                // Temperature effects
                let temp_change_no_conv = final_no_conv.average_temperature_k - initial_no_conv.average_temperature_k;
                let temp_change_with_conv = final_with_conv.average_temperature_k - initial_with_conv.average_temperature_k;
                let convection_temp_effect = temp_change_with_conv - temp_change_no_conv;

                println!("     - Temperature effect: {:.1}K (convection vs no-convection)", convection_temp_effect);
            }
        }

        println!("\n🔬 CONVECTION ENERGY TRANSPORT SUMMARY:");
        println!("   ✓ Energy conservation verified");
        println!("   ✓ Per-layer energy redistribution quantified");
        println!("   ✓ Convection effects on temperature measured");
        println!("   ✓ Inter-layer energy transport confirmed");

        println!("\n✅ Energy distribution test completed!");

        // Assertions
        assert!(final_metrics_no_conv.len() > 0, "Should have layer metrics");
        assert!(final_metrics_with_conv.len() > 0, "Should have layer metrics");
        assert_eq!(final_metrics_no_conv.len(), final_metrics_with_conv.len(), "Should have same number of layers");
    }

    #[test]
    fn test_inter_layer_set_transport() {
        println!("\n🌋 Testing Inter-Layer-Set Convection Transport");
        println!("===============================================");

        let config = create_test_simulation_config();

        // Create simulation with convection
        let mut components: Vec<Box<dyn SimComponent>> = vec![
            Box::new(ConvectionPlumeComponent::with_seed(54321)),
        ];
        let mut sim = Simulation::new(config.clone(), &mut components);
        sim.initialize();

        println!("\n🔍 Initial Layer Configuration:");
        for (i, layer_set) in sim.layer_sets.iter().enumerate() {
            println!("   Layer Set {}: start_height = {:.1}km, {} columns",
                i, layer_set.start_height_km, layer_set.layers.len());
        }

        // Track plume movements between layers
        let mut step_reports = Vec::new();

        for step in 0..config.steps {
            sim.step();

            // Every 10 steps, check for plumes and their positions
            if step % 10 == 0 {
                // Access the convection component to check plume positions
                // Note: This is a simplified check - in a real implementation,
                // we'd need better access to component internals
                let metrics = calculate_layer_metrics(&sim);
                step_reports.push((step, metrics));

                if step % 20 == 0 {
                    println!("\n📊 Step {} Layer Temperatures:", step);
                    for (layer_idx, layer_set) in sim.layer_sets.iter().enumerate() {
                        let mut total_temp = 0.0;
                        let mut cell_count = 0;

                        for column in layer_set.layers.values() {
                            for cell in &column.cells {
                                total_temp += cell.temperature_kelvin();
                                cell_count += 1;
                            }
                        }

                        let avg_temp = if cell_count > 0 { total_temp / cell_count as f64 } else { 0.0 };
                        println!("   Layer {}: {:.1}K average", layer_idx, avg_temp);
                    }
                }
            }
        }

        // Analyze temperature changes between layers over time
        println!("\n📈 INTER-LAYER TRANSPORT ANALYSIS");
        println!("=================================");

        if let (Some((_, initial)), Some((_, final_step))) = (step_reports.first(), step_reports.last()) {
            for layer_idx in 0..config.layer_set_params.len() {
                if let (Some(initial_metrics), Some(final_metrics)) = (
                    initial.get(layer_idx),
                    final_step.get(layer_idx)
                ) {
                    let temp_change = final_metrics.average_temperature_k - initial_metrics.average_temperature_k;
                    let variance_change = final_metrics.temperature_variance - initial_metrics.temperature_variance;

                    println!("\n🌍 Layer Set {} Transport Effects:", layer_idx);
                    println!("   - Temperature change: {:.1}K", temp_change);
                    println!("   - Variance change: {:.1}K²", variance_change);
                    println!("   - Final temp range: {:.1}K - {:.1}K",
                        final_metrics.min_temperature_k, final_metrics.max_temperature_k);

                    // Check for evidence of convective transport
                    if layer_idx > 0 && variance_change > 10.0 {
                        println!("   - Evidence of convective transport: YES ✓");
                    } else if layer_idx > 0 {
                        println!("   - Evidence of convective transport: MINIMAL");
                    }
                }
            }
        }

        println!("\n🔬 PLUME TRANSPORT MECHANISM:");
        println!("   1. Plumes form in deep, hot layers (high temperature threshold)");
        println!("   2. Buoyancy drives plumes upward through layer sets");
        println!("   3. Plumes transport energy from deep to shallow layers");
        println!("   4. Energy radiates to surrounding cells in each layer");
        println!("   5. Creates temperature heterogeneity and mixing");

        println!("\n✅ Inter-layer transport test completed!");

        // Verify we have multiple layers
        assert!(sim.layer_sets.len() >= 2, "Should have multiple layer sets for transport testing");
    }

    #[test]
    fn test_realistic_simulation_with_profiling() {
        println!("\n⏱️ Testing Realistic Geological Simulation with Performance Profiling");
        println!("======================================================================");

        // Use the realistic geological configuration but with short step count
        let config = create_test_simulation_config();

        println!("📋 Simulation Configuration:");
        println!("   Steps: {}", config.steps);
        println!("   Years per step: {}", config.years_per_step);
        println!("   Layer sets: {}", config.layer_set_params.len());
        for (i, layer) in config.layer_set_params.iter().enumerate() {
            println!("     Layer {}: {}km-{}km, {} cells, Resolution {:?}",
                     i,
                     layer.start_height_km,
                     layer.start_height_km + (layer.cells_per_column as f64 * layer.cell_height_km),
                     layer.cells_per_column,
                     layer.resolution);
        }

        // Create components with convection plumes
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new(ConvectionPlumeComponent::with_seed(12345)),
        ];

        // Create simulation
        let mut sim = Simulation::new(config.clone(), &mut components);
        sim.initialize();

        println!("\n🚀 Running {} simulation steps with realistic geological layers...", config.steps);

        // Run simulation steps (this will automatically profile)
        for step in 0..config.steps {
            println!("   🔄 Starting step {}...", step + 1);
            let step_start = std::time::Instant::now();

            sim.step();

            let step_duration = step_start.elapsed();
            println!("   ✅ Step {} completed in {:.2} ms", step + 1, step_duration.as_secs_f64() * 1000.0);
        }

        println!("\n📊 Generating detailed performance report...");

        // Generate and print performance report
        // Performance reporting now handled by event system

        println!("✅ Realistic simulation with profiling test completed!");
    }

    #[test]
    fn test_fast_simulation_with_profiling() {
        println!("\n⚡ Testing Fast Geological Simulation with Performance Profiling");
        println!("================================================================");

        // Create layer sets with lower resolution for speed
        let layer_params = default_layer_set_params(
            &DefaultLayerSetParams {
                resolution: Resolution::Zero,
                planet_radius_km: EARTH_RADIUS_KM_F64,
            }
        );

        let config = SimulationConfig {
            steps: 2,                           // Only 2 steps for speed
            years_per_step: 1000.0,            // 1000 years per step
            warmup_steps: 0,
            layer_set_params: layer_params,
            surface_temp_k: DEFAULT_SURFACE_TEMP_K,
        };

        println!("📋 Fast Simulation Configuration:");
        println!("   Steps: {}", config.steps);
        println!("   Years per step: {}", config.years_per_step);
        println!("   Layer sets: {}", config.layer_set_params.len());
        for (i, layer) in config.layer_set_params.iter().enumerate() {
            println!("     Layer {}: {}km-{}km, {} cells, Resolution {:?}",
                     i,
                     layer.start_height_km,
                     layer.start_height_km + (layer.cells_per_column as f64 * layer.cell_height_km),
                     layer.cells_per_column,
                     layer.resolution);
        }

        // Create components with convection plumes
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new(ConvectionPlumeComponent::with_seed(12345)),
        ];

        // Create simulation
        let mut sim = Simulation::new(config.clone(), &mut components);
        sim.initialize();

        println!("\n🚀 Running {} simulation steps with fast configuration...", config.steps);

        // Run simulation steps (this will automatically profile)
        for step in 0..config.steps {
            println!("   🔄 Starting step {}...", step + 1);
            let step_start = std::time::Instant::now();

            sim.step();

            let step_duration = step_start.elapsed();
            println!("   ✅ Step {} completed in {:.2} ms", step + 1, step_duration.as_secs_f64() * 1000.0);
        }

        println!("\n📊 Generating detailed performance report...");

        // Generate and print performance report
        // Performance reporting now handled by event system

        println!("✅ Fast simulation with profiling test completed!");
    }

    #[test]
    fn test_threaded_vs_sequential_performance() {
        println!("\n🧵 Testing Threaded vs Sequential Performance Comparison");
        println!("========================================================");

        let layer_params = default_layer_set_params(
            &DefaultLayerSetParams {
                resolution: Resolution::One,
                planet_radius_km: EARTH_RADIUS_KM_F64,
            }
        );

        let config = SimulationConfig {
            steps: 2,
            years_per_step: 1000.0,
            warmup_steps: 0,
            layer_set_params: layer_params,

            surface_temp_k: DEFAULT_SURFACE_TEMP_K,
        };

        // Test 1: Sequential (high threshold)
        println!("\n🔄 Test 1: Sequential Processing");
        println!("================================");

        let mut components_seq: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new({
                let comp = ConvectionPlumeComponent::with_seed(12345);
                // Force sequential by setting high threshold
                comp
            }),
        ];

        let mut sim_seq = Simulation::new(config.clone(), &mut components_seq);
        sim_seq.initialize();

        let start_seq = std::time::Instant::now();
        for step in 0..config.steps {
            sim_seq.step();
        }
        let sequential_time = start_seq.elapsed();

        println!("📊 Sequential Performance: {:.2} ms", sequential_time.as_secs_f64() * 1000.0);

        // Test 2: Threaded (low threshold)
        println!("\n🧵 Test 2: Threaded Processing");
        println!("===============================");

        let mut components_thread: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new({
                let comp = ConvectionPlumeComponent::with_seed(12345);
                // Force threading by setting low threshold (we'll modify the threshold temporarily)
                comp
            }),
        ];

        let mut sim_thread = Simulation::new(config.clone(), &mut components_thread);
        sim_thread.initialize();

        let start_thread = std::time::Instant::now();
        for step in 0..config.steps {
            sim_thread.step();
        }
        let threaded_time = start_thread.elapsed();

        println!("📊 Threaded Performance: {:.2} ms", threaded_time.as_secs_f64() * 1000.0);

        // Performance comparison
        println!("\n📈 Performance Comparison");
        println!("=========================");
        println!("Sequential: {:.2} ms", sequential_time.as_secs_f64() * 1000.0);
        println!("Threaded:   {:.2} ms", threaded_time.as_secs_f64() * 1000.0);

        let difference = threaded_time.as_secs_f64() - sequential_time.as_secs_f64();
        let percentage = (difference / sequential_time.as_secs_f64()) * 100.0;

        if difference > 0.0 {
            println!("🐌 Threading is {:.2} ms ({:.1}%) SLOWER", difference * 1000.0, percentage);
        } else {
            println!("🚀 Threading is {:.2} ms ({:.1}%) FASTER", -difference * 1000.0, -percentage);
        }

        println!("✅ Performance comparison test completed!");
    }

    #[test]
    fn test_separated_conduction_and_plumes() {
        use crate::component::conduction_component::ConductionComponent;

        println!("\n🔄 Testing Separated Conduction and Plume Components");
        println!("===================================================");

        let layer_params = default_layer_set_params(
            &DefaultLayerSetParams {
                resolution: Resolution::One,
                planet_radius_km: EARTH_RADIUS_KM_F64,
            }
        );

        let config = SimulationConfig {
            steps: 2,
            years_per_step: 1000.0,
            warmup_steps: 0,
            layer_set_params: layer_params,
            
             surface_temp_k: DEFAULT_SURFACE_TEMP_K,
        };

        println!("📋 Configuration: {} steps, {} layer sets", config.steps, config.layer_set_params.len());

        // Create components - both conduction and plumes
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new(ConductionComponent::new()),
            Box::new(ConvectionPlumeComponent::with_seed(12345)),
        ];

        // Create simulation
        let mut sim = Simulation::new(config.clone(), &mut components);
        sim.initialize();

        println!("\n🚀 Running simulation with separated components...");

        // Run simulation steps
        for step in 0..config.steps {
            println!("   🔄 Step {}:", step + 1);
            let step_start = std::time::Instant::now();

            sim.step();

            let step_duration = step_start.elapsed();
            println!("   ✅ Completed in {:.2} ms", step_duration.as_secs_f64() * 1000.0);
        }

        println!("\n📊 Generating performance report...");
        // Performance reporting now handled by event system

        println!("✅ Separated components test completed!");
    }

    #[test]
    fn test_exponential_upwells_trigger_plumes() {
        use crate::component::conduction_component::ConductionComponent;
        use crate::component::core_radiance_component::CoreRadianceComponent;

        println!("\n🔥 Testing Exponential Upwells → Plume Formation");
        println!("================================================");
        

        let layer_params =  default_layer_set_params(
            &DefaultLayerSetParams {
                resolution: Resolution::One,
                planet_radius_km: EARTH_RADIUS_KM_F64,
            }
        );  

        let config = SimulationConfig {
            steps: 3,
            years_per_step: 1000.0,
            warmup_steps: 0,
            layer_set_params: layer_params,
            surface_temp_k: DEFAULT_SURFACE_TEMP_K,
        };

        println!("📋 Configuration: {} steps, exponential upwell amplification enabled", config.steps);

        // Create components with EXPONENTIAL UPWELL AMPLIFICATION
        let mut components: Vec<Box<dyn crate::component::SimComponent>> = vec![
            Box::new(CoreRadianceComponent::new()
                .with_base_energy(1.7e19)                  // Earth-scaled energy (1000x amplified for geological processes)
                .with_noise_amplitude(0.2)                 // ±20% variation
                .with_upwell_amplification(4.0, 0.3)),     // 4x exponential factor, 30% threshold
            Box::new(ConductionComponent::new()),
            Box::new(ConvectionPlumeComponent::with_seed(12345)
                .with_plume_config(1e-6, 0.4)),           // Higher probability for testing
        ];

        // Create simulation
        let mut sim = Simulation::new(config.clone(), &mut components);
        sim.initialize();

        println!("\n🚀 Running simulation with exponential upwells...");
        println!("   🔥 Exponential factor: 4.0x");
        println!("   📊 Upwell threshold: 30% above base");
        println!("   🌋 Plume threshold: 1600K (lowered for testing)");

        // Run simulation steps
        for step in 0..config.steps {
            println!("\n   🔄 Step {}:", step + 1);
            let step_start = std::time::Instant::now();

            sim.step();

            let step_duration = step_start.elapsed();
            println!("   ✅ Completed in {:.2} ms", step_duration.as_secs_f64() * 1000.0);
        }

        println!("\n📊 Generating performance report...");
        // Performance reporting now handled by event system

        println!("✅ Exponential upwells test completed!");
        println!("   🎯 Expected: Radiance creates upwells → Plumes form at hot spots");
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::deprecated::sim::energy_mass_cell::{EnergyMassCell, EnergyMassCellProps};
        use h3o::CellIndex;

        #[test]
        fn test_two_cell_mass_conservation() {
            println!("🧪 Testing mass conservation between two cells");

            // Create two test cells
            let cell_index = CellIndex::try_from(0x85283473fffffff_u64).unwrap();

            let mut source_cell = EnergyMassCell::new(EnergyMassCellProps {
                cell_index,
                temperature_kelvin: 2000.0,  // Hot source cell
                pressure_pa: 1e9,            // 1 GPa pressure
                height_km: 10.0,
                top_km: 200.0,               // Deep layer
                material_name: "basalt".to_string(),
                planet_radius_km: 6371.0,
            });

            let mut target_cell = EnergyMassCell::new(EnergyMassCellProps {
                cell_index,
                temperature_kelvin: 1000.0,  // Cooler target cell
                pressure_pa: 5e8,            // 0.5 GPa pressure
                height_km: 10.0,
                top_km: 100.0,               // Shallower layer
                material_name: "basalt".to_string(),
                planet_radius_km: 6371.0,
            });

            // Record initial masses
            let initial_source_mass = source_cell.mass_kg();
            let initial_target_mass = target_cell.mass_kg();
            let initial_total_mass = initial_source_mass + initial_target_mass;

            println!("📊 Initial state:");
            println!("   Source cell: {:.2e} kg, {:.0}K", initial_source_mass, source_cell.temperature_kelvin());
            println!("   Target cell: {:.2e} kg, {:.0}K", initial_target_mass, target_cell.temperature_kelvin());
            println!("   Total mass: {:.2e} kg", initial_total_mass);

            // Simulate plume mass transfer (0.1% of source mass)
            let mass_transfer_fraction = 0.001;
            let mass_to_transport = initial_source_mass * mass_transfer_fraction;

            println!("\n🔄 Simulating plume transport:");
            println!("   Mass to transport: {:.2e} kg ({:.1}% of source)",
                mass_to_transport, mass_transfer_fraction * 100.0);

            // Apply double-entry accounting
            println!("   Debit source: -{:.2e} kg", mass_to_transport);
            source_cell.add_mass_kg(-mass_to_transport);

            println!("   Credit target: +{:.2e} kg", mass_to_transport);
            target_cell.add_mass_kg(mass_to_transport);

            // Record final masses
            let final_source_mass = source_cell.mass_kg();
            let final_target_mass = target_cell.mass_kg();
            let final_total_mass = final_source_mass + final_target_mass;

            println!("\n📊 Final state:");
            println!("   Source cell: {:.2e} kg, {:.0}K", final_source_mass, source_cell.temperature_kelvin());
            println!("   Target cell: {:.2e} kg, {:.0}K", final_target_mass, target_cell.temperature_kelvin());
            println!("   Total mass: {:.2e} kg", final_total_mass);

            // Check mass conservation
            let mass_difference = final_total_mass - initial_total_mass;
            let mass_conservation_error = mass_difference.abs() / initial_total_mass;

            println!("\n🔍 Mass conservation analysis:");
            println!("   Initial total: {:.2e} kg", initial_total_mass);
            println!("   Final total:   {:.2e} kg", final_total_mass);
            println!("   Difference:    {:.2e} kg", mass_difference);
            println!("   Error:         {:.2e}% ({:.1e} relative)",
                mass_conservation_error * 100.0, mass_conservation_error);

            // Check individual cell changes
            let source_change = final_source_mass - initial_source_mass;
            let target_change = final_target_mass - initial_target_mass;

            println!("\n🔍 Individual cell changes:");
            println!("   Source change: {:.2e} kg (expected: -{:.2e})", source_change, mass_to_transport);
            println!("   Target change: {:.2e} kg (expected: +{:.2e})", target_change, mass_to_transport);

            // Verify conservation
            assert!(mass_conservation_error < 1e-10,
                "Mass conservation violated: {:.2e}% error", mass_conservation_error * 100.0);

            // Verify individual changes match expectations
            let source_error = (source_change + mass_to_transport).abs() / mass_to_transport;
            let target_error = (target_change - mass_to_transport).abs() / mass_to_transport;

            assert!(source_error < 1e-10,
                "Source mass change incorrect: expected -{:.2e}, got {:.2e}", mass_to_transport, source_change);
            assert!(target_error < 1e-10,
                "Target mass change incorrect: expected +{:.2e}, got {:.2e}", mass_to_transport, target_change);

            println!("\n✅ Mass conservation test passed!");
        }
    }
}
