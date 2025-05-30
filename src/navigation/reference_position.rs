use crate::prelude::{Epoch, Frame, Orbit};

use anise::{
    astro::PhysicsResult,
    constants::frames::{EARTH_ITRF93, EARTH_J2000, IAU_EARTH_FRAME},
    math::Vector6,
};

#[derive(Debug, Copy, Clone)]
pub struct QcReferencePosition {
    orbit: Orbit,
}

impl std::fmt::Display for QcReferencePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Reference Position - {}", self.orbit)
    }
}

impl QcReferencePosition {
    // Define new [QcReferencePosition] from ECEF coordinates
    pub fn new(ecef_m: (f64, f64, f64), epoch: Epoch) -> Self {
        // const GM_M3_S2: f64 = 3.986004418E14;

        let pos_vel = Vector6::new(
            ecef_m.0 * 1e-3,
            ecef_m.1 * 1e-3,
            ecef_m.2 * 1e-3,
            0.0,
            0.0,
            0.0,
        );

        let orbit = Orbit::from_cartesian_pos_vel(pos_vel, epoch, EARTH_J2000);

        Self { orbit }
    }

    /// Converts this [QcReferencePosition] as Geodetic coordinates
    /// (latitude in degrees, longitude in degrees, altitude above mean sea level in km).
    pub fn to_earth_geodetic_degrees_km(&self) -> PhysicsResult<(f64, f64, f64)> {
        self.orbit.latlongalt()
    }

    /// Create a new [QcReferencePosition] from an [Orbit]
    pub fn from_orbit(orbit: &Orbit) -> Self {
        Self { orbit: *orbit }
    }

    // /// Express this [QcReferencePosition] as an [Orbit]
    // #[cfg(feature = "navigation")]
    // pub fn to_orbit(&self, t: Epoch, frame: Frame) -> Orbit {
    //     let (x_km, y_km, z_km) = (
    //         self.ecef_m.0 * 1.0E-3,
    //         self.ecef_m.1 * 1.0E-3,
    //         self.ecef_m.2 * 1.0E-3,
    //     );

    //     Orbit::from_position(x_km, y_km, z_km, t, frame)
    // }
}

#[cfg(feature = "html")]
use maud::{html, Markup, Render};

#[cfg(feature = "html")]
impl Render for QcReferencePosition {
    fn render(&self) -> Markup {
        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "ECEF Coordinates"
                        }
                        td {
                            "x (km)"
                        }
                        td {
                            "y (km)"
                        }
                        td {
                            "z (km)"
                        }
                    }
                    tr {
                        td {

                        }
                        td {
                            (format!("{:.6}", self.orbit.radius_km.x))
                        }
                        td {
                            (format!("{:.6}", self.orbit.radius_km.y))
                        }
                        td {
                            (format!("{:.6}", self.orbit.radius_km.z))
                        }
                    }
                }
            }
        }
    }
}
