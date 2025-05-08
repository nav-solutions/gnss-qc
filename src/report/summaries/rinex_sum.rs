use rinex::{
    hardware::{Antenna, Receiver},
    marker::GeodeticMarker,
    prelude::{Header, RinexType, Version},
};

use crate::{navigation::QcReferencePosition, prelude::TimeScale};

/// [QcRINEXFileSummary] summary.
#[derive(Debug, Clone, Default)]
pub struct QcRINEXFileSummary {
    /// RINEX [Version]
    pub version: Version,

    /// [TimeScale] if that applies
    pub timescale: Option<TimeScale>,

    /// Reference Position (if any)
    pub reference_position: Option<QcReferencePosition>,

    /// Receiver info (if any)
    pub receiver: Option<Receiver>,

    /// Antenna info (if any)
    pub antenna: Option<Antenna>,

    /// Geodetic marker
    pub geodetic_marker: Option<GeodeticMarker>,

    /// V3 System Time Corrections (if any)
    pub v3_time_corrections: Vec<(TimeScale, TimeScale)>,
}

impl QcRINEXFileSummary {
    pub fn from_header(header: &Header) -> Self {
        Self {
            version: header.version,
            antenna: header.rcvr_antenna.clone(),
            receiver: header.rcvr.clone(),
            geodetic_marker: header.geodetic_marker.clone(),

            reference_position: if let Some((x_ecef_m, y_ecef_m, z_ecef_m)) = header.rx_position {
                Some(QcReferencePosition::new((x_ecef_m, y_ecef_m, z_ecef_m)))
            } else {
                None
            },

            v3_time_corrections: if header.rinex_type == RinexType::NavigationData {
                Default::default()
            } else {
                Default::default()
            },

            timescale: match header.rinex_type {
                RinexType::ObservationData => {
                    if let Some(obs) = &header.obs {
                        if let Some(epoch) = obs.timeof_first_obs {
                            Some(epoch.time_scale)
                        } else if let Some(epoch) = obs.timeof_last_obs {
                            Some(epoch.time_scale)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                RinexType::IonosphereMaps | RinexType::MeteoData | RinexType::ClockData => {
                    Some(TimeScale::UTC)
                }
                RinexType::NavigationData | RinexType::AntennaData | RinexType::DORIS => None,
            },
        }
    }
}
