use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "html")]
use maud::{html, Markup, Render};

mod indexing;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
mod orbit;

pub use crate::config::indexing::QcPreferedIndexing;

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
pub use orbit::QcOrbitPreference;

/// [Error]s during configuration process.
#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("invalid report type")]
    InvalidReportType,

    #[cfg(feature = "navigation")]
    #[error("invalid orbit preference")]
    InvalidOrbitPreference,
}

/// [QcConfig] allows to define a custom reference point,
/// or dictate the behavior of the framework in a few specific steps.
/// For example, which orbit source should be prefered when orbital projection is needed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QcConfig {
    /// Select a prefered Indexing method. This only applies to
    /// RINEX files, other files are indexed by data publishers.
    /// Correct data indexing is mandatory for differential navigation (RTK).
    #[serde(default)]
    pub indexing: QcPreferedIndexing,

    /// [OrbitPreference] applied to the navigation process.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    #[serde(default)]
    pub orbit_preference: QcOrbitPreference,
}

impl QcConfig {
    /// Update your indexing preference
    pub fn set_prefered_indexing(&mut self, indexing: QcPreferedIndexing) {
        self.indexing = indexing;
    }

    /// Update the user defined Orbit source preference.
    #[cfg(feature = "navigation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
    pub fn set_orbit_preference(&mut self, preference: QcOrbitPreference) {
        self.orbit_preference = preference;
    }

    /// Returns an updated [QcConfig] with prefered indexing method
    pub fn with_prefered_indexing(&self, indexing: QcPreferedIndexing) -> Self {
        let mut s = self.clone();
        s.indexing = indexing;
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
}

#[cfg(feature = "html")]
impl Render for QcConfig {
    fn render(&self) -> Markup {
        html! {
            table class="styled-table" {
                tbody {
                    tr {
                        th {
                            "Indexing Preference"
                        }
                        td {
                            (self.indexing.to_string())
                        }
                    }
                    tr {
                        th {
                            "Orbit preference"
                        }
                        td {
                            (self.orbit_preference.to_string())
                        }
                    }
                }
            }
        }
    }
}
