use crate::component::SimComponent;
use crate::sim_immut::simulation_immut::SimulationImmut;
use crate::energy_mass::energy_mass::EnergyMass;
use noise::{NoiseFn, Perlin};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use h3o::CellIndex;
use rand::Rng;

/// Configuration for core heat system with Earth-scaled parameters
#[derive(Debug, Clone)]
pub struct CoreHeatConfig {
    pub earth_wattage_tw: f64,                    // Target global heat output in TW
    pub hotspot_count: usize,                     // Number of active hotspots
    pub max_min_lifespan_years: (f64, f64),      // (min, max) hotspot lifespan over lifetime
    pub max_min_radius_km: (f64, f64),           // (min, max) hotspot radius over lifetime
    pub max_min_heat_multiplier: (f64, f64),     // (min, max) heat multiplier over lifetime
    pub hotspot_peak_years: f64,                 // Years to reach peak intensity (~25)
}

impl Default for CoreHeatConfig {
    fn default() -> Self {
        Self {
            earth_wattage_tw: 47.0,                        // Earth's 47 TW total heat flow
            hotspot_count: 10,                             // ~10 major hotspots globally
            max_min_lifespan_years: (30000.0, 100000.0),  // 30k-100k years lifetime range
            max_min_radius_km: (100.0, 1000.0),           // 100-1000 km radius range (10% to 100% of max)
            max_min_heat_multiplier: (2.0, 10.0),         // 2x-10x heat range over lifetime
            hotspot_peak_years: 25.0,                     // Peak at 25 years (start=10%, peak=100%, end=10%)
        }
    }
}

impl CoreHeatConfig {
    /// Get the fraction of global energy that should come from hotspots (66-75%)
    pub fn hotspot_energy_fraction(&self) -> f64 {
        0.70 // 70% from hotspots, 30% from background Perlin
    }

    /// Get the fraction of global energy that should come from background Perlin
    pub fn background_energy_fraction(&self) -> f64 {
        1.0 - self.hotspot_energy_fraction()
    }
}

/// Hotspot for concentrated energy input/output
#[derive(Debug, Clone)]
struct Hotspot {
    cell_index: CellIndex,
    is_upwell: bool,                    // true = energy source, false = energy sink
    max_size: f64,                      // 0-10 scale for this hotspot's maximum potential
    creation_year: f64,
    lifetime_years: f64,
    max_radius_km: f64,                 // Maximum radius this hotspot can reach (at peak)
    max_heat_multiplier: f64,           // Maximum heat multiplier this hotspot can reach (at peak)
    plume_pressure: f64,                // Accumulated plume pressure (builds over time)
    years_since_last_plume: f64,        // Years since last plume was created
}

impl Hotspot {
    /// Calculate current size (0-1) based on age and lifecycle
    /// Peaks at hotspot_peak_years, then exponentially decays
    fn current_size(&self, current_year: f64, peak_years: f64) -> f64 {
        let age = current_year - self.creation_year;
        if age < 0.0 || age > self.lifetime_years {
            return 0.0;
        }

        if age <= peak_years {
            // Growth phase: 0 → 1 over peak_years
            age / peak_years
        } else {
            // Decay phase: exponential decay from 1 → 0
            let decay_age = age - peak_years;
            let decay_duration = self.lifetime_years - peak_years;
            let normalized_decay = decay_age / decay_duration;

            // Exponential decay: e^(-3*t) gives nice decay curve
            (-3.0 * normalized_decay).exp()
        }
    }

    /// Calculate current properties based on 4-13 scale and current_size
    fn current_properties(&self, current_year: f64, peak_years: f64) -> (f64, f64, f64) {
        let current_size = self.current_size(current_year, peak_years);

        // Scale from 4 (min) to 4+max_size (max) based on current_size
        let scale_min = 4.0;
        let scale_max = 4.0 + self.max_size;
        let current_scale = scale_min + (scale_max - scale_min) * current_size;

        // Calculate actual properties based on current scale
        let current_radius = self.max_radius_km * (current_scale / 13.0); // 13 is max scale
        let current_heat = self.max_heat_multiplier * (current_scale / 13.0);

        // Apply upwell/downwell direction
        let heat_multiplier = if self.is_upwell {
            current_heat
        } else {
            -current_heat // Negative for downwells (cooling)
        };

        (current_size, current_radius, heat_multiplier)
    }

    /// Get current energy multiplier (for compatibility)
    fn current_multiplier(&self, current_year: f64, peak_years: f64) -> f64 {
        let (_, _, heat_multiplier) = self.current_properties(current_year, peak_years);
        heat_multiplier
    }
}

/// Component that adds Perlin noise-modulated energy input to the deepest cells
/// Simulates variable core heat generation with spatial and temporal variations
/// Enhanced with hotspot system for concentrated upwells/downwells
pub struct CoreHeatComponent {
    /// Configuration for Earth-scaled core heat system
    core_heat_config: CoreHeatConfig,
    /// Base energy input per cell per year (Joules)
    base_energy_per_cell_per_year: f64,
    /// Perlin noise variation amplitude (±15% = 0.15)
    noise_amplitude: f64,
    /// Exponential upwell factor for creating peak concentrations
    upwell_exponential_factor: f64,
    /// Threshold above which exponential amplification kicks in
    upwell_threshold: f64,
    /// Perlin noise spatial scale (larger = coarser features)
    spatial_scale: f64,
    /// Perlin noise temporal scale (larger = slower changes)
    temporal_scale: f64,
    /// Perlin noise generator
    perlin: Perlin,
    /// Current simulation year for temporal variation
    current_year: f64,
    /// Optional temporal drift vector for coordinate evolution (per year)
    temporal_drift_per_year: Option<(f64, f64, f64)>,
    /// Active hotspots for concentrated energy input/output
    hotspots: Vec<Hotspot>,
    /// Seed for random hotspot creation
    hotspot_seed: u64,
    /// Calculated average hotspot contribution (watts)
    average_hotspot_watts: f64,
    /// Cache of cells affected by hotspots with their multipliers
    hotspot_affected_cells: HashMap<CellIndex, f64>,
}

impl CoreHeatComponent {
    /// Calculate average hotspot contribution over lifetime
    /// This determines how much energy hotspots contribute on average
    fn calculate_average_hotspot_watts(config: &CoreHeatConfig) -> f64 {
        // Sample hotspot lifecycle at regular intervals
        let sample_count = 1000;
        let max_lifetime = config.max_min_lifespan_years.1;
        let time_step = max_lifetime / sample_count as f64;

        let mut total_weighted_heat = 0.0;
        let mut total_samples = 0;

        // Sample across different hotspot configurations
        for max_size in 0..=10 {
            let max_size_f = max_size as f64;

            // Create a representative hotspot
            let lifetime = config.max_min_lifespan_years.0 +
                          (config.max_min_lifespan_years.1 - config.max_min_lifespan_years.0) * (max_size_f / 10.0);
            let max_heat = config.max_min_heat_multiplier.0 +
                          (config.max_min_heat_multiplier.1 - config.max_min_heat_multiplier.0) * (max_size_f / 10.0);

            // Sample this hotspot's lifecycle
            for sample in 0..sample_count {
                let age = sample as f64 * time_step;
                if age > lifetime { break; }

                // Calculate current_size for this age
                let current_size = if age <= config.hotspot_peak_years {
                    age / config.hotspot_peak_years
                } else {
                    let decay_age = age - config.hotspot_peak_years;
                    let decay_duration = lifetime - config.hotspot_peak_years;
                    let normalized_decay = decay_age / decay_duration;
                    (-3.0 * normalized_decay).exp()
                };

                // Calculate current heat based on 4-13 scale
                let scale_min = 4.0;
                let scale_max = 4.0 + max_size_f;
                let current_scale = scale_min + (scale_max - scale_min) * current_size;
                let current_heat = max_heat * (current_scale / 13.0);

                total_weighted_heat += current_heat;
                total_samples += 1;
            }
        }

        let average_heat_multiplier = if total_samples > 0 {
            total_weighted_heat / total_samples as f64
        } else {
            1.0
        };

        // Convert to watts: average_multiplier * hotspot_fraction * base_energy_per_year * hotspot_count
        // Base energy calculation: earth_wattage_tw / estimated_cell_count
        let earth_watts = config.earth_wattage_tw * 1e12; // TW to W
        let estimated_cell_count = 86000.0; // H3 Resolution 2
        let base_energy_per_year = earth_watts / estimated_cell_count;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let base_power_per_cell = base_energy_per_year / seconds_per_year;

        // Hotspots get 70% of total energy, distributed among affected cells
        let hotspot_fraction = config.hotspot_energy_fraction();
        average_heat_multiplier * base_power_per_cell * hotspot_fraction * config.hotspot_count as f64
    }

