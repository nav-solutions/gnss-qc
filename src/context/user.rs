//! User preferences
use thiserror::Error;

use crate::prelude::{TimeSeries, TimeScale};

use serde::{Deserialize, Serialize};

#[cfg(feature = "navigation")]
use crate::prelude::Vector3;

#[cfg(feature = "html")]
use maud::{html, Markup, Render};

/// Customization Errors
#[derive(Debug, Clone, Error)]
pub enum Error {
}

use std::fmt::Display;
use std::str::FromStr;

#[derive(Copy, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QcUserPreferences {
    /// Prefered [TimeScale].
    /// All data points and solutions will be expressed in given [TimeScale].
    pub timescale: TimeScale,

    /// User position (on navigation feature only) expressed as
    /// ECEF coordinates in meters.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    #[serde(default)]
    pub ground_pos_m_ecef: Option<Vector3>,

    /// Possible temporal window, used to crop the temporal axis
    /// in future analysis.
    pub time_window: Option<TimeSeries>,
}

impl QcUserPreferences {
    /// Upgrade these [QcUserPreferences] with an updated
    /// ground position, expressed as ECEF coordinates in meters.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn with_ground_position_ecef_m(mut self, ecef_m: Vector3) -> Self {
        self.ground_pos_m_ecef = Some(ecef_m);
        self
    }
    
    /// Upgrade these [QcUserPreferences] with an updated
    /// ground position, expressed as geodetic coordinates:
    /// (latitude (ddeg), longitude (ddeg), altitude (m))
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn with_ground_position_geo_degrees(mut self, geo_degrees: Vector3) -> Result<Self, Error> {
        let ecef_m = geo_degrees.to_position()?;
        self.ground_pos_m_ecef = Some(ecef_m);
        self
    }
}

impl Render for QcConfig {
    fn render(&self) -> Markup {
        html! {
            tr {
                td {
                    "Timescale"
                }
                td {
                    (self.timescale.to_string())
                }
            }
            tr {
                td {
                    "Ground position (ECEF)" // TODO
                }
            }
        }
    }
}
