use glam::Vec3;
use h3o::{CellIndex, EARTH_RADIUS_KM, LatLng, Resolution};
use rand::Rng;
use rand::seq::SliceRandom;

/// Cell data within a radius with comprehensive information for radiance calculations
#[derive(Debug, Clone)]
pub struct CellWithinRadius {
    pub cell_id: CellIndex,
    pub distance_km: f64,
    pub point_3d: Vec3,
    pub rad_percent: f64, // Distance as percentage of radius (0.0 = center, 1.0 = edge)
}

pub struct PointSampler {
    primary_cells: Vec<CellIndex>,
    current_index: usize,
    planet_radius_km: i32,
}

impl PointSampler {
    /// Create a new sampler for one plate
    pub fn new(planet_radius_km: i32) -> Self {
        PointSampler {
            primary_cells: CellIndex::base_cells().collect(),
            current_index: 0,
            planet_radius_km,
        }
    }

    fn ll_tuple(cell: &CellIndex) -> (f64, f64) {
        let ll = LatLng::from(*cell);
        (ll.lat_radians(), ll.lng_radians())
    }

    fn random_child(parent: &CellIndex) -> CellIndex {
        let mut rng = rand::rng();

        let children: Vec<CellIndex> = parent.children(Resolution::Three).collect();
        let idx = rng.random_range(0..children.len());
        children[idx]
    }

    fn cell_to_unit(cell: &CellIndex) -> Vec3 {
        let (lat, lng) = PointSampler::ll_tuple(&cell);
        let (cos_lat, sin_lat) = (lat.cos(), lat.sin());
        let (cos_lng, sin_lng) = (lng.cos(), lng.sin());
        // Convert spherical coordinates to Cartesian coordinates on unit sphere
        Vec3::new(
            (cos_lat * cos_lng) as f32,
            (cos_lat * sin_lng) as f32,
            sin_lat as f32,
        )
    }

    /*
    returns a "semi-random" point on the planet.
    each time it is called it returns a random h3 location in a different
    primary cell region.

    The points are "spaced out" and not perfectly overalpping
    which is _not_ a trait of true randomness but prevents plates from
    being perfectly on top of each other.

    In theory they will be as far apart from each other as a primary radii
    but in truth they will be from (level 3 radii ... (2 x primary radii - level 3 radii))
    apart from each other.
     */
    pub fn random_point_on_planet(&mut self) -> Vec3 {
        if self.current_index == 0 {
            let mut rng = rand::rng();
            self.primary_cells.shuffle(&mut rng);
        }

        let primary_cell = self.primary_cells[self.current_index];
        let cell = PointSampler::random_child(&primary_cell);

        let unit = PointSampler::cell_to_unit(&cell);
        let point = unit * self.planet_radius_km as f32;

        // 4) Advance the index, wrapping around
        self.current_index = (self.current_index + 1) % self.primary_cells.len();

        point
    }
}

pub struct H3Utils;

impl H3Utils {
    pub fn iter_cells_with_base(
        resolution: Resolution,
    ) -> impl Iterator<Item = (CellIndex, CellIndex)> {
        CellIndex::base_cells()
            .into_iter()
            .flat_map(move |base_cell| {
                base_cell
                    .children(resolution)
                    .into_iter()
                    .map(move |child| (child, base_cell))
            })
    }

    /// Get all neighboring cells for a given cell index
    pub fn neighbors_for(cell_index: CellIndex) -> Vec<CellIndex> {
        // Get all neighboring cells
        let neighbors: Vec<CellIndex> = cell_index
            .grid_disk::<Vec<_>>(1)
            .into_iter()
            .filter(|&neighbor| neighbor != cell_index)
            .collect();
        neighbors
    }