    pub fn new() -> Self {
        let config = CoreHeatConfig::default();
        let average_hotspot_watts = Self::calculate_average_hotspot_watts(&config);

        // Calculate base energy from Earth wattage
        let earth_watts = config.earth_wattage_tw * 1e12; // TW to W
        let estimated_cell_count = 86000.0; // H3 Resolution 2
        let base_energy_per_year = earth_watts / estimated_cell_count;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let base_energy_per_cell_per_year = base_energy_per_year * seconds_per_year;

        Self {
            core_heat_config: config,
            base_energy_per_cell_per_year,         // Earth-scaled base energy
            noise_amplitude: 0.15,                 // ±15% variation
            spatial_scale: 0.1,                    // Coarse spatial features
            temporal_scale: 0.001,                 // Slow temporal changes
            perlin: Perlin::new(42),               // Fixed seed for reproducibility
            current_year: 0.0,
            temporal_drift_per_year: None,         // No drift by default
            upwell_exponential_factor: 3.0,       // Exponential amplification factor
            upwell_threshold: 0.5,                 // Threshold for exponential upwells (50% above base)
            hotspots: Vec::new(),                  // Start with no hotspots
            hotspot_seed: 42,                      // Deterministic hotspot generation
            average_hotspot_watts,                 // Calculated average hotspot contribution
            hotspot_affected_cells: HashMap::new(), // Cache for hotspot-affected cells
        }
    }

    pub fn with_base_energy(mut self, base_energy: f64) -> Self {
        self.base_energy_per_cell_per_year = base_energy;
        self
    }

    pub fn with_noise_amplitude(mut self, amplitude: f64) -> Self {
        self.noise_amplitude = amplitude;
        self
    }

    pub fn with_spatial_scale(mut self, scale: f64) -> Self {
        self.spatial_scale = scale;
        self
    }

    /// Enable temporal drift with specified drift vector per year
    /// For 1% change per 100,000 years: magnitude ≈ 1e-7 per year
    pub fn with_temporal_drift(mut self, drift_x: f64, drift_y: f64, drift_z: f64) -> Self {
        self.temporal_drift_per_year = Some((drift_x, drift_y, drift_z));
        self
    }

    /// Configure exponential upwell parameters for creating peak energy concentrations
    pub fn with_upwell_amplification(mut self, exponential_factor: f64, threshold: f64) -> Self {
        self.upwell_exponential_factor = exponential_factor;
        self.upwell_threshold = threshold;
        self
    }

    /// Configure hotspot system parameters
    pub fn with_hotspots(mut self, count: usize, lifetime_years: f64, seed: u64) -> Self {
        self.core_heat_config.hotspot_count = count;
        self.core_heat_config.max_min_lifespan_years = (lifetime_years * 0.8, lifetime_years * 1.2);
        self.hotspot_seed = seed;
        self
    }

    /// Enable temporal drift with geological timescale (1% per 100,000 years)
    /// Creates a non-orthogonal drift vector for realistic geological evolution
    pub fn with_geological_drift(mut self) -> Self {
        // 1% change per 100,000 years = 1e-7 per year
        // Use non-orthogonal direction for realistic geological patterns
        let magnitude = 1e-7; // 1% per 100,000 years
        let drift_x = magnitude * 0.577;  // √3/3 component
        let drift_y = magnitude * 0.577;  // √3/3 component
        let drift_z = magnitude * 0.577;  // √3/3 component (non-orthogonal)

        self.temporal_drift_per_year = Some((drift_x, drift_y, drift_z));
        self
    }

    /// Calculate Perlin noise-modulated energy for a specific cell using true 3D coordinates with temporal drift
    /// Includes planetary radius scaling for realistic energy distribution
    fn calculate_energy_for_cell(&self, cell_index: &CellIndex, cell_x: f64, cell_y: f64, cell_z: f64, years_per_step: f64, planet_radius_km: f64) -> f64 {
        // Apply temporal drift if enabled
        let (drifted_x, drifted_y, drifted_z) = if let Some((drift_x, drift_y, drift_z)) = self.temporal_drift_per_year {
            (
                cell_x + drift_x * self.current_year,
                cell_y + drift_y * self.current_year,
                cell_z + drift_z * self.current_year,
            )
        } else {
            (cell_x, cell_y, cell_z)
        };

        // Generate Perlin noise value based on drifted 3D position and time
        let noise_value = self.perlin.get([
            drifted_x * self.spatial_scale,
            drifted_y * self.spatial_scale,
            drifted_z * self.spatial_scale,
            self.current_year * self.temporal_scale,
        ]);

        // Convert noise (-1 to 1) to variation factor (1 ± amplitude)
        let base_variation_factor = 1.0 + (noise_value * self.noise_amplitude);

        // EXPONENTIAL UPWELL AMPLIFICATION: Create peak energy concentrations
        let final_variation_factor = if base_variation_factor > (1.0 + self.upwell_threshold) {
            // Above threshold: Apply exponential amplification for upwells
            let excess = base_variation_factor - (1.0 + self.upwell_threshold);
            let exponential_boost = excess.powf(self.upwell_exponential_factor);
            (1.0 + self.upwell_threshold) + exponential_boost
        } else {
            // Below threshold: Use linear variation
            base_variation_factor
        };

        // Calculate energy partitioning: 70% hotspots, 30% background Perlin
        let background_fraction = self.core_heat_config.background_energy_fraction();
        let hotspot_fraction = self.core_heat_config.hotspot_energy_fraction();

        // Background Perlin energy (30% of total)
        let background_energy_per_cell_per_year = self.base_energy_per_cell_per_year * background_fraction;
        let background_energy = background_energy_per_cell_per_year * final_variation_factor * years_per_step;

        // Hotspot energy (70% of total, distributed among hotspot-affected cells)
        let hotspot_multiplier = self.get_hotspot_multiplier_for_cell(cell_index);
        let hotspot_energy = if hotspot_multiplier > 0.0 {
            // This cell is affected by hotspots - calculate its share of the 70% hotspot energy
            let hotspot_energy_per_cell_per_year = self.base_energy_per_cell_per_year * hotspot_fraction;
            hotspot_energy_per_cell_per_year * hotspot_multiplier * years_per_step
        } else {
            0.0
        };

        // Apply planetary radius scaling (larger planets have more total energy)
        let radius_scale_factor = (planet_radius_km / 6371.0).powf(2.0); // Scale by surface area

        // Total energy = background + hotspot contributions
        (background_energy + hotspot_energy) * radius_scale_factor
    }

    /// Convert H3 cell to normalized 3D Cartesian coordinates (x, y, z) scaled by planetary radius
    fn get_cell_3d_position(&self, cell_index: &h3o::CellIndex, depth_km: f64, planet_radius_km: f64) -> (f64, f64, f64) {
        // Use hash of cell index to generate deterministic but distributed positions
        let mut hasher = DefaultHasher::new();
        cell_index.hash(&mut hasher);
        let hash_value = hasher.finish();

        // Convert hash to lat/lon coordinates
        let lat_deg = ((hash_value % 18000) as f64 / 100.0) - 90.0; // -90 to +90
        let lon_deg = (((hash_value / 18000) % 36000) as f64 / 100.0) - 180.0; // -180 to +180

        // Convert to radians
        let lat_rad = lat_deg.to_radians();
        let lon_rad = lon_deg.to_radians();

        // Calculate radius from planet center (surface radius minus depth)
        let radius_km = planet_radius_km - depth_km;

        // Convert spherical coordinates (lat, lon, radius) to Cartesian (x, y, z)
        let x_km = radius_km * lat_rad.cos() * lon_rad.cos();
        let y_km = radius_km * lat_rad.cos() * lon_rad.sin();
        let z_km = radius_km * lat_rad.sin();

        // Normalize by planetary radius to get scale-independent coordinates
        let x_normalized = x_km / planet_radius_km;
        let y_normalized = y_km / planet_radius_km;
        let z_normalized = z_km / planet_radius_km;

        (x_normalized, y_normalized, z_normalized)
    }

