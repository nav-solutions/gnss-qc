//! CGGTTS solutions (common view time transfer).
//!
//! P.V.T solutions (SPP, CPP or PPP, absolute navigation) are post-fitted, per satellite,
//! over standardized tracking periods (BIPM schedule by default: 16' periods made of
//! a 3' setup followed by 13' of data collection). One CGGTTS track is released
//! per satellite tracked during an entire period.
use std::collections::HashMap;

use cggtts::prelude::{
    CommonViewCalendar, CommonViewClass, Observation as FitObservation, SkyTracker,
};

use gnss_rtk::prelude::{
    Almanac, Carrier, Constellation, Duration, Epoch, IonosphereBias, PVTSolution, Signal,
    TimeScale, SPEED_OF_LIGHT_M_S, SV,
};

use thiserror::Error;

#[cfg(feature = "log")]
use log::debug;

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcCggttsReport, QcCggttsTrack, QcOutput, QcOutputQueue, QcPvtSolution},
    },
    navigation::{QcNavigationConfig, QcNavigationError, QcNavigationTechnique, QcNavigator},
};

/// CGGTTS setup errors
#[derive(Debug, Error)]
pub enum QcCggttsError {
    /// CGGTTS requires absolute navigation (the clock state is not resolved in RTK)
    #[error("cggtts requires absolute navigation: RTK is not supported")]
    RtkNotSupported,

    #[error("navigation: {0}")]
    Navigation(#[from] QcNavigationError),
}

/// CGGTTS configuration
#[derive(Debug, Clone)]
pub struct QcCggttsConfig {
    /// Tracking calendar. Default: BIPM schedule
    /// (16' periods aligned to MJD 50722 + 2', -4' daily offset).
    pub calendar: CommonViewCalendar,

    /// Setup duration at the beginning of each period, during which data is not
    /// collected. Must match the calendar periods. Default: 3' (BIPM).
    pub setup_duration: Duration,

    /// Minimal track duration: satellites tracked for a shorter duration
    /// (rising, setting, gaps) do not produce a track. Default: 12'.
    pub min_track_duration: Duration,

    /// Data gap tolerance: a larger gap in the P.V.T solutions (or in the contribution
    /// of a satellite) restarts its track. Default: 1'.
    pub gap_tolerance: Duration,

    /// Common view class of the tracks. Default: multi channel (all in view receivers).
    pub class: CommonViewClass,

    /// Also stream the P.V.T solutions. Default: false.
    pub pvt_solutions: bool,
}

impl Default for QcCggttsConfig {
    fn default() -> Self {
        Self {
            calendar: CommonViewCalendar::bipm(),
            setup_duration: Duration::from_seconds(180.0),
            min_track_duration: Duration::from_seconds(720.0),
            gap_tolerance: Duration::from_seconds(60.0),
            class: CommonViewClass::MultiChannel,
            pvt_solutions: false,
        }
    }
}

/// CGGTTS "frequency code" of the signal(s) used by the navigation
pub fn cggtts_frequency_code(constellation: Constellation, signal: &Signal) -> String {
    let dual = matches!(signal, Signal::Dual(_));

    let code = match (constellation, dual) {
        (Constellation::GPS | Constellation::QZSS, true) => "L3P",
        (Constellation::GPS | Constellation::QZSS, false) => match signal {
            Signal::Single(Carrier::L2) => "L2P",
            Signal::Single(Carrier::L5) => "L5",
            _ => "L1C",
        },
        (Constellation::Galileo, true) => "L3E",
        (Constellation::Galileo, false) => match signal {
            Signal::Single(Carrier::L5) => "E5a",
            Signal::Single(Carrier::E5b) => "E5b",
            _ => "E1",
        },
        (Constellation::BeiDou, true) => "L3B",
        (Constellation::BeiDou, false) => "B1i",
        (_, true) => "L3",
        (_, false) => "L1",
    };

    code.to_string()
}

/// Issue of ephemeris (CGGTTS "IOE"): IODE (GPS, QZSS), IODnav (Galileo), AODE (BeiDou)
fn issue_of_ephemeris(ctx: &QcEpochContext, sv: SV) -> Option<u16> {
    let (_, eph) = ctx.ephemerides.select(sv, ctx.epoch)?;
    let field = match sv.constellation {
        Constellation::Galileo => "iodnav",
        Constellation::BeiDou => "aode",
        _ => "iode",
    };
    eph.ephemeris.get_orbit_f64(field).map(|ioe| ioe as u16)
}

#[derive(Debug, Default, Clone)]
struct SvTrackInfo {
    frc: String,
    ioe: u16,
}

/// Current tracking period (UTC)
#[derive(Debug, Clone, Copy)]
struct Period {
    collection_start: Epoch,
    end: Epoch,
}

/// Resolves CGGTTS tracks: absolute navigation (SPP, CPP or PPP), then post-fit of the
/// solutions per satellite over each tracking period. Tracks are streamed as soon as
/// their period ends ([QcOutput::CggttsTrack]); a [QcCggttsReport] and a
/// [crate::prelude::QcNavigationReport] are emitted at the end of the session.
pub struct QcCggttsProcessor {
    cfg: QcCggttsConfig,
    navigator: QcNavigator,
    tracker: SkyTracker,
    period: Option<Period>,
    receiver: Option<QcIndexing>,
    info: HashMap<SV, SvTrackInfo>,
    report: Option<QcCggttsReport>,
    last_epoch: Option<Epoch>,
}

impl QcCggttsProcessor {
    /// Deploys a new [QcCggttsProcessor].
    /// ## Input
    /// - almanac: ANISE almanac
    /// - navigation: navigation configuration (absolute navigation only: SPP, CPP or PPP)
    /// - cfg: CGGTTS configuration
    pub fn new(
        almanac: Almanac,
        navigation: QcNavigationConfig,
        cfg: QcCggttsConfig,
    ) -> Result<Self, QcCggttsError> {
        if matches!(navigation.technique, QcNavigationTechnique::Rtk { .. }) {
            return Err(QcCggttsError::RtkNotSupported);
        }

        let navigator = QcNavigator::new(almanac, navigation)?;

        Ok(Self {
            tracker: SkyTracker::new().with_gap_tolerance(cfg.gap_tolerance),
            cfg,
            navigator,
            period: None,
            receiver: None,
            info: HashMap::new(),
            report: None,
            last_epoch: None,
        })
    }

