use rinex::{
    hardware::{Antenna, Receiver},
    marker::GeodeticMarker,
    prelude::{Header, RinexType, Version},
};

use crate::{
    navigation::QcReferencePosition,
    prelude::{Frame, TimeScale},
};

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
    /// Builds [QcRINEXFileSummary] from provided [Header]
    pub fn from_header(header: &Header, frame_ecef: Frame) -> Self {
        Self {
            version: header.version,
            antenna: header.rcvr_antenna.clone(),
            receiver: header.rcvr.clone(),
            geodetic_marker: header.geodetic_marker.clone(),

            reference_position: if let Some((x_ecef_m, y_ecef_m, z_ecef_m)) = header.rx_position {
                if let Some(obs) = &header.obs {
                    if let Some(time_of_first_obs) = obs.timeof_first_obs {
                        Some(QcReferencePosition::new(
                            (x_ecef_m, y_ecef_m, z_ecef_m),
                            time_of_first_obs,
                            frame_ecef,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            },

            v3_time_corrections: if header.rinex_type == RinexType::NavigationData {
                if let Some(nav) = &header.nav {
                    nav.time_offsets.iter().map(|k| (k.lhs, k.rhs)).collect()
                } else {
                    Default::default()
                }
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