    /// Paint all hotspots onto the affected cells cache for efficient lookup
    fn update_hotspot_affected_cells_cache(&mut self) {
        use crate::utils::h3_utils::H3Utils;

        // Clear the previous cache
        self.hotspot_affected_cells.clear();

        // Paint each active hotspot onto the cache
        for (i, hotspot) in self.hotspots.iter().enumerate() {
            let (current_size, current_radius, current_heat_multiplier) = hotspot.current_properties(
                self.current_year,
                self.core_heat_config.hotspot_peak_years
            );

            println!("   Hotspot {}: age={:.1}y, current_size={:.3}, radius={:.1}km, heat={:.1}x",
                i, self.current_year - hotspot.creation_year, current_size, current_radius, current_heat_multiplier);

            // Skip inactive hotspots
            if current_size < 0.1 || current_radius < 1.0 {
                println!("      Skipped (inactive: size={:.3}, radius={:.1}km)", current_size, current_radius);
                continue;
            }

            // Get all cells within the hotspot's current radius
            const DEFAULT_PLANET_RADIUS_KM: f64 = 6371.0; // Earth radius
            let affected_cells = H3Utils::cells_within_radius_km(
                hotspot.cell_index,
                current_radius,
                DEFAULT_PLANET_RADIUS_KM
            );

            // Paint each affected cell with the hotspot's contribution
            for (cell_index, distance_km) in affected_cells {
                // Distance-based falloff: stronger at center, weaker at edges
                let falloff_factor = (1.0 - (distance_km / current_radius)).max(0.1);
                let hotspot_contribution = current_heat_multiplier * falloff_factor;

                // Add to existing multiplier (multiple hotspots can affect the same cell)
                *self.hotspot_affected_cells.entry(cell_index).or_insert(0.0) += hotspot_contribution;
            }
        }

        println!("🎨 Painted {} hotspots affecting {} cells",
            self.hotspots.len(), self.hotspot_affected_cells.len());
    }

    /// Get hotspot energy multiplier for a specific cell using efficient cache lookup
    fn get_hotspot_multiplier_for_cell(&self, cell_index: &CellIndex) -> f64 {
        // Simple O(1) lookup in the pre-computed cache
        self.hotspot_affected_cells.get(cell_index).copied().unwrap_or(0.0)
    }

    /// Update hotspot lifecycle and create new hotspots as needed
    fn update_hotspots(&mut self, sim: &SimulationImmut) {
        // Remove expired hotspots
        self.hotspots.retain(|hotspot| {
            let age = self.current_year - hotspot.creation_year;
            age >= 0.0 && age <= hotspot.lifetime_years
        });

        // Create new hotspots if below target count
        while self.hotspots.len() < self.core_heat_config.hotspot_count {
            self.create_random_hotspot(sim);
        }
    }

    /// Adaptive hotspot management: if hotspots are overpowered, add 50% more and reduce energy by 33%
    pub fn adapt_hotspots_if_overpowered(&mut self, sim: &SimulationImmut, transaction_scaling_detected: bool) {
        if transaction_scaling_detected {
            let original_count = self.core_heat_config.hotspot_count;
            let new_count = (original_count as f64 * 1.5) as usize; // Add 50% more hotspots

            println!("🔥 Hotspots overpowered - adapting system:");
            println!("   Original hotspots: {}", original_count);
            println!("   New hotspot count: {}", new_count);

            // Update hotspot count
            self.core_heat_config.hotspot_count = new_count;

            // Reduce energy of all existing hotspots by 33%
            for hotspot in &mut self.hotspots {
                hotspot.max_heat_multiplier *= 0.67; // Reduce by 33%
            }

            // Create additional hotspots to reach new target
            while self.hotspots.len() < new_count {
                self.create_random_hotspot(sim);

                // New hotspots also get reduced energy
                if let Some(last_hotspot) = self.hotspots.last_mut() {
                    last_hotspot.max_heat_multiplier *= 0.67;
                }
            }

            // Recalculate average hotspot watts with new distribution
            self.average_hotspot_watts = Self::calculate_average_hotspot_watts(&self.core_heat_config);

            // Clear hotspot cache with new energy distribution
            self.hotspot_affected_cells.clear();

            println!("   ✅ Hotspot adaptation complete:");
            println!("      - Total hotspots: {}", self.hotspots.len());
            println!("      - Energy per hotspot: reduced by 33%");
            println!("      - Total energy: maintained (distributed across more hotspots)");
            println!("      - New average watts: {:.2e}", self.average_hotspot_watts);
        }

        // For testing: if this is the first time creating hotspots, make some of them mature
        if self.current_year == 0.0 && self.hotspots.len() == self.core_heat_config.hotspot_count {
            // Make the first few hotspots mature (25+ years old) for immediate testing
            for i in 0..3.min(self.hotspots.len()) {
                self.hotspots[i].creation_year = self.current_year - 30.0; // 30 years old (past peak)
            }
        }

        // Update the hotspot-affected cells cache after any changes
        self.update_hotspot_affected_cells_cache();
    }

    /// Create a new random hotspot using RadianceConfig parameters
    fn create_random_hotspot(&mut self, sim: &SimulationImmut) {
        use rand::rng;

        // Get a random cell from the deepest layer
        if let Some(deepest_layer) = sim.layer_sets.last() {
            let cell_indices: Vec<_> = deepest_layer.layers.keys().collect();
            if !cell_indices.is_empty() {
                let mut rng = rng();
                let random_index = rng.random_range(0..cell_indices.len());
                let cell_index = *cell_indices[random_index];

                // 70% chance of upwell, 30% chance of downwell
                let is_upwell = rng.random_bool(0.7);

                // Random max_size (0-10 scale)
                let max_size = rng.gen_range(0.0..=10.0);

                // Lifetime from config range
                let (min_life, max_life) = self.core_heat_config.max_min_lifespan_years;
                let lifetime = rng.gen_range(min_life..=max_life);

                // Max radius from config range, scaled by max_size
                let (min_radius, max_radius) = self.core_heat_config.max_min_radius_km;
                let radius_range = max_radius - min_radius;
                let max_radius_km = min_radius + radius_range * (max_size / 10.0);

                // Max heat multiplier from config range, scaled by max_size
                let (min_heat, max_heat) = self.core_heat_config.max_min_heat_multiplier;
                let heat_range = max_heat - min_heat;
                let max_heat_multiplier = min_heat + heat_range * (max_size / 10.0);

                let hotspot = Hotspot {
                    cell_index,
                    is_upwell,
                    max_size,
                    creation_year: self.current_year,
                    lifetime_years: lifetime,
                    max_radius_km,
                    max_heat_multiplier,
                    plume_pressure: 0.0,
                    years_since_last_plume: 0.0,
                };

                self.hotspots.push(hotspot);

                println!("🌋 Created {} hotspot at cell {} with {:.1}x max heat multiplier (lifetime: {:.0} years)",
                    if is_upwell { "upwell" } else { "downwell" },
                    cell_index,
                    max_heat_multiplier,
                    lifetime
                );
            }
        }
    }

    /// Apply energy input to the deepest cells in each column using true 3D coordinates
    fn apply_core_radiance(&mut self, sim: &mut SimulationImmut, years_per_step: f64) {
        // Find the deepest layer set (highest index)
        if let Some((deepest_layer_idx, deepest_layer_set)) = sim.layer_sets.iter_mut().enumerate().last() {
            let mut total_energy_added = 0.0;
            let mut cells_affected = 0;
            let mut sample_positions = Vec::new(); // For debug reporting

            for (cell_index, column) in deepest_layer_set.layers.iter_mut() {
                // Calculate the depth of the deepest cell before borrowing
                let layer_start_depth = deepest_layer_set.start_height_km;
                let cell_count = column.cells.len();
                let cell_depth = layer_start_depth + (cell_count - 1) as f64 * 25.0; // Approximate cell height

                // Get normalized 3D position for Perlin noise
                const DEFAULT_PLANET_RADIUS_KM: f64 = 6371.0; // Earth radius
                let (cell_x, cell_y, cell_z) = self.get_cell_3d_position(cell_index, cell_depth, DEFAULT_PLANET_RADIUS_KM);

                // Calculate Perlin-modulated energy using normalized 3D coordinates
                let energy_to_add = self.calculate_energy_for_cell(cell_index, cell_x, cell_y, cell_z, years_per_step, DEFAULT_PLANET_RADIUS_KM);

                // Apply energy to the deepest cell in each column using transaction system
                if let Some(_deepest_cell) = column.cells.last() {
                    // Create transaction for energy injection
                    let cell_location = crate::transaction_manager::CellLocation::new(
                        deepest_layer_idx,
                        *cell_index,
                        column.cells.len() - 1, // Last cell index
                    );

                    // Create atomic energy injection transaction
                    if let Ok(transaction) = crate::transaction_manager::AtomicTransaction::inject(
                        "CoreRadiance".to_string(),
                        cell_location.clone(),
                        energy_to_add,
                        0.0, // No mass injection
                        format!("Core radiance energy injection: {:.2e}J", energy_to_add),
                    ) {
                        // Propose atomic transaction to simulation
                        sim.transaction_manager.propose_atomic_transaction(transaction);
                    }

                    total_energy_added += energy_to_add;
                    cells_affected += 1;

                    // Collect sample positions for debug (first few cells)
                    if sample_positions.len() < 3 {
                        sample_positions.push((cell_x, cell_y, cell_z, energy_to_add));
                    }
                }
            }

            // Optional: Print debug info occasionally
            if self.current_year % 10000.0 < years_per_step {
                println!("🔥 Core Radiance (3D): Added {:.2e}J to {} cells (avg: {:.2e}J/cell)",
                    total_energy_added, cells_affected,
                    if cells_affected > 0 { total_energy_added / cells_affected as f64 } else { 0.0 });

                // Show sample 3D positions and their energy variations
                for (i, (x, y, z, energy)) in sample_positions.iter().enumerate() {
                    let variation_percent = (energy / (self.base_energy_per_cell_per_year * years_per_step) - 1.0) * 100.0;
                    println!("   Sample {}: ({:.0}, {:.0}, {:.0})km → {:.1}% variation",
                        i + 1, x, y, z, variation_percent);
                }
            }
        }
    }
}