    /// Tracking period containing `t` (UTC)
    fn period_of(&self, t: Epoch) -> Period {
        let end = self.cfg.calendar.next_period_start_after(t);
        let start = end - self.cfg.calendar.total_period_duration();
        Period {
            collection_start: start + self.cfg.setup_duration,
            end,
        }
    }

    /// Fits all tracked satellites and releases the tracks
    fn release<O: From<QcOutput>>(&mut self, epoch: Epoch, output: &mut QcOutputQueue<O>) {
        let tracker = std::mem::replace(
            &mut self.tracker,
            SkyTracker::new().with_gap_tolerance(self.cfg.gap_tolerance),
        );

        let (receiver, report) = match (&self.receiver, &mut self.report) {
            (Some(receiver), Some(report)) => (receiver, report),
            _ => return,
        };

        let mut tracker = tracker;
        let mut satellites = tracker.satellites().copied().collect::<Vec<_>>();
        satellites.sort();

        for sv in satellites {
            match tracker.track_fit(sv) {
                Ok(fitted) => {
                    if fitted.duration < self.cfg.min_track_duration {
                        report.short_tracks += 1;
                        continue;
                    }

                    let info = self.info.get(&sv).cloned().unwrap_or_default();
                    let track = fitted.to_track(self.cfg.class, info.ioe, &info.frc);

                    report.tracks += 1;

                    output.push(
                        epoch,
                        QcOutput::CggttsTrack(QcCggttsTrack {
                            receiver: receiver.clone(),
                            track: Box::new(track),
                        }),
                    );
                }
                Err(_e) => {
                    #[cfg(feature = "log")]
                    debug!("{} ({}) cggtts fit error: {}", epoch, sv, _e);
                    report.fit_errors += 1;
                }
            }
        }
    }

    /// Feeds the trackers with one P.V.T solution
    fn track(&mut self, ctx: &QcEpochContext, t_utc: Epoch, pvt: &PVTSolution) {
        let refsys = pvt.clock_offset_s;

        for contribution in pvt.sv.iter() {
            let sv = contribution.sv;

            // satellite clock offset that was applied to resolve this solution
            let dt_sv = match self.navigator.satellite_clock_offset_s(sv, ctx.epoch) {
                Some(dt) => dt,
                None => continue,
            };

            // REFSV: reference clock - satellite clock = REFSYS - dt_sv
            let refsv = refsys - dt_sv;

            let mdtr = contribution.tropo_bias.unwrap_or_default() / SPEED_OF_LIGHT_M_S;

            let (mdio, msio) = match contribution.iono_bias {
                Some(IonosphereBias::Modeled(bias_m)) => (bias_m / SPEED_OF_LIGHT_M_S, None),
                Some(IonosphereBias::Measured(bias_m)) => (0.0, Some(bias_m / SPEED_OF_LIGHT_M_S)),
                // dual frequency: physically cancelled
                None => (0.0, None),
            };

            self.tracker.new_observation(
                sv,
                FitObservation {
                    epoch: t_utc,
                    refsv,
                    refsys,
                    mdtr,
                    mdio,
                    msio,
                    elevation: contribution.elevation_deg,
                    azimuth: contribution.azimuth_deg,
                },
            );

            let info = self.info.entry(sv).or_default();
            info.frc = cggtts_frequency_code(sv.constellation, &contribution.signal);

            if let Some(ioe) = issue_of_ephemeris(ctx, sv) {
                info.ioe = ioe;
            }
        }
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcCggttsProcessor {
    fn name(&self) -> &str {
        "cggtts"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let result = self.navigator.solve(ctx);

        // CGGTTS is scheduled and expressed in UTC
        let t_utc = ctx.epoch.to_time_scale(TimeScale::UTC);
        self.last_epoch = Some(ctx.epoch);

        // end of period: release
        if self.period.is_some_and(|period| t_utc >= period.end) {
            self.release(ctx.epoch, output);
            self.period = None;
        }

        if self.period.is_none() {
            self.period = Some(self.period_of(t_utc));
        }

        let (receiver, solution) = match result {
            Some(Ok(solution)) => solution,
            _ => return,
        };

        if self.receiver.is_none() {
            self.report = Some(QcCggttsReport::new(receiver.clone()));
            self.receiver = Some(receiver.clone());
        }

        let period = self.period.unwrap();

        if t_utc >= period.collection_start {
            self.track(ctx, t_utc, &solution);
        }

        if self.cfg.pvt_solutions {
            output.push(
                ctx.epoch,
                QcOutput::PVTSolution(QcPvtSolution { receiver, solution }),
            );
        }
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<O>) {
        let epoch = self.last_epoch.unwrap_or_default();

        // last period: complete tracks only (minimal duration)
        self.release(epoch, output);

        if let Some(report) = self.report.take() {
            output.push(epoch, QcOutput::CggttsReport(report));
        }

        if let Some(report) = self.navigator.take_report() {
            output.push(epoch, QcOutput::NavigationReport(report));
        }
    }
}
