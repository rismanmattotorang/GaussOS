use std::arch::x86_64::*;
use crate::pathfinding::Point;

/// SIMD-accelerated distance calculator
pub struct SimdDistanceCalculator {
    chunk_size: usize,
}

impl SimdDistanceCalculator {
    pub fn new() -> Self {
        Self { chunk_size: 4 }
    }

    /// Calculate distances between points using SIMD instructions
    #[target_feature(enable = "avx2")]
    pub unsafe fn calculate_distances_simd(&self, points: &[Point], target: &Point) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points.len());
        
        for chunk in points.chunks(4) {
            let mut xs = _mm256_setzero_pd();
            let mut ys = _mm256_setzero_pd();
            let mut zs = _mm256_setzero_pd();
            
            // Load coordinates
            for (i, point) in chunk.iter().enumerate() {
                let x = point.coords.x;
                let y = point.coords.y;
                let z = point.coords.z;
                xs = _mm256_insertf64x2(xs, _mm_set_pd(x, 0.0), i as i32);
                ys = _mm256_insertf64x2(ys, _mm_set_pd(y, 0.0), i as i32);
                zs = _mm256_insertf64x2(zs, _mm_set_pd(z, 0.0), i as i32);
            }
            
            // Calculate differences
            let tx = target.coords.x;
            let ty = target.coords.y;
            let tz = target.coords.z;
            let dx = _mm256_sub_pd(xs, _mm256_set1_pd(tx));
            let dy = _mm256_sub_pd(ys, _mm256_set1_pd(ty));
            let dz = _mm256_sub_pd(zs, _mm256_set1_pd(tz));
            
            // Calculate squared distances
            let dx2 = _mm256_mul_pd(dx, dx);
            let dy2 = _mm256_mul_pd(dy, dy);
            let dz2 = _mm256_mul_pd(dz, dz);
            
            // Sum components
            let sum = _mm256_add_pd(_mm256_add_pd(dx2, dy2), dz2);
            
            // Extract distances
            let mut result = [0.0; 4];
            _mm256_storeu_pd(result.as_mut_ptr(), _mm256_sqrt_pd(sum));
            
            distances.extend_from_slice(&result[..chunk.len()]);
        }
        
        distances
    }

    /// Calculate distances between points using scalar operations (fallback)
    pub fn calculate_distances_scalar(&self, points: &[Point], target: &Point) -> Vec<f64> {
        points.iter()
            .map(|p| (p.coords - target.coords).norm())
            .collect()
    }

    /// Calculate distances using the best available method
    pub fn calculate_distances(&self, points: &[Point], target: &Point) -> Vec<f64> {
        if is_x86_feature_detected!("avx2") {
            unsafe { self.calculate_distances_simd(points, target) }
        } else {
            self.calculate_distances_scalar(points, target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector3;

    #[test]
    fn test_distance_calculation() {
        let calc = SimdDistanceCalculator::new();
        let points = vec![
            Point::from(Vector3::new(0.0, 0.0, 0.0)),
            Point::from(Vector3::new(1.0, 0.0, 0.0)),
            Point::from(Vector3::new(0.0, 1.0, 0.0)),
            Point::from(Vector3::new(1.0, 1.0, 1.0)),
        ];
        let target = Point::from(Vector3::new(0.0, 0.0, 0.0));

        let scalar_distances = calc.calculate_distances_scalar(&points, &target);
        let simd_distances = calc.calculate_distances(&points, &target);

        assert_eq!(scalar_distances.len(), simd_distances.len());
        for (scalar, simd) in scalar_distances.iter().zip(simd_distances.iter()) {
            assert!((scalar - simd).abs() < 1e-10);
        }
    }
} 