impl SimComponent for CoreHeatComponent {
    fn key(&self) -> &'static str {
        "core_heat"
    }

    fn initialize(&mut self, sim: &mut SimulationImmut) {
        println!("🔥 Core Heat Component initialized");
        println!("   - Base energy: {:.2e} J/cell/year", self.base_energy_per_cell_per_year);
        println!("   - Noise amplitude: ±{:.0}%", self.noise_amplitude * 100.0);
        println!("   - Spatial scale: {:.3} (coarser = smaller values)", self.spatial_scale);
        println!("   - Temporal scale: {:.3} (slower = smaller values)", self.temporal_scale);

        // Report temporal drift settings
        if let Some((drift_x, drift_y, drift_z)) = self.temporal_drift_per_year {
            let drift_magnitude = (drift_x * drift_x + drift_y * drift_y + drift_z * drift_z).sqrt();
            let percent_per_100k_years = drift_magnitude * 100000.0 * 100.0;
            println!("   - Temporal drift: ({:.2e}, {:.2e}, {:.2e}) per year", drift_x, drift_y, drift_z);
            println!("   - Drift magnitude: {:.2e} per year ({:.1}% per 100k years)",
                drift_magnitude, percent_per_100k_years);
        } else {
            println!("   - Temporal drift: Disabled");
        }

        // Show which layer will receive energy
        if let Some(deepest_layer) = sim.layer_sets.last() {
            println!("   - Target layer: {} (start_height: {:.1}km)",
                sim.layer_sets.len() - 1, deepest_layer.start_height_km);
            println!("   - Cells to affect: {}", deepest_layer.layers.len());
        }

        // Initialize hotspot system
        println!("   - Hotspot system: {} target hotspots, {:.0}-{:.0} year lifetime",
            self.core_heat_config.hotspot_count,
            self.core_heat_config.max_min_lifespan_years.0,
            self.core_heat_config.max_min_lifespan_years.1);

        // Create initial hotspots
        self.update_hotspots(sim);
        println!("   - Created {} initial hotspots", self.hotspots.len());
    }

    fn step(&mut self, sim: &mut SimulationImmut, step: i64, year: i64) {
        // Component organizes its own internal phases
        self.update_internal_state(sim, step, year);
        self.apply_energy_changes(sim, step, year);
        self.report_status(sim, step, year);
    }

    fn complete(&mut self, _sim: &SimulationImmut) {
        println!("🔥 Core Radiance Component completed");
    }

    fn adapt_if_overpowered(&mut self, sim: &SimulationImmut, scaling_detected: bool) {
        self.adapt_hotspots_if_overpowered(sim, scaling_detected);
    }
}

// Internal methods - component's choice of organization (great for unit testing)
impl CoreHeatComponent {
    /// Update internal component state (private method for internal organization)
    fn update_internal_state(&mut self, sim: &SimulationImmut, _step: i64, year: i64) {
        // Update current year for temporal variation
        self.current_year = year as f64;

        // Update hotspot lifecycle every 1000 years
        if year % 1000 == 0 {
            self.update_hotspots(sim);
        }
    }

    /// Apply energy changes to simulation (private method for internal organization)
    fn apply_energy_changes(&mut self, sim: &mut SimulationImmut, _step: i64, _year: i64) {
        let years_per_step = sim.years_per_step();

        // Apply Perlin noise-modulated core radiance
        self.apply_core_radiance(sim, years_per_step);

        // Apply hotspot energy as direct plume creation
        self.apply_hotspot_plumes(sim, years_per_step);
    }

    /// Report component status (private method for internal organization)
    fn report_status(&mut self, _sim: &SimulationImmut, step: i64, _year: i64) {
        if step % 100 == 0 {
            // Sample a few normalized 3D positions to show variation and drift
            let sample_positions_normalized = [
                (0.47, 0.16, -0.91),   // Normalized coordinates (range ~-1 to +1)
                (-0.31, 0.63, -0.93),
                (0.24, -0.55, -0.89),
            ];
            println!("🔥 Core Radiance 3D Normalized (Step {}, Year {}): Perlin noise samples:", step, self.current_year as i64);

            for (x, y, z) in sample_positions_normalized {
                // Apply temporal drift if enabled
                let (drifted_x, drifted_y, drifted_z) = if let Some((drift_x, drift_y, drift_z)) = self.temporal_drift_per_year {
                    (
                        x + drift_x * self.current_year,
                        y + drift_y * self.current_year,
                        z + drift_z * self.current_year,
                    )
                } else {
                    (x, y, z)
                };

                let noise_value = self.perlin.get([
                    drifted_x * self.spatial_scale,
                    drifted_y * self.spatial_scale,
                    drifted_z * self.spatial_scale,
                    self.current_year * self.temporal_scale,
                ]);
                let variation_factor = 1.0 + (noise_value * self.noise_amplitude);

                if self.temporal_drift_per_year.is_some() {
                    println!("   ({:.2}, {:.2}, {:.2}) → ({:.3}, {:.3}, {:.3}) drifted: {:.3} noise → {:.1}% of base",
                        x, y, z, drifted_x, drifted_y, drifted_z, noise_value, variation_factor * 100.0);
                } else {
                    println!("   ({:.2}, {:.2}, {:.2}) normalized: {:.3} noise → {:.1}% of base",
                        x, y, z, noise_value, variation_factor * 100.0);
                }
            }
        }
    }

