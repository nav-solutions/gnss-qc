//! Geodesy helpers (WGS84)

const WGS84_A_M: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;

/// ECEF (meters) to geodetic (latitude, longitude in radians, altitude above ellipsoid in m)
pub fn ecef_to_geodetic(ecef_m: (f64, f64, f64)) -> (f64, f64, f64) {
    let (x, y, z) = ecef_m;
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let p = (x * x + y * y).sqrt();
    let lon = y.atan2(x);

    let mut lat = z.atan2(p * (1.0 - e2));
    let mut alt = 0.0;

    for _ in 0..10 {
        let (sin_lat, cos_lat) = lat.sin_cos();
        let n = WGS84_A_M / (1.0 - e2 * sin_lat * sin_lat).sqrt();
        alt = p / cos_lat - n;
        lat = z.atan2(p * (1.0 - e2 * n / (n + alt)));
    }

    (lat, lon, alt)
}

/// Rotates a local (east, north, up) vector (meters) at `reference` (ECEF meters)
/// into an ECEF vector (meters).
pub fn enu_to_ecef_vector(
    reference_ecef_m: (f64, f64, f64),
    enu: (f64, f64, f64),
) -> (f64, f64, f64) {
    let (lat, lon, _) = ecef_to_geodetic(reference_ecef_m);
    let (sin_lat, cos_lat) = lat.sin_cos();
    let (sin_lon, cos_lon) = lon.sin_cos();
    let (e, n, u) = enu;

    (
        -sin_lon * e - sin_lat * cos_lon * n + cos_lat * cos_lon * u,
        cos_lon * e - sin_lat * sin_lon * n + cos_lat * sin_lon * u,
        cos_lat * n + sin_lat * u,
    )
}

/// Expresses an ECEF error vector (meters) in the local (east, north, up) frame at `reference`.
pub fn ecef_to_enu_vector(
    reference_ecef_m: (f64, f64, f64),
    delta: (f64, f64, f64),
) -> (f64, f64, f64) {
    let (lat, lon, _) = ecef_to_geodetic(reference_ecef_m);
    let (sin_lat, cos_lat) = lat.sin_cos();
    let (sin_lon, cos_lon) = lon.sin_cos();
    let (dx, dy, dz) = delta;

    (
        -sin_lon * dx + cos_lon * dy,
        -sin_lat * cos_lon * dx - sin_lat * sin_lon * dy + cos_lat * dz,
        cos_lat * cos_lon * dx + cos_lat * sin_lon * dy + sin_lat * dz,
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn enu_roundtrip() {
        let reference = (3582105.2910, 532589.7313, 5232754.8054);
        let (lat, lon, alt) = ecef_to_geodetic(reference);
        assert!((lat.to_degrees() - 55.49).abs() < 0.1);
        assert!((lon.to_degrees() - 8.46).abs() < 0.1);
        assert!(alt.abs() < 200.0);

        let enu = (1.0, 2.0, 3.0);
        let ecef = enu_to_ecef_vector(reference, enu);
        let back = ecef_to_enu_vector(reference, ecef);
        assert!((back.0 - 1.0).abs() < 1e-9);
        assert!((back.1 - 2.0).abs() < 1e-9);
        assert!((back.2 - 3.0).abs() < 1e-9);
    }
}
