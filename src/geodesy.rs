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

/// ETRF2000 to ITRF2014 transformation (inverse of EUREF TN-1 "ITRF2014 to ETRF2000",
/// EPSG:8405, 14 parameters expressed at epoch 2000.0), first order.
///
/// European stations coordinates (RINEX headers, national networks) are often expressed
/// in ETRF2000 (attached to the Eurasian plate), while absolute navigation solutions
/// obtained with IGS precise products are expressed in ITRF2014 (IGS14):
/// both frames differ by ~0.75 m in northern Europe in 2020.
///
/// ## Input
/// - etrf_ecef_m: ETRF2000 ECEF coordinates (m)
/// - year: decimal year of the coordinates (e.g. 2020.48)
pub fn etrf2000_to_itrf2014(etrf_ecef_m: (f64, f64, f64), year: f64) -> (f64, f64, f64) {
    const MAS: f64 = std::f64::consts::PI / 180.0 / 3600.0 / 1000.0;
    let dt = year - 2000.0;

    let t = (
        (53.7 + 0.1 * dt) * 1.0E-3,
        (51.2 + 0.1 * dt) * 1.0E-3,
        (-55.1 - 1.9 * dt) * 1.0E-3,
    );

    let d = (1.02 + 0.11 * dt) * 1.0E-9;
    let r1 = (0.891 + 0.081 * dt) * MAS;
    let r2 = (5.390 + 0.490 * dt) * MAS;
    let r3 = (-8.712 - 0.792 * dt) * MAS;

    // X_etrf = X_itrf + T + M.X_itrf, inverted to first order (M ~ 1E-7)
    let x = (
        etrf_ecef_m.0 - t.0,
        etrf_ecef_m.1 - t.1,
        etrf_ecef_m.2 - t.2,
    );

    let mx = (
        d * x.0 - r3 * x.1 + r2 * x.2,
        r3 * x.0 + d * x.1 - r1 * x.2,
        -r2 * x.0 + r1 * x.1 + d * x.2,
    );

    (x.0 - mx.0, x.1 - mx.1, x.2 - mx.2)
}

#[cfg(test)]
mod frames_test {
    use super::*;

    #[test]
    fn etrf2000_itrf2014_denmark() {
        // ESBC00DNK marker (ETRF2000), mid 2020
        let etrf = (3582105.2910, 532589.7313, 5232754.8054);
        let itrf = etrf2000_to_itrf2014(etrf, 2020.48);

        let delta = (itrf.0 - etrf.0, itrf.1 - etrf.1, itrf.2 - etrf.2);
        let (e, n, u) = ecef_to_enu_vector(etrf, delta);

        // ~0.5 m East, ~0.56 m North, a few cm vertically
        assert!((e - 0.5).abs() < 0.1, "east: {}", e);
        assert!((n - 0.56).abs() < 0.1, "north: {}", n);
        assert!(u.abs() < 0.1, "up: {}", u);
    }
}