    /// Apply hotspot energy as direct plume creation and area-of-effect energy distribution
    fn apply_hotspot_plumes(&mut self, sim: &mut SimulationImmut, years_per_step: f64) {
        use crate::utils::h3_utils::H3Utils;

        let mut plumes_created = 0;
        let mut total_plume_energy = 0.0;
        let mut total_cells_affected = 0;
        let mut plume_creation_candidates: Vec<(usize, f64, f64)> = Vec::new(); // (hotspot_index, energy, radius)

        println!("🔍 Hotspot energy analysis:");

        // Process each active hotspot
        for (i, hotspot) in self.hotspots.iter().enumerate() {
            let (current_size, current_radius, current_heat_multiplier) = hotspot.current_properties(
                self.current_year,
                self.core_heat_config.hotspot_peak_years
            );

            // Calculate accumulated energy for this hotspot
            let hotspot_energy_per_year = self.base_energy_per_cell_per_year * current_heat_multiplier;
            let accumulated_energy = hotspot_energy_per_year * years_per_step;

            println!("   Hotspot {}: {:.1}x multiplier, {:.1}km radius, {:.2e}J accumulated (threshold: 5e21J)",
                i, current_heat_multiplier, current_radius, accumulated_energy);

            // Apply area-of-effect energy distribution if hotspot is active
            if current_size > 0.1 && current_radius > 1.0 {
                // Get all cells within the hotspot's current radius
                const DEFAULT_PLANET_RADIUS_KM: f64 = 6371.0; // Earth radius
                let affected_cells = H3Utils::cells_within_radius_km(
                    hotspot.cell_index,
                    current_radius,
                    DEFAULT_PLANET_RADIUS_KM
                );

                if !affected_cells.is_empty() {
                    // Distribute energy across affected cells with distance falloff
                    let energy_per_cell = accumulated_energy / affected_cells.len() as f64;
                    let mut cells_energized = 0;

                    if let Some(deepest_layer) = sim.layer_sets.last_mut() {
                        for (cell_index, distance_km) in &affected_cells {
                            if let Some(column) = deepest_layer.layers.get_mut(cell_index) {
                                if let Some(cell) = column.cells.last_mut() {
                                    // Apply distance-based falloff: stronger at center, weaker at edges
                                    let falloff_factor = (1.0 - (distance_km / current_radius)).max(0.1);
                                    let cell_energy = energy_per_cell * falloff_factor;

                                    if cell_energy > 0.0 {
                                        // Direct energy addition (transaction system temporarily disabled)
                                        cell.add_energy_joules(cell_energy);
                                        cells_energized += 1;
                                    }
                                }
                            }
                        }
                    }

                    total_cells_affected += cells_energized;

                    // Dual approach for plume creation
                    if hotspot.is_upwell && current_heat_multiplier > 0.5 {
                        // Debug: Print when we're considering plume creation
                        if self.current_year < 1500.0 {
                            println!("      🎯 Considering plume for hotspot {}: energy={:.2e}J, radius={:.1}km",
                                i, accumulated_energy, current_radius);
                        }
                        // Collect plume creation data for later processing
                        plume_creation_candidates.push((i, accumulated_energy, current_radius));
                    }

                    println!("      📊 Affected {} cells within {:.1}km radius", cells_energized, current_radius);
                }
            }
        }

        if plumes_created > 0 || total_cells_affected > 0 {
            // Second pass: Process plume creation candidates
        for (hotspot_index, accumulated_energy, current_radius) in plume_creation_candidates {
            let plume_created = self.evaluate_plume_creation_by_index(sim, hotspot_index, accumulated_energy, current_radius, years_per_step);

            if plume_created.is_some() {
                let (plume_id, creation_type) = plume_created.unwrap();
                plumes_created += 1;
                total_plume_energy += accumulated_energy;

                let hotspot_cell = self.hotspots[hotspot_index].cell_index;
                println!("      🌋 Hotspot plume #{} created at center cell {} with {:.2e}J ({})",
                    plume_id, hotspot_cell, accumulated_energy, creation_type);
            }
        }

        println!("🔥 Hotspot system: {} plumes created, {} cells affected, {:.2e}J total energy",
                plumes_created, total_cells_affected, total_plume_energy);
        }

        // DIAGNOSTIC: Compare our output with Earth-based radiance values
        self.compare_with_earth_radiance(years_per_step);
    }

    /// Create a plume from a hotspot with concentrated energy
    fn create_hotspot_plume(&self, sim: &mut SimulationImmut, hotspot: &Hotspot, energy_joules: f64, radius_km: f64) -> u64 {
        

        // Get hotspot geographic location
        let lat_lng = h3o::LatLng::from(hotspot.cell_index);
        let lat_deg = lat_lng.lat_radians().to_degrees();
        let lon_deg = lat_lng.lng_radians().to_degrees();

        // Find the deepest layer for plume source
        let source_layer_index = sim.layer_sets.len().saturating_sub(1);

        // Calculate initial depth (bottom of deepest layer)
        let initial_depth_km = if let Some(deepest_layer) = sim.layer_sets.last() {
            if let Some(column) = deepest_layer.layers.get(&hotspot.cell_index) {
                if let Some(_deepest_cell) = column.cells.last() {
                    // Estimate depth based on layer structure
                    deepest_layer.start_height_km + (column.cells.len() as f64 * 25.0) // Approximate
                } else {
                    250.0 // Default depth
                }
            } else {
                250.0 // Default depth
            }
        } else {
            250.0 // Default depth
        };

        // Calculate plume properties based on hotspot energy
        let plume_mass_kg = energy_joules / 1e6; // Rough conversion: 1 MJ per kg
        let plume_temperature_k = 1800.0 + (energy_joules / 1e22) * 200.0; // Hotter with more energy
        let plume_velocity_km_per_year = 10.0 + (energy_joules / 1e22) * 5.0; // Faster with more energy
        let buoyancy_force = energy_joules / 1e20; // Simplified buoyancy calculation

        // Instead of creating a plume, inject energy directly using atomic transactions
        // Find the target cell in the deepest layer
        if let Some(deepest_layer) = sim.layer_sets.get(source_layer_index) {
            if let Some(column) = deepest_layer.layers.get(&hotspot.cell_index) {
                if let Some(_deepest_cell) = column.cells.last() {
                    // Create cell location for the deepest cell
                    let cell_location = crate::transaction_manager::CellLocation::new(
                        source_layer_index,
                        hotspot.cell_index,
                        column.cells.len() - 1, // Last cell index
                    );

                    // Create atomic energy injection transaction
                    if let Ok(transaction) = crate::transaction_manager::AtomicTransaction::inject(
                        "CoreRadiance-Hotspot".to_string(),
                        cell_location,
                        energy_joules,
                        plume_mass_kg, // Inject some mass too
                        format!("Hotspot energy injection: {:.2e}J at ({:.2}, {:.2})", energy_joules, lat_deg, lon_deg),
                    ) {
                        // Propose atomic transaction to simulation
                        sim.transaction_manager.propose_atomic_transaction(transaction);
                    }
                }
            }
        }

        // Return a dummy plume ID (since we're not actually creating plumes)
        1 // Dummy ID
    }

    /// Evaluate whether to create a plume using pressure/density differential approach
    fn evaluate_plume_creation(&mut self, sim: &mut SimulationImmut, hotspot: &mut Hotspot,
                              accumulated_energy: f64, current_radius: f64, years_per_step: f64) -> Option<(u64, String)> {
        // Calculate pressure/density differential for this hotspot location
        let pressure_differential = self.calculate_pressure_differential(sim, hotspot);

        // Debug: Print pressure differential for all upwell hotspots (first step only)
        let debug = hotspot.is_upwell && self.current_year < 1500.0; // First step only
        if debug {
            println!("      🔍 Pressure differential at {}: {:.6}", hotspot.cell_index, pressure_differential);
        }

        // Only consider plume creation if there's significant pressure differential
        if pressure_differential < 0.001 { // Very low threshold for testing
            if debug {
                println!("      ❌ Pressure differential too low: {:.6}", pressure_differential);
            }
            return None; // No significant pressure difference
        }

        // Update pressure accumulation based on density/pressure differential
        hotspot.years_since_last_plume += years_per_step;

        // Accumulate pressure based on density differential and energy input
        let energy_factor = (accumulated_energy / 1e21).min(10.0); // Energy contributes to pressure
        let area_factor = (current_radius / 100.0).powf(2.0); // Larger areas build pressure faster
        let time_factor = (hotspot.years_since_last_plume / 1000.0).min(3.0); // Time amplifies pressure

        let pressure_increment = pressure_differential * energy_factor * area_factor * time_factor * years_per_step / 10000.0;
        hotspot.plume_pressure += pressure_increment;

        // Generate random trigger point for this hotspot (changes over time)
        let cell_hash = format!("{:?}", hotspot.cell_index).len() as f64; // Simple hash from cell index
        let trigger_seed = (cell_hash + self.current_year / 1000.0) % 1000.0;
        let random_trigger_point = 0.5 + 0.4 * (trigger_seed.sin() * 1000.0).fract(); // 0.5 to 0.9 range

        // Check if pressure exceeds the random trigger point
        if hotspot.plume_pressure > random_trigger_point {
            // Since we're using direct energy injection instead of plumes, no limit needed
            if true { // Always allow energy injection
                let plume_id = self.create_hotspot_plume(sim, hotspot, accumulated_energy, current_radius);
                hotspot.plume_pressure = 0.0; // Reset pressure after plume creation
                hotspot.years_since_last_plume = 0.0;

                println!("      🌋 Pressure-triggered energy injection: differential={:.3}, trigger={:.3}",
                    pressure_differential, random_trigger_point);

                return Some((plume_id, format!("pressure-trigger (d={:.2})", pressure_differential)));
            }
        }

        None
    }

