use crate::prelude::{Epoch, Frame, Orbit};

#[derive(Debug, Copy, Clone)]
pub struct QcReferencePosition {
    /// Ecef coordinates in meters
    ecef_m: (f64, f64, f64),
}

impl QcReferencePosition {
    /// Define new [QcReferencePosition] from ECEF coordinates
    pub fn new(ecef_m: (f64, f64, f64)) -> Self {
        Self { ecef_m }
    }

    /// Create a new [QcReferencePosition] from an [Orbit]
    #[cfg(feature = "navigation")]
    pub fn from_orbit(orbit: &Orbit) -> Self {
        let posvel_m = orbit.to_cartesian_pos_vel() * 1.0E3;
        let ecef_m = (posvel_m[0], posvel_m[1], posvel_m[2]);
        Self { ecef_m }
    }

    /// Express this [QcReferencePosition] as an [Orbit]
    #[cfg(feature = "navigation")]
    pub fn to_orbit(&self, t: Epoch, frame: Frame) -> Orbit {
        let (x_km, y_km, z_km) = (
            self.ecef_m.0 * 1.0E-3,
            self.ecef_m.1 * 1.0E-3,
            self.ecef_m.2 * 1.0E-3,
        );

        Orbit::from_position(x_km, y_km, z_km, t, frame)
    }
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
                            (format!("{:.6}", self.ecef_m.0))
                        }
                        td {
                            (format!("{:.6}", self.ecef_m.1))
                        }
                        td {
                            (format!("{:.6}", self.ecef_m.2))
                        }
                    }
                }
            }
        }
    }
}