    /// Get all cells within a specific radius (in km) from a center cell
    /// Uses grid_disk with calculated ring distance and filters by actual distance
    /// IMPORTANT: H3 is designed for Earth coordinates, so we scale the radius by Earth/planet ratio
    pub fn cells_within_radius_km(
        center_cell: CellIndex,
        radius_km: f64,
        planet_radius_km: f64,
    ) -> Vec<(CellIndex, f64)> {
        // Scale the radius to Earth coordinates since H3 is Earth-based
        let earth_radius_scale = EARTH_RADIUS_KM / planet_radius_km;
        let earth_scaled_radius_km = radius_km * earth_radius_scale;

        // Calculate approximate ring distance needed for H3 grid_disk (using Earth scaling)
        let resolution = center_cell.resolution();
        let earth_cell_edge_length_km =
            Self::estimate_cell_edge_length_km(resolution, EARTH_RADIUS_KM);

        // Calculate how many rings we need to cover the Earth-scaled radius (with some buffer)
        let rings_needed =
            ((earth_scaled_radius_km * 1.25) / earth_cell_edge_length_km).ceil() as u32;

        // Get all cells within the calculated ring distance
        let candidate_cells: Vec<CellIndex> = center_cell
            .grid_disk::<Vec<_>>(rings_needed)
            .into_iter()
            .filter(|&cell| cell != center_cell)
            .collect();

        // Filter by actual distance (using planet radius) and return with distances
        let mut result = Vec::new();
        for cell in candidate_cells {
            let distance_m = Self::cell_distance_m(center_cell, cell, planet_radius_km);
            let distance_km = distance_m / 1000.0;

            if distance_km <= radius_km {
                result.push((cell, distance_km));
            }
        }

        result
    }

    /// Estimate the edge length of an H3 cell at a given resolution
    /// This is an approximation based on H3 geometry for Earth coordinates
    /// For non-Earth planets, the caller should scale the result appropriately
    fn estimate_cell_edge_length_km(resolution: h3o::Resolution, planet_radius_km: f64) -> f64 {
        // H3 edge lengths scale by approximately sqrt(7) between resolutions
        // Base edge length at resolution 0 is approximately 1107.712 km on Earth
        let earth_base_edge_km = 1107.712;
        let scale_factor = (planet_radius_km / EARTH_RADIUS_KM);
        let resolution_scale = 7.0_f64.powf(-(resolution as u8 as f64) / 2.0);

        earth_base_edge_km * scale_factor * resolution_scale
    }

    /// Get all cells within a radius with a percentage buffer (e.g., 125% = 1.25)
    /// Returns cells within the buffered radius along with their actual distances
    /// Properly handles planetary radius scaling for non-Earth planets
    pub fn cells_within_radius_with_buffer(
        center_cell: CellIndex,
        base_radius_km: f64,
        buffer_percentage: f64,
        planet_radius_km: f64,
    ) -> Vec<(CellIndex, f64)> {
        let buffered_radius_km = base_radius_km * buffer_percentage;
        Self::cells_within_radius_km(center_cell, buffered_radius_km, planet_radius_km)
    }

    /// Get all cells within a radius with comprehensive data for radiance calculations
    /// Returns structured data including cell_id, distance, 3D point, and radius percentage
    pub fn cells_within_radius_from(
        center_cell: CellIndex,
        radius_km: f64,
        planet_radius_km: f64,
    ) -> Vec<CellWithinRadius> {
        // Scale the radius to Earth coordinates since H3 is Earth-based
        let earth_radius_scale = EARTH_RADIUS_KM / planet_radius_km;
        let earth_scaled_radius_km = radius_km * earth_radius_scale;

        // Calculate approximate ring distance needed for H3 grid_disk (using Earth scaling)
        let resolution = center_cell.resolution();
        let earth_cell_edge_length_km =
            Self::estimate_cell_edge_length_km(resolution, EARTH_RADIUS_KM);

        // Calculate how many rings we need to cover the Earth-scaled radius (with some buffer)
        let rings_needed =
            ((earth_scaled_radius_km * 1.25) / earth_cell_edge_length_km).ceil() as u32;

        // Get all cells within the calculated ring distance (including center cell)
        let candidate_cells: Vec<CellIndex> = center_cell
            .grid_disk::<Vec<_>>(rings_needed)
            .into_iter()
            .collect();

        // Get center cell 3D point for distance calculations
        let center_point_3d = Self::cell_to_3d_point(center_cell, planet_radius_km);

        // Filter by actual distance and build comprehensive result
        let mut result = Vec::new();
        for cell in candidate_cells {
            let distance_m = Self::cell_distance_m(center_cell, cell, planet_radius_km);
            let distance_km = distance_m / 1000.0;

            if distance_km <= radius_km {
                let point_3d = Self::cell_to_3d_point(cell, planet_radius_km);
                let rad_percent = if radius_km > 0.0 {
                    distance_km / radius_km
                } else {
                    0.0
                };

                result.push(CellWithinRadius {
                    cell_id: cell,
                    distance_km,
                    point_3d,
                    rad_percent,
                });
            }
        }

        result
    }