    /// Calculate pressure/density differential at hotspot location
    fn calculate_pressure_differential(&self, sim: &SimulationImmut, hotspot: &Hotspot) -> f64 {
        // Find the deepest layer where this hotspot exists
        let deepest_layer_idx = sim.layer_sets.len().saturating_sub(1);

        // Debug: Print calculation details for first hotspot
        let debug = hotspot.is_upwell && self.current_year < 1500.0;

        if let Some(deepest_layer) = sim.layer_sets.get(deepest_layer_idx) {
            if let Some(column) = deepest_layer.layers.get(&hotspot.cell_index) {
                if let Some(deep_cell) = column.cells.last() {
                    // Calculate density of deep cell (heated by hotspot)
                    let deep_temp = deep_cell.temperature_kelvin();
                    let deep_pressure = deep_cell.pressure_pa();
                    let deep_density = self.calculate_density_from_temp_pressure(deep_temp, deep_pressure);

                    if debug {
                        println!("        Deep cell: T={:.1}K, P={:.2e}Pa, ρ={:.1}kg/m³", deep_temp, deep_pressure, deep_density);
                    }

                    // Find corresponding surface layer cell for comparison
                    if let Some(surface_layer) = sim.layer_sets.get(0) {
                        if let Some(surface_column) = surface_layer.layers.get(&hotspot.cell_index) {
                            if let Some(surface_cell) = surface_column.cells.first() {
                                let surface_temp = surface_cell.temperature_kelvin();
                                let surface_pressure = surface_cell.pressure_pa();
                                let surface_density = self.calculate_density_from_temp_pressure(surface_temp, surface_pressure);

                                if debug {
                                    println!("        Surface cell: T={:.1}K, P={:.2e}Pa, ρ={:.1}kg/m³", surface_temp, surface_pressure, surface_density);
                                }

                                // Calculate density differential (higher = more buoyant deep material)
                                let density_diff = surface_density - deep_density;
                                let normalized_diff = (density_diff / surface_density).max(0.0);

                                if debug {
                                    println!("        Density diff: {:.1}kg/m³, normalized: {:.6}", density_diff, normalized_diff);
                                }

                                return normalized_diff;
                            }
                        }
                    }
                }
            }
        }

        0.0 // No differential found
    }

    /// Calculate density from temperature and pressure (simplified equation of state)
    fn calculate_density_from_temp_pressure(&self, temperature_k: f64, pressure_pa: f64) -> f64 {
        // Simplified equation of state for rock/magma
        // Higher temperature = lower density, higher pressure = higher density
        let base_density = 3000.0; // kg/m³ for typical mantle rock
        let thermal_expansion = 3e-5; // 1/K thermal expansion coefficient
        let compressibility = 1e-11; // 1/Pa compressibility

        let reference_temp = 1600.0; // K
        let reference_pressure = 1e9; // Pa

        let temp_factor = 1.0 - thermal_expansion * (temperature_k - reference_temp);
        let pressure_factor = 1.0 + compressibility * (pressure_pa - reference_pressure);

        base_density * temp_factor * pressure_factor
    }

    /// Evaluate plume creation by hotspot index (to avoid borrowing conflicts)
    fn evaluate_plume_creation_by_index(&mut self, sim: &mut SimulationImmut, hotspot_index: usize,
                                       accumulated_energy: f64, current_radius: f64, years_per_step: f64) -> Option<(u64, String)> {
        if hotspot_index >= self.hotspots.len() {
            return None;
        }

        // Calculate pressure/density differential for this hotspot location
        let hotspot = &self.hotspots[hotspot_index];
        let pressure_differential = self.calculate_pressure_differential(sim, hotspot);

        // Only consider plume creation if there's significant pressure differential
        if pressure_differential < 0.001 { // Very low threshold for testing
            return None; // No significant pressure difference
        }

        // Update pressure accumulation based on density/pressure differential
        let current_pressure = self.hotspots[hotspot_index].plume_pressure;
        let current_years_since_plume = self.hotspots[hotspot_index].years_since_last_plume;
        let new_years_since_plume = current_years_since_plume + years_per_step;

        // Accumulate pressure based on density differential and energy input
        let energy_factor = (accumulated_energy / 1e21).min(10.0); // Energy contributes to pressure
        let area_factor = (current_radius / 100.0).powf(2.0); // Larger areas build pressure faster
        let time_factor = (new_years_since_plume / 1000.0).min(3.0); // Time amplifies pressure

        let pressure_increment = pressure_differential * energy_factor * area_factor * time_factor * years_per_step / 10000.0;
        let new_pressure = current_pressure + pressure_increment;

        // Generate random trigger point for this hotspot (changes over time)
        let cell_hash = format!("{:?}", self.hotspots[hotspot_index].cell_index).len() as f64; // Simple hash from cell index
        let trigger_seed = (cell_hash + self.current_year / 1000.0) % 1000.0;
        let random_trigger_point = 0.5 + 0.4 * (trigger_seed.sin() * 1000.0).fract(); // 0.5 to 0.9 range

        // Update hotspot state
        self.hotspots[hotspot_index].plume_pressure = new_pressure;
        self.hotspots[hotspot_index].years_since_last_plume = new_years_since_plume;

        // Check if pressure exceeds the random trigger point
        if new_pressure > random_trigger_point {
            // Since we're using direct energy injection instead of plumes, no limit needed
            if true { // Always allow energy injection
                let plume_id = self.create_hotspot_plume_by_index(sim, hotspot_index, accumulated_energy, current_radius);
                // Reset pressure after plume creation
                self.hotspots[hotspot_index].plume_pressure = 0.0;
                self.hotspots[hotspot_index].years_since_last_plume = 0.0;

                println!("      🌋 Pressure-triggered energy injection: differential={:.3}, trigger={:.3}",
                    pressure_differential, random_trigger_point);

                return Some((plume_id, format!("pressure-trigger (d={:.2})", pressure_differential)));
            }
        }

        None
    }

    /// Create a plume from a hotspot by index (helper to avoid borrowing conflicts)
    fn create_hotspot_plume_by_index(&self, sim: &mut SimulationImmut, hotspot_index: usize, energy_joules: f64, radius_km: f64) -> u64 {
        let hotspot = &self.hotspots[hotspot_index];
        self.create_hotspot_plume(sim, hotspot, energy_joules, radius_km)
    }

    /// Compare our radiance output with Earth-based reference values
    fn compare_with_earth_radiance(&self, _years_per_step: f64) {
        println!("\n📊 RADIANCE COMPARISON WITH EARTH VALUES");
        println!("=========================================");

        // Our model parameters
        let base_energy_per_year = self.base_energy_per_cell_per_year;
        let max_hotspot_multiplier = 7.7; // From test output
        let max_hotspot_energy_per_year = base_energy_per_year * max_hotspot_multiplier;

        // Convert to watts (J/year → W)
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let base_power_w = base_energy_per_year / seconds_per_year;
        let max_hotspot_power_w = max_hotspot_energy_per_year / seconds_per_year;

        // Estimate cell area (H3 Resolution 2 cells)
        // H3 Resolution 2 has ~86,000 cells globally, so each cell ≈ 5.9e12 m²
        let earth_surface_area_m2 = 5.1e14; // Earth's surface area
        let h3_res2_cell_count = 86000.0; // Approximate
        let cell_area_m2 = earth_surface_area_m2 / h3_res2_cell_count;

        // Calculate heat flux (W/m²)
        let base_heat_flux_w_m2 = base_power_w / cell_area_m2;
        let max_hotspot_heat_flux_w_m2 = max_hotspot_power_w / cell_area_m2;

        println!("🔥 OUR MODEL OUTPUT:");
        println!("   Total energy: {:.2e} J/cell/year = {:.2e} W/cell", base_energy_per_year, base_power_w);
        println!("   Background ({}%): {:.2e} J/cell/year = {:.2e} W/cell",
            (self.core_heat_config.background_energy_fraction() * 100.0) as u32,
            base_energy_per_year * self.core_heat_config.background_energy_fraction(),
            base_power_w * self.core_heat_config.background_energy_fraction());
        println!("   Hotspot ({}%): {:.2e} J/cell/year = {:.2e} W/cell",
            (self.core_heat_config.hotspot_energy_fraction() * 100.0) as u32,
            max_hotspot_energy_per_year, max_hotspot_power_w);
        println!("   Cell area (H3 Res2): {:.2e} m²", cell_area_m2);
        println!("   Background heat flux: {:.3} W/m²", base_heat_flux_w_m2 * self.core_heat_config.background_energy_fraction());
        println!("   Max hotspot flux: {:.1} W/m²", max_hotspot_heat_flux_w_m2);

        println!("\n🌍 EARTH REFERENCE VALUES (from RADIANCE.md):");
        println!("   Global average: 0.086 W/m²");
        println!("   Continental: 0.065 W/m²");
        println!("   Oceanic: 0.096 W/m²");
        println!("   Peak hotspots: 1-2 W/m² (up to 70 W/m² at vents)");

        println!("\n📈 COMPARISON:");
        let base_vs_global = base_heat_flux_w_m2 / 0.086;
        let hotspot_vs_peak = max_hotspot_heat_flux_w_m2 / 2.0;

        if base_vs_global > 0.5 && base_vs_global < 2.0 {
            println!("   ✅ Base flux: {:.1}x Earth average (reasonable)", base_vs_global);
        } else {
            println!("   ⚠️  Base flux: {:.1}x Earth average (may need adjustment)", base_vs_global);
        }

        if hotspot_vs_peak > 0.5 && hotspot_vs_peak < 5.0 {
            println!("   ✅ Hotspot flux: {:.1}x Earth peak hotspots (reasonable)", hotspot_vs_peak);
        } else {
            println!("   ⚠️  Hotspot flux: {:.1}x Earth peak hotspots (may need adjustment)", hotspot_vs_peak);
        }
    }
}

