use serde::{Deserialize, Serialize};
use thiserror::Error;

use maud::{html, Markup, Render};

use crate::{
    config::{orbit::QcOrbitPreference, report::QcReportType},
    context::QcIdentifier,
};

pub mod report;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub mod orbit;

/// [Error]s during configuration process.
#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid report type")]
    InvalidReportType,
    #[cfg(feature = "navigation")]
    #[error("invalid orbit preference")]
    InvalidOrbitPreference,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum QcIndexingMethod {
    /// Let the framework index data by itself.
    /// Correctly defined products will be correctly indexed.
    /// Products for which no classification could be determined, will wind up
    /// as "unclassified".
    #[default]
    Auto,

    /// Select a prefered indexing method. The framework will apply it where possible.
    Manual(QcIdentifier),
}

/// [QcConfig] allows to define a custom reference point,
/// or dictate the behavior of the framework in a few specific steps.
/// For example, which orbit source should be prefered when orbital projection is needed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QcConfig {
    /// Select a prefered Indexing method.
    pub indexing: QcIndexingMethod,

    #[serde(default)]
    pub report: QcReportType,

    /// [OrbitPreference] applie to the navigation process.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub orbit_preference: QcOrbitPreference,

    /// Reference coordinates, defined externally, that should
    /// apply to the receiver. Usually, one would use this if they
    /// have better knowledge of the position.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    #[serde(default)]
    pub user_rx_ecef: Option<(f64, f64, f64)>,
}

impl QcConfig {
    /// Update the [QcReportType] preference.
    pub fn set_report_type(&mut self, report_type: QcReportType) {
        self.report = report_type;
    }

    /// Update the user defined Orbit source preference.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn set_orbit_preference(&mut self, preference: QcOrbitPreference) {
        self.orbit_preference = preference;
    }

    /// Update the user defined RX position ECEF coordinates
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn set_reference_rx_ecef_coordinates(&mut self, ecef_m: (f64, f64, f64)) {
        self.user_rx_ecef = Some(ecef_m);
    }

    /// Build a [QcConfig] with updated [QcReportType] preference.
    pub fn with_report_type(&self, report_type: QcReportType) -> Self {
        let mut s = self.clone();
        s.report = report_type;
        s
    }

    /// Build a [QcConfig] with updated [QcOrbitPreference].
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn with_orbit_preference(&self, preference: QcOrbitPreference) -> Self {
        let mut s = self.clone();
        s.orbit_preference = preference;
        s
    }

    /// Build a [QcConfig] with updated user defined RX position as ECEF coordinates.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn with_user_rx_position_ecef(&self, ecef_m: (f64, f64, f64)) -> Self {
        let mut s = self.clone();
        s.user_rx_ecef = Some(ecef_m);
        s
    }
}

impl Render for QcConfig {
    fn render(&self) -> Markup {
        html! {
            tr {
                td {
                    "Reporting"
                }
                td {
                    (self.report.to_string())
                }
            }
            tr {
                td {
                    "Orbit preference"
                }
                td {
                    (self.orbit_preference.to_string())
                }
            }
        }
    }
}