    /// Calculate the great circle distance between two H3 cells in meters
    /// Uses the haversine formula for accurate distance calculation
    pub fn cell_distance_m(cell_a: CellIndex, cell_b: CellIndex, planet_radius_km: f64) -> f64 {
        let lat_lng_a = LatLng::from(cell_a);
        let lat_lng_b = LatLng::from(cell_b);

        let lat1 = lat_lng_a.lat_radians();
        let lon1 = lat_lng_a.lng_radians();
        let lat2 = lat_lng_b.lat_radians();
        let lon2 = lat_lng_b.lng_radians();

        // Haversine formula
        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;

        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        // Distance in meters
        planet_radius_km * c * 1000.0
    }

    /// Calculate the great circle distance between two H3 cells in meters (Earth radius)
    /// Convenience method using Earth's radius
    pub fn cell_distance_earth_m(cell_a: CellIndex, cell_b: CellIndex) -> f64 {
        Self::cell_distance_m(cell_a, cell_b, EARTH_RADIUS_KM)
    }

    /// Get the area of a cell at the given resolution for a planet with the given radius
    /// Scales the Earth-based area by the ratio of planet radius to Earth radius squared
    /// Uses cached Earth areas to avoid recomputation
    pub fn cell_area(res: Resolution, planet_radius_km: f64) -> f64 {
        let earth_areas = EARTH_CELL_AREAS_CACHE.get_or_init(|| generate_earth_cell_areas());
        let earth_area_km2 = earth_areas[res as usize];
        let scale_factor = (planet_radius_km / crate::constants::EARTH_RADIUS_KM as f64).powi(2);
        earth_area_km2 * scale_factor
    }

    /// Get the total number of H3 cells at a given resolution
    pub fn cell_count_at_resolution(res: Resolution) -> u64 {
        cell_count_at_resolution(res)
    }

    /// Convert a cell index to a 3D point on the planet surface
    /// Returns a Vec3 representing the position in 3D space with the given planet radius
    pub fn cell_to_3d_point(cell_index: CellIndex, planet_radius_km: f64) -> Vec3 {
        let lat_lng = LatLng::from(cell_index);
        let lat_rad = lat_lng.lat_radians();
        let lng_rad = lat_lng.lng_radians();

        // Convert spherical coordinates to Cartesian coordinates
        let (cos_lat, sin_lat) = (lat_rad.cos(), lat_rad.sin());
        let (cos_lng, sin_lng) = (lng_rad.cos(), lng_rad.sin());

        Vec3::new(
            (planet_radius_km * cos_lat * cos_lng) as f32,
            (planet_radius_km * cos_lat * sin_lng) as f32,
            (planet_radius_km * sin_lat) as f32,
        )
    }

    /// Convert a cell index to a normalized 3D point on the unit sphere
    /// Returns a Vec3 representing the position on a unit sphere (radius = 1.0)
    /// Useful for Perlin noise sampling and other spherical calculations
    pub fn cell_to_unit_sphere_point(cell_index: CellIndex) -> Vec3 {
        let lat_lng = LatLng::from(cell_index);
        let lat_rad = lat_lng.lat_radians();
        let lng_rad = lat_lng.lng_radians();

        // Convert spherical coordinates to Cartesian coordinates on unit sphere
        let (cos_lat, sin_lat) = (lat_rad.cos(), lat_rad.sin());
        let (cos_lng, sin_lng) = (lng_rad.cos(), lng_rad.sin());

        Vec3::new(
            (cos_lat * cos_lng) as f32,
            (cos_lat * sin_lng) as f32,
            sin_lat as f32,
        )
        .normalize() // Ensure it's exactly on unit sphere
    }
}