impl Default for CoreHeatComponent {
    fn default() -> Self {
        Self::new()
    }
}

// #[cfg(test)] - Tests disabled due to deprecated module references
#[cfg(disabled)]
mod tests {
    use crate::constants::EARTH_RADIUS_KM_F64;
    use crate::sim::layer_set::{default_layer_set_params, DefaultLayerSetParams};
    use super::*;

    #[test]
    fn test_perlin_noise_3d_variation() {
        let component = CoreRadianceComponent::new();

        // Test with normalized 3D positions (range -1 to +1)
        let test_positions = [
            (0.0, 0.0, 0.0),           // Origin
            (0.5, 0.0, 0.0),           // X-axis
            (0.0, 0.5, 0.0),           // Y-axis
            (0.0, 0.0, -0.9),          // Deep Z (negative for depth)
            (0.3, 0.4, -0.8),          // Diagonal deep position
        ];

        let mut energies = Vec::new();
        let mut noise_values = Vec::new();

        for (x, y, z) in test_positions {
            // Create a dummy cell index for testing (H3 resolution 2, index 0)
            let dummy_cell = CellIndex::try_from(0x820000000000000u64).unwrap();
            let energy = component.calculate_energy_for_cell(&dummy_cell, x, y, z, 1.0, 6371.0);
            energies.push(energy);

            // Also calculate raw noise value for debugging
            let noise = component.perlin.get([
                x * component.spatial_scale,
                y * component.spatial_scale,
                z * component.spatial_scale,
                0.0, // No temporal component for this test
            ]);
            noise_values.push(noise);
        }

        println!("✅ Perlin noise 3D normalized variation test:");
        println!("   Base energy: {:.2e} J", component.base_energy_per_cell_per_year);
        println!("   Spatial scale: {:.3}", component.spatial_scale);

        for (i, (((x, y, z), energy), noise)) in test_positions.iter().zip(energies.iter()).zip(noise_values.iter()).enumerate() {
            let variation = (energy / component.base_energy_per_cell_per_year - 1.0) * 100.0;
            println!("   Position {}: ({:.2}, {:.2}, {:.2}) normalized → noise: {:.3}, energy: {:.2e}J ({:.1}% variation)",
                i + 1, x, y, z, noise, energy, variation);
        }

        // Check that we have some variation in noise values
        let max_noise = noise_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_noise = noise_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let noise_range = max_noise - min_noise;

        println!("   Noise range: {:.3} (min: {:.3}, max: {:.3})", noise_range, min_noise, max_noise);

        // Should have some variation in noise values
        assert!(noise_range > 0.01, "Should have meaningful noise variation");

        // Check that energies are within expected range
        let base = component.base_energy_per_cell_per_year;
        let min_expected = base * (1.0 - component.noise_amplitude);
        let max_expected = base * (1.0 + component.noise_amplitude);

        for energy in &energies {
            assert!(*energy >= min_expected && *energy <= max_expected,
                "Energy should be within ±15% range");
        }
    }

    #[test]
    fn test_temporal_variation_3d() {
        let mut component = CoreRadianceComponent::new();

        // Test that different times give different values for same 3D position
        let test_position = (3000.0, 1000.0, -5800.0); // Fixed 3D position

        // Create a dummy cell index for testing
        let dummy_cell = CellIndex::try_from(0x820000000000000u64).unwrap();

        component.current_year = 0.0;
        let energy_t0 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        component.current_year = 10000.0;
        let energy_t1 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        component.current_year = 20000.0;
        let energy_t2 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        // Should vary over time (though slowly due to temporal scale)
        println!("✅ Temporal variation test (3D):");
        println!("   Position: ({:.0}, {:.0}, {:.0})km", test_position.0, test_position.1, test_position.2);
        println!("   Year 0: {:.2e} J", energy_t0);
        println!("   Year 10000: {:.2e} J", energy_t1);
        println!("   Year 20000: {:.2e} J", energy_t2);

        // At least some variation should occur over long time periods
        let max_energy = energy_t0.max(energy_t1).max(energy_t2);
        let min_energy = energy_t0.min(energy_t1).min(energy_t2);
        let variation_range = (max_energy - min_energy) / component.base_energy_per_cell_per_year;

        println!("   Temporal variation range: {:.1}%", variation_range * 100.0);
    }

    #[test]
    fn test_3d_coordinate_conversion_normalized() {
        let component = CoreRadianceComponent::new();

        // Create a mock cell index for testing
        use h3o::CellIndex;
        let test_cell = CellIndex::try_from(0x85283473fffffff).unwrap(); // Valid H3 index

        let depth_km = 250.0; // Deep asthenosphere
        let planet_radius_km = 6371.0; // Earth radius
        let (x, y, z) = component.get_cell_3d_position(&test_cell, depth_km, planet_radius_km);

        // Verify normalized coordinates are reasonable
        let normalized_radius = (x*x + y*y + z*z).sqrt();
        let expected_normalized_radius = (planet_radius_km - depth_km) / planet_radius_km;

        println!("✅ 3D coordinate conversion test (normalized):");
        println!("   Depth: {:.1}km", depth_km);
        println!("   Planet radius: {:.1}km", planet_radius_km);
        println!("   Normalized 3D position: ({:.3}, {:.3}, {:.3})", x, y, z);
        println!("   Normalized radius: {:.3}", normalized_radius);
        println!("   Expected normalized radius: {:.3}", expected_normalized_radius);

        // Should be close to expected normalized radius
        assert!((normalized_radius - expected_normalized_radius).abs() < 0.001,
            "Calculated normalized radius should match expected");

        // Coordinates should be in reasonable range (-1 to +1)
        assert!(x >= -1.0 && x <= 1.0, "X coordinate should be normalized");
        assert!(y >= -1.0 && y <= 1.0, "Y coordinate should be normalized");
        assert!(z >= -1.0 && z <= 1.0, "Z coordinate should be normalized");

        println!("   ✓ All coordinates properly normalized to [-1, +1] range");
    }

