//! Ionospheric Pierce Points (IPP)
use ionex::prelude::{Point, IPP};

use gnss_rs::prelude::{Constellation, SV};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    geodesy::{ecef_to_enu_vector, ecef_to_geodetic},
    io::{
        input::indexing::QcIndexing,
        output::{QcIppOutput, QcOutput, QcOutputQueue},
    },
    orbit::{satellite_state, QcOrbitPreference},
};

/// IPP configuration
#[derive(Debug, Clone, PartialEq)]
pub struct QcIppConfig {
    /// Orbital products preference. Default: precise products whenever available.
    pub orbits: QcOrbitPreference,

    /// Elevation mask (degrees). Default: 10°.
    pub elevation_mask_deg: f64,

    /// Altitude of the ionospheric shell (single layer model), in km.
    /// Default: 450 km (IGS ionosphere maps).
    pub shell_altitude_km: f64,

    /// Radius of the (spherical) Earth, in km. Default: 6371 km (IGS ionosphere maps).
    pub earth_radius_km: f64,

    /// Receivers to process. Default: all receivers.
    pub receivers: Option<Vec<QcIndexing>>,

    /// Constellations to process. Default: all constellations.
    pub constellations: Option<Vec<Constellation>>,

    /// Receiver position (ECEF m). Default: the reference position of each signal
    /// source (RINEX header). Receivers without position are not processed.
    pub receiver_position_ecef_m: Option<(f64, f64, f64)>,
}

impl Default for QcIppConfig {
    fn default() -> Self {
        Self {
            orbits: QcOrbitPreference::Auto,
            elevation_mask_deg: 10.0,
            shell_altitude_km: 450.0,
            earth_radius_km: 6371.0,
            receivers: None,
            constellations: None,
            receiver_position_ecef_m: None,
        }
    }
}

/// Resolves the Ionospheric Pierce Points of every line of sight (receiver to satellite
/// in sight), epoch after epoch, using the IONEX library single layer model.
/// Satellites positions come from the synchronized context: precise products (SP3)
/// or broadcast ephemerides (Keplerian solving), see [QcOrbitPreference].
#[derive(Debug, Default, Clone)]
pub struct QcIppProcessor {
    cfg: QcIppConfig,
    svs: Vec<SV>,
}

impl QcIppProcessor {
    /// Builds a new [QcIppProcessor]
    pub fn new(cfg: QcIppConfig) -> Self {
        Self {
            cfg,
            svs: Vec::with_capacity(64),
        }
    }
}

/// (elevation, azimuth) in degrees of `target` seen from `origin` (both ECEF m)
pub fn elevation_azimuth_deg(origin: (f64, f64, f64), target: (f64, f64, f64)) -> (f64, f64) {
    let delta = (
        target.0 - origin.0,
        target.1 - origin.1,
        target.2 - origin.2,
    );
    let (e, n, u) = ecef_to_enu_vector(origin, delta);
    let elevation = u.atan2((e * e + n * n).sqrt()).to_degrees();
    let azimuth = e.atan2(n).to_degrees().rem_euclid(360.0);
    (elevation, azimuth)
}

impl<O: From<QcOutput>> QcProcessor<O> for QcIppProcessor {
    fn name(&self) -> &str {
        "ipp"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        for rx in ctx.receivers {
            if let Some(receivers) = &self.cfg.receivers {
                if !receivers.contains(&rx.receiver) {
                    continue;
                }
            }

            let position = match self
                .cfg
                .receiver_position_ecef_m
                .or(rx.reference_position_ecef_m)
            {
                Some(position) => position,
                None => continue,
            };

            let (lat_rad, long_rad, _) = ecef_to_geodetic(position);
            let receiver = Point::new(long_rad.to_degrees(), lat_rad.to_degrees());

            self.svs.clear();
            self.svs.extend(rx.signals.iter().map(|signal| signal.sv));
            self.svs.sort();
            self.svs.dedup();

            for sv in self.svs.iter() {
                if let Some(constellations) = &self.cfg.constellations {
                    if !constellations.contains(&sv.constellation) {
                        continue;
                    }
                }

                let state = match satellite_state(ctx, *sv, ctx.epoch, self.cfg.orbits) {
                    Some(state) => state,
                    None => continue,
                };

                let (elevation_deg, azimuth_deg) =
                    elevation_azimuth_deg(position, state.position_m);

                if elevation_deg < self.cfg.elevation_mask_deg {
                    continue;
                }

                let ipp = match IPP::new(
                    receiver,
                    elevation_deg,
                    azimuth_deg,
                    self.cfg.earth_radius_km,
                    self.cfg.shell_altitude_km,
                ) {
                    Ok(ipp) => ipp,
                    Err(_) => continue,
                };

                output.push(
                    ctx.epoch,
                    QcOutput::IPP(QcIppOutput {
                        receiver: rx.receiver.clone(),
                        sv: *sv,
                        orbit: state.kind,
                        elevation_deg,
                        azimuth_deg,
                        latitude_deg: ipp.coordinates.y(),
                        longitude_deg: ipp.coordinates.x(),
                        altitude_km: ipp.altitude_km,
                        obliquity_factor: ipp.obliquity_factor(),
                    }),
                );
            }
        }
    }
}