/// Get the total number of H3 cells at a given resolution
pub fn cell_count_at_resolution(res: Resolution) -> u64 {
    // H3 formula: 2 + 120 * 7^(res-1) for res > 0, and 122 for res = 0
    match res {
        Resolution::Zero => 122,
        _ => {
            let res_num = res as u8;
            2 + 120 * 7_u64.pow((res_num) as u32)
        }
    }
}

pub fn area_km2_at_resolution(res: Resolution, planet_radius_km2: f64) -> f64 {
    let base_cell = CellIndex::base_cells().next().unwrap();
    let first_child = base_cell.children(res).next().unwrap();
    first_child.area_km2() * (planet_radius_km2 / EARTH_RADIUS_KM).powf(2.0)
}

// Function to generate the areas array at runtime
pub fn generate_earth_cell_areas() -> [f64; 16] {
    let mut areas = [0.0; 16];
    for i in 0..16 {
        areas[i] = area_km2_at_resolution(Resolution::try_from(i as u8).unwrap(), EARTH_RADIUS_KM);
    }
    areas
}

// Lazy static for the areas array
use std::sync::OnceLock;
static EARTH_CELL_AREAS_CACHE: OnceLock<[f64; 16]> = OnceLock::new();

pub fn earth_cell_areas() -> &'static [f64; 16] {
    EARTH_CELL_AREAS_CACHE.get_or_init(|| generate_earth_cell_areas())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::EARTH_RADIUS_KM;
    use glam::Vec3;

    #[test]
    fn test_point_sampler_creation() {
        let radius_km = EARTH_RADIUS_KM;
        let sampler = PointSampler::new(radius_km);
        assert_eq!(sampler.planet_radius_km, radius_km);
        assert_eq!(sampler.current_index, 0);
        assert_eq!(sampler.primary_cells.len(), 122); // h3 base cells count
    }

    #[test]
    fn test_random_point_on_planet_radius() {
        let radius_km = 6371;
        let mut sampler = PointSampler::new(radius_km);

        let point = sampler.random_point_on_planet();
        let length = point.length();

        // The length should be close to radius_km (allow small floating error)
        let radius_f64 = radius_km as f32;
        let epsilon = 1.0; // 1 km tolerance
        assert!(
            (length - radius_f64).abs() < epsilon,
            "Point length {} not close to radius {}",
            length,
            radius_f64
        );
    }

    #[test]
    fn test_random_points_are_different() {
        let radius_km = 6371;
        let mut sampler = PointSampler::new(radius_km);

        let point1 = sampler.random_point_on_planet();
        let point2 = sampler.random_point_on_planet();

        // Points should not be exactly equal
        assert_ne!(point1, point2, "Two consecutive points should not be equal");
    }

    #[test]
    fn test_points_are_on_sphere_surface() {
        let radius_km = 6371;
        let mut sampler = PointSampler::new(radius_km);

        for _ in 0..10 {
            let point = sampler.random_point_on_planet();
            let length = point.length();
            let radius_f64 = radius_km as f32;
            let epsilon = 1.0;
            assert!(
                (length - radius_f64).abs() < epsilon,
                "Point length {} not close to radius {}",
                length,
                radius_f64
            );
        }
    }

    #[test]
    fn test_closest_neighbor_distances_statistics() {
        let radius_km = 6371;
        let mut sampler = PointSampler::new(radius_km);

        let num_points = 30;
        let mut points: Vec<Vec3> = Vec::with_capacity(num_points);

        for _ in 0..num_points {
            points.push(sampler.random_point_on_planet());
        }

        // For each point, find the closest neighbor distance
        let mut closest_distances = Vec::with_capacity(num_points);

        for (i, p) in points.iter().enumerate() {
            let mut min_dist = f32::MAX;
            for (j, q) in points.iter().enumerate() {
                if i == j {
                    continue;
                }
                let dist = (*p - *q).length();
                if dist < min_dist {
                    min_dist = dist;
                }
            }
            closest_distances.push(min_dist);
        }

        // Compute statistics: min, average, std deviation
        let min_distance = closest_distances.iter().cloned().fold(f32::MAX, f32::min);
        let sum: f32 = closest_distances.iter().sum();
        let avg_distance = sum / closest_distances.len() as f32;

        let variance = closest_distances
            .iter()
            .map(|d| {
                let diff = d - avg_distance as f32;
                diff * diff
            })
            .sum::<f32>()
            / closest_distances.len() as f32;
        let std_dev = variance.sqrt();

        println!(
            "Closest neighbor distances (km): min = {:.2}, avg = {:.2}, std dev = {:.2}",
            min_distance, avg_distance, std_dev
        );

        // Basic sanity checks
        assert!(
            min_distance > 100.0,
            "Minimum closest neighbor distance should be > 100"
        );
        assert!(
            avg_distance > 1000.0,
            "Average closest neighbor distance should be > 1000"
        );
    }

    #[test]
    fn test_cells_within_radius_from() {
        use h3o::Resolution;

        // Test with a known cell at resolution 3
        let center_cell = h3o::CellIndex::base_cells()
            .next()
            .unwrap()
            .children(Resolution::Three)
            .next()
            .unwrap();

        let radius_km = 100.0;
        let planet_radius_km = EARTH_RADIUS_KM;

        // Get cells within radius
        let cells_within =
            H3Utils::cells_within_radius_from(center_cell, radius_km, planet_radius_km as f64);

        // Should have some cells within the radius
        assert!(!cells_within.is_empty(), "Should find cells within radius");

        // All cells should be within the specified radius
        for cell_data in &cells_within {
            assert!(
                cell_data.distance_km <= radius_km,
                "Cell distance {:.2}km should be <= radius {:.2}km",
                cell_data.distance_km,
                radius_km
            );

            assert!(
                cell_data.rad_percent >= 0.0 && cell_data.rad_percent <= 1.0,
                "Radius percentage {:.3} should be between 0.0 and 1.0",
                cell_data.rad_percent
            );

            // Verify rad_percent calculation
            let expected_percent = cell_data.distance_km / radius_km;
            let percent_diff = (cell_data.rad_percent - expected_percent).abs();
            assert!(
                percent_diff < 1e-10,
                "Radius percentage calculation error: expected {:.6}, got {:.6}",
                expected_percent,
                cell_data.rad_percent
            );

            // Verify 3D point is on the planet surface
            let point_magnitude = (cell_data.point_3d.x.powi(2)
                + cell_data.point_3d.y.powi(2)
                + cell_data.point_3d.z.powi(2))
            .sqrt();
            let expected_magnitude = planet_radius_km as f32;
            assert!(
                (point_magnitude - expected_magnitude).abs() < 1.0,
                "3D point magnitude {:.1} should be close to planet radius {:.1}",
                point_magnitude,
                expected_magnitude
            );
        }

        // Test edge cases
        let cells_zero_radius =
            H3Utils::cells_within_radius_from(center_cell, 0.0, planet_radius_km as f64);
        assert!(
            cells_zero_radius.is_empty(),
            "Zero radius should return no cells"
        );

        println!("✅ cells_within_radius_from test passed");
        println!("   Center cell: {:?}", center_cell);
        println!("   Radius: {:.1}km", radius_km);
        println!("   Cells found: {}", cells_within.len());
        if !cells_within.is_empty() {
            let min_distance = cells_within
                .iter()
                .map(|c| c.distance_km)
                .fold(f64::INFINITY, f64::min);
            let max_distance = cells_within
                .iter()
                .map(|c| c.distance_km)
                .fold(0.0, f64::max);
            println!(
                "   Distance range: {:.2}km - {:.2}km",
                min_distance, max_distance
            );
        }
       // assert!(std_dev >= 300.0, "Standard deviation should be >= 300");
    }

    #[test]
    fn test_cell_area_scaling() {
        // Test that cell area scales correctly with planet radius
        let earth_radius = EARTH_RADIUS_KM as f64;
        let mars_radius = 3390.0; // Mars radius in km

        let res = Resolution::Five;

        // Area for Earth should match the precomputed value
        let earth_area = H3Utils::cell_area(res, earth_radius);
        let expected_earth_area = earth_cell_areas()[res as usize];
        assert!((earth_area - expected_earth_area).abs() < 1e-6);

        // Area for Mars should be scaled by (mars_radius/earth_radius)^2
        let mars_area = H3Utils::cell_area(res, mars_radius);
        let expected_scale = (mars_radius / earth_radius).powi(2);
        let expected_mars_area = expected_earth_area * expected_scale;
        assert!((mars_area - expected_mars_area).abs() < 1e-6);

        println!("Earth area at res {}: {:.6} km²", res as u8, earth_area);
        println!("Mars area at res {}: {:.6} km²", res as u8, mars_area);
        println!("Scale factor: {:.6}", expected_scale);
    }

    #[test]
    fn test_cell_count_at_resolution() {
        // Test known H3 cell counts for different resolutions
        assert_eq!(H3Utils::cell_count_at_resolution(Resolution::Zero), 122);
        assert_eq!(H3Utils::cell_count_at_resolution(Resolution::One), 842);
        assert_eq!(H3Utils::cell_count_at_resolution(Resolution::Two), 5882);
        assert_eq!(H3Utils::cell_count_at_resolution(Resolution::Three), 41162);

        // Test the standalone function as well
        assert_eq!(cell_count_at_resolution(Resolution::Zero), 122);
        assert_eq!(cell_count_at_resolution(Resolution::One), 842);

        println!("Cell counts:");
        for res in 0..=5 {
            let resolution = Resolution::try_from(res).unwrap();
            let count = H3Utils::cell_count_at_resolution(resolution);
            println!("Resolution {}: {} cells", res, count);
        }
    }

    #[test]
    fn test_cell_to_3d_point() {
        use crate::constants::EARTH_RADIUS_KM;

        // Test with a known cell
        let first_cell = CellIndex::base_cells().next().unwrap();
        let earth_radius = EARTH_RADIUS_KM as f64;

        // Get 3D point for Earth
        let point_earth = H3Utils::cell_to_3d_point(first_cell, earth_radius);

        // The magnitude should be approximately equal to Earth's radius
        let magnitude =
            (point_earth.x.powi(2) + point_earth.y.powi(2) + point_earth.z.powi(2)).sqrt();
        assert!(
            (magnitude - earth_radius as f32).abs() < 1.0,
            "Point magnitude {} should be close to Earth radius {}",
            magnitude,
            earth_radius
        );

        // Test with Mars radius
        let mars_radius = 3390.0;
        let point_mars = H3Utils::cell_to_3d_point(first_cell, mars_radius);
        let magnitude_mars =
            (point_mars.x.powi(2) + point_mars.y.powi(2) + point_mars.z.powi(2)).sqrt();
        assert!(
            (magnitude_mars - mars_radius as f32).abs() < 1.0,
            "Point magnitude {} should be close to Mars radius {}",
            magnitude_mars,
            mars_radius
        );

        // The direction should be the same, only magnitude should differ
        let earth_unit = point_earth.normalize();
        let mars_unit = point_mars.normalize();
        let dot_product = earth_unit.dot(mars_unit);
        assert!(
            dot_product > 0.999,
            "Unit vectors should be nearly identical, dot product: {}",
            dot_product
        );

        println!("Earth point: {:?} (magnitude: {})", point_earth, magnitude);
        println!(
            "Mars point: {:?} (magnitude: {})",
            point_mars, magnitude_mars
        );
    }
}