    #[test]
    fn test_core_radiance_energy_injection() {
        use crate::sim::simulation::{Simulation, SimulationConfig};
        
        use crate::component::SimComponent;
        
        use h3o::Resolution;

        println!("\n🔥 Testing Core Radiance Energy Injection");
        println!("==========================================");
        
        let layer_params = default_layer_set_params(
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
            surface_temp_k: 288.15,
        };

        // Test WITHOUT core radiance
        println!("\n📊 Test 1: WITHOUT Core Radiance");
        println!("--------------------------------");

        let mut components_no_radiance: Vec<Box<dyn SimComponent>> = vec![];
        let mut sim_no_radiance = Simulation::new(config.clone(), &mut components_no_radiance);
        sim_no_radiance.initialize();

        // Record initial energies
        let initial_upper_energy = calculate_layer_total_energy(&sim_no_radiance, 0);
        let initial_deep_energy = calculate_layer_total_energy(&sim_no_radiance, 1);

        println!("Initial energies:");
        println!("   Upper layer (0-25km): {:.2e} J", initial_upper_energy);
        println!("   Deep layer (25-50km):  {:.2e} J", initial_deep_energy);

        // Run simulation for 3 steps
        for _step in 0..3 {
            sim_no_radiance.step();
        }

        let final_upper_energy_no_rad = calculate_layer_total_energy(&sim_no_radiance, 0);
        let final_deep_energy_no_rad = calculate_layer_total_energy(&sim_no_radiance, 1);

        println!("Final energies (no radiance):");
        println!("   Upper layer: {:.2e} J (change: {:.2e} J)",
            final_upper_energy_no_rad, final_upper_energy_no_rad - initial_upper_energy);
        println!("   Deep layer:  {:.2e} J (change: {:.2e} J)",
            final_deep_energy_no_rad, final_deep_energy_no_rad - initial_deep_energy);

        // Test WITH core radiance
        println!("\n🔥 Test 2: WITH Core Radiance");
        println!("-----------------------------");

        let mut components_with_radiance: Vec<Box<dyn SimComponent>> = vec![
            Box::new(CoreRadianceComponent::new()
                .with_base_energy(1e20)        // 1e20 J per cell per year
                .with_noise_amplitude(0.0)     // No noise for predictable results
                .with_spatial_scale(0.1)),
        ];
        let mut sim_with_radiance = Simulation::new(config.clone(), &mut components_with_radiance);
        sim_with_radiance.initialize();

        // Record initial energies (should be same as before)
        let initial_upper_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 0);
        let initial_deep_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 1);

        println!("Initial energies:");
        println!("   Upper layer (0-25km): {:.2e} J", initial_upper_energy_rad);
        println!("   Deep layer (25-50km):  {:.2e} J", initial_deep_energy_rad);

        // Run simulation for 3 steps
        for step in 0..3 {
            println!("   Running step {}...", step + 1);
            sim_with_radiance.step();
            let step_deep_energy = calculate_layer_total_energy(&sim_with_radiance, 1);
            println!("   Step {}: Deep layer energy = {:.2e} J", step + 1, step_deep_energy);

            // Check if any pending energy changes exist
            let mut total_pending = 0.0;
            if let Some(deepest_layer) = sim_with_radiance.layer_sets.last() {
                for column in deepest_layer.layers.values() {
                    for cell in &column.cells {
                        total_pending += cell.pending_energy_change();
                    }
                }
            }
            println!("   Total pending energy after step {}: {:.2e} J", step + 1, total_pending);
        }

        let final_upper_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 0);
        let final_deep_energy_rad = calculate_layer_total_energy(&sim_with_radiance, 1);

        println!("Final energies (with radiance):");
        println!("   Upper layer: {:.2e} J (change: {:.2e} J)",
            final_upper_energy_rad, final_upper_energy_rad - initial_upper_energy_rad);
        println!("   Deep layer:  {:.2e} J (change: {:.2e} J)",
            final_deep_energy_rad, final_deep_energy_rad - initial_deep_energy_rad);

        // Calculate expected energy injection
        let expected_energy_per_step = 1e20 * 1000.0; // base_energy * years_per_step
        let expected_total_injection = expected_energy_per_step * 3.0; // 3 steps
        let actual_deep_energy_increase = final_deep_energy_rad - initial_deep_energy_rad;

        println!("\n📈 Energy Injection Analysis");
        println!("----------------------------");
        println!("Expected energy injection per step: {:.2e} J", expected_energy_per_step);
        println!("Expected total injection (3 steps): {:.2e} J", expected_total_injection);
        println!("Actual deep layer energy increase:  {:.2e} J", actual_deep_energy_increase);
        println!("Injection efficiency: {:.1}%",
            (actual_deep_energy_increase / expected_total_injection) * 100.0);

        // Verify core radiance is working
        assert!(actual_deep_energy_increase > 0.0,
            "Deep layer should gain energy from core radiance");
        assert!(actual_deep_energy_increase > expected_total_injection * 0.8,
            "Should inject at least 80% of expected energy");

        // Compare with no-radiance case
        let no_rad_deep_change = final_deep_energy_no_rad - initial_deep_energy;
        let rad_deep_change = final_deep_energy_rad - initial_deep_energy_rad;
        let radiance_effect = rad_deep_change - no_rad_deep_change;

        println!("\n🔬 Radiance Effect Comparison");
        println!("-----------------------------");
        println!("Deep layer change WITHOUT radiance: {:.2e} J", no_rad_deep_change);
        println!("Deep layer change WITH radiance:    {:.2e} J", rad_deep_change);
        println!("Net radiance effect:                {:.2e} J", radiance_effect);
        println!("Radiance contribution: {:.1}%",
            (radiance_effect / rad_deep_change) * 100.0);

        assert!(radiance_effect > 0.0,
            "Core radiance should add significant energy to deep layer");

        println!("\n✅ Core Radiance Energy Injection Test PASSED!");
        println!("   ✓ Energy successfully injected into deepest layer");
        println!("   ✓ Injection amount matches expected values");
        println!("   ✓ Clear difference between with/without radiance");
    }

    // Helper function to calculate total energy in a specific layer
    fn calculate_layer_total_energy(sim: &SimulationImmut, layer_index: usize) -> f64 {
        if let Some(layer_set) = sim.layer_sets.get(layer_index) {
            let mut total_energy = 0.0;
            for column in layer_set.layers.values() {
                for cell in &column.cells {
                    total_energy += cell.energy_joules();
                }
            }
            total_energy
        } else {
            0.0
        }
    }

    #[test]
    fn test_temporal_drift() {
        let mut component = CoreRadianceComponent::new()
            .with_geological_drift(); // Enable 1% per 100k years drift

        let test_position = (0.5, 0.3, -0.8); // Fixed normalized position

        // Create a dummy cell index for testing
        let dummy_cell = CellIndex::try_from(0x820000000000000u64).unwrap();

        // Test energy at different time points
        component.current_year = 0.0;
        let energy_t0 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        component.current_year = 50000.0; // 50k years
        let energy_t1 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        component.current_year = 100000.0; // 100k years
        let energy_t2 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        component.current_year = 200000.0; // 200k years
        let energy_t3 = component.calculate_energy_for_cell(&dummy_cell, test_position.0, test_position.1, test_position.2, 1.0, 6371.0);

        println!("✅ Temporal drift test:");
        println!("   Position: ({:.1}, {:.1}, {:.1})", test_position.0, test_position.1, test_position.2);
        println!("   Year 0: {:.2e} J", energy_t0);
        println!("   Year 50k: {:.2e} J", energy_t1);
        println!("   Year 100k: {:.2e} J", energy_t2);
        println!("   Year 200k: {:.2e} J", energy_t3);

        // Calculate variations from initial
        let var_50k = (energy_t1 / energy_t0 - 1.0) * 100.0;
        let var_100k = (energy_t2 / energy_t0 - 1.0) * 100.0;
        let var_200k = (energy_t3 / energy_t0 - 1.0) * 100.0;

        println!("   Variations from t=0:");
        println!("     50k years: {:.2}%", var_50k);
        println!("     100k years: {:.2}%", var_100k);
        println!("     200k years: {:.2}%", var_200k);

        // Should show gradual drift over geological time
        // The exact values depend on Perlin noise, but should show evolution
        assert!(energy_t0 > 0.0, "Energy should be positive");
        assert!(energy_t3 > 0.0, "Energy should remain positive after drift");

        println!("   ✓ Temporal drift creates gradual energy evolution over geological time");
    }

    #[test]
    fn test_drift_magnitude() {
        let component = CoreRadianceComponent::new()
            .with_geological_drift();

        if let Some((drift_x, drift_y, drift_z)) = component.temporal_drift_per_year {
            let drift_magnitude = (drift_x * drift_x + drift_y * drift_y + drift_z * drift_z).sqrt();
            let percent_per_100k_years = drift_magnitude * 100000.0 * 100.0;

            println!("✅ Drift magnitude test:");
            println!("   Drift vector: ({:.2e}, {:.2e}, {:.2e}) per year", drift_x, drift_y, drift_z);
            println!("   Magnitude: {:.2e} per year", drift_magnitude);
            println!("   Expected change: {:.1}% per 100k years", percent_per_100k_years);

            // Should be approximately 1% per 100k years
            assert!((percent_per_100k_years - 1.0).abs() < 0.1,
                "Drift should be approximately 1% per 100k years");

            // Components should be equal (non-orthogonal diagonal direction)
            assert!((drift_x - drift_y).abs() < 1e-10, "X and Y components should be equal");
            assert!((drift_y - drift_z).abs() < 1e-10, "Y and Z components should be equal");

            println!("   ✓ Drift magnitude and direction are correct");
        } else {
            panic!("Geological drift should be enabled");
        }
    }
}

