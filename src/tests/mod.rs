mod timeshift;

pub mod toolkit;

mod indexing;
mod rinex;
mod sp3;

use log::LevelFilter;
use std::sync::Once;

use crate::prelude::{Almanac, Epoch, Frame, Orbit, EARTH_J2000};

static INIT: Once = Once::new();

pub fn init_logger() {
    INIT.call_once(|| {
        env_logger::builder()
            .is_test(true)
            .filter_level(LevelFilter::Debug)
            .init();
    });
}

pub fn test_almanac() -> Almanac {
    Almanac::until_2035().unwrap_or_else(|e| panic!("Failed to build test Almanac: {}", e))
}

pub fn test_earth_frame() -> Frame {
    Almanac::until_2035()
        .unwrap_or_else(|e| panic!("Failed to build test Almanac: {}", e))
        .frame_from_uid(EARTH_J2000)
        .unwrap_or_else(|e| panic!("Failed to build test EARTH-J2000 frame: {}", e))
}

pub const REFERENCE_COORDS_ECEF_M: (f64, f64, f64) = (3628427.9118, 562059.0936, 5197872.2150);

/// Express [REFERENCE_COORDS_ECEF_M] as ANISE [Orbit].
pub fn test_reference_orbit(t: Epoch, frame: Frame) -> Orbit {
    Orbit::from_position(
        REFERENCE_COORDS_ECEF_M.0 / 1.0E3,
        REFERENCE_COORDS_ECEF_M.1 / 1.0E3,
        REFERENCE_COORDS_ECEF_M.2 / 1.0E3,
        t,
        frame,
    )
}
