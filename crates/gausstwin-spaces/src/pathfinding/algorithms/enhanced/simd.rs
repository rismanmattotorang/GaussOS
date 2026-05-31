use nalgebra::Vector3;
use std::arch::x86_64::*;

#[derive(Clone, Copy)]
pub struct SimdPoint {
    x: f64,
    y: f64,
    z: f64,
}

impl From<Vector3<f64>> for SimdPoint {
    fn from(v: Vector3<f64>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<SimdPoint> for Vector3<f64> {
    fn from(p: SimdPoint) -> Self {
        Vector3::new(p.x, p.y, p.z)
    }
}

pub struct SimdDistanceCalculator;

impl SimdDistanceCalculator {
    #[target_feature(enable = "avx2")]
    pub unsafe fn calculate_distances_simd(points: &[SimdPoint], query: SimdPoint) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points.len());
        let chunks = points.chunks_exact(4);
        let remainder = chunks.remainder();

        // Process 4 points at a time using AVX2
        let query_x = _mm256_set1_pd(query.x);
        let query_y = _mm256_set1_pd(query.y);
        let query_z = _mm256_set1_pd(query.z);

        for chunk in chunks {
            // Load 4 points into SIMD registers
            let mut xs = _mm256_undefined_pd();
            let mut ys = _mm256_undefined_pd();
            let mut zs = _mm256_undefined_pd();

            for (i, point) in chunk.iter().enumerate() {
                xs = _mm256_insertf64x2(xs, _mm_set_pd(point.x, 0.0), i as i32);
                ys = _mm256_insertf64x2(ys, _mm_set_pd(point.y, 0.0), i as i32);
                zs = _mm256_insertf64x2(zs, _mm_set_pd(point.z, 0.0), i as i32);
            }

            // Calculate squared differences
            let dx = _mm256_sub_pd(xs, query_x);
            let dy = _mm256_sub_pd(ys, query_y);
            let dz = _mm256_sub_pd(zs, query_z);

            // Calculate squared distances
            let dx2 = _mm256_mul_pd(dx, dx);
            let dy2 = _mm256_mul_pd(dy, dy);
            let dz2 = _mm256_mul_pd(dz, dz);

            // Sum components and take square root
            let sum = _mm256_add_pd(_mm256_add_pd(dx2, dy2), dz2);
            let dist = _mm256_sqrt_pd(sum);

            // Store results
            let mut result = [0.0; 4];
            _mm256_storeu_pd(result.as_mut_ptr(), dist);
            distances.extend_from_slice(&result);
        }

        // Process remaining points
        for point in remainder {
            let dx = point.x - query.x;
            let dy = point.y - query.y;
            let dz = point.z - query.z;
            distances.push((dx * dx + dy * dy + dz * dz).sqrt());
        }

        distances
    }

    pub fn calculate_distances(points: &[SimdPoint], query: SimdPoint) -> Vec<f64> {
        if is_x86_feature_detected!("avx2") {
            unsafe { Self::calculate_distances_simd(points, query) }
        } else {
            Self::calculate_distances_scalar(points, query)
        }
    }

    fn calculate_distances_scalar(points: &[SimdPoint], query: SimdPoint) -> Vec<f64> {
        points
            .iter()
            .map(|p| {
                let dx = p.x - query.x;
                let dy = p.y - query.y;
                let dz = p.z - query.z;
                (dx * dx + dy * dy + dz * dz).sqrt()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_simd_distances() {
        let mut rng = rand::thread_rng();
        let points: Vec<SimdPoint> = (0..100)
            .map(|_| {
                SimdPoint {
                    x: rng.gen_range(-10.0..10.0),
                    y: rng.gen_range(-10.0..10.0),
                    z: rng.gen_range(-10.0..10.0),
                }
            })
            .collect();

        let query = SimdPoint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        let simd_distances = SimdDistanceCalculator::calculate_distances(&points, query);
        let scalar_distances = SimdDistanceCalculator::calculate_distances_scalar(&points, query);

        assert_eq!(simd_distances.len(), scalar_distances.len());
        for (simd, scalar) in simd_distances.iter().zip(scalar_distances.iter()) {
            assert!((simd - scalar).abs() < 1e-10);
        }
    }
} 