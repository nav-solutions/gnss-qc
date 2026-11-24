//! Navigation processor
use std::{cell::RefCell, rc::Rc};

use gnss_rtk::prelude::{
    Almanac, Candidate, ClockProfile, Config, Constellation, Epoch, Method, PVTSolution, Solver,
    UserParameters, UserProfile, EARTH_J2000, SV,
};

use thiserror::Error;

#[cfg(feature = "log")]
use log::debug;

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcNavigationReport, QcOutput, QcOutputQueue, QcPvtSolution},
    },
    navigation::{
        candidates::build_candidates,
        provider::{
            QcClockProvider, QcEnvironment, QcNavigationData, QcNullEphemeris, QcOrbitProvider,
            QcTime,
        },
        rtk::QcRtkBase,
    },
    orbit::{QcClockKind, QcClockPreference, QcOrbitKind, QcOrbitPreference},
};

/// Navigation technique
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcNavigationTechnique {
    /// Absolute navigation (no reference station): position and clock are resolved.
    Absolute,

    /// Differential navigation, relative to a reference station that is one of the
    /// signal sources. Its position is taken from its source descriptor
    /// (RINEX header) unless specified here.
    Rtk {
        base: QcIndexing,
        position_ecef_m: Option<(f64, f64, f64)>,
    },
}

/// Navigation configuration
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcNavigationConfig {
    /// Navigation technique
    pub technique: QcNavigationTechnique,

    /// Rover (signal source to navigate). When not defined, the first signal source
    /// (that is not the RTK base) is selected.
    pub rover: Option<QcIndexing>,

    /// Constellations to navigate with
    pub constellations: Vec<Constellation>,

    /// Orbital products preference
    pub orbits: QcOrbitPreference,

    /// Satellite clocks preference: precise clocks (Clock RINEX, SP3)
    /// or clocks of the radio messages.
    #[cfg_attr(feature = "serde", serde(default))]
    pub clocks: QcClockPreference,

    /// Solver configuration (navigation method, models..)
    pub solver: Config,

    /// User profile
    pub user: UserParameters,

    /// Possible a priori position (ECEF m), otherwise the solver surveys from scratch.
    pub apriori_ecef_m: Option<(f64, f64, f64)>,
}

impl Default for QcNavigationConfig {
    fn default() -> Self {
        Self {
            technique: QcNavigationTechnique::Absolute,
            rover: None,
            constellations: vec![Constellation::GPS, Constellation::Galileo],
            orbits: QcOrbitPreference::Auto,
            clocks: QcClockPreference::Auto,
            solver: Config::default(),
            user: UserParameters::new(UserProfile::Pedestrian, ClockProfile::Quartz),
            apriori_ecef_m: None,
        }
    }
}

impl QcNavigationConfig {
    /// Selects the navigation [Method]
    pub fn with_method(mut self, method: Method) -> Self {
        self.solver = self.solver.with_navigation_method(method);
        self
    }

    /// Selects the constellations
    pub fn with_constellations(mut self, constellations: &[Constellation]) -> Self {
        self.constellations = constellations.to_vec();
        self
    }

    /// Selects the orbital products preference
    pub fn with_orbits(mut self, orbits: QcOrbitPreference) -> Self {
        self.orbits = orbits;
        self
    }

    /// Selects the satellite clocks preference
    pub fn with_clocks(mut self, clocks: QcClockPreference) -> Self {
        self.clocks = clocks;
        self
    }

    /// Selects the rover
    pub fn with_rover(mut self, rover: QcIndexing) -> Self {
        self.rover = Some(rover);
        self
    }

    /// Differential navigation relative to this signal source
    pub fn with_rtk_base(mut self, base: QcIndexing) -> Self {
        self.technique = QcNavigationTechnique::Rtk {
            base,
            position_ecef_m: None,
        };
        self
    }

    /// Selects the [UserParameters]
    pub fn with_user_parameters(mut self, user: UserParameters) -> Self {
        self.user = user;
        self
    }
}

/// Navigation setup errors
#[derive(Debug, Error)]
pub enum QcNavigationError {
    #[error("almanac: failed to obtain the earth frame: {0}")]
    EarthFrame(String),
}

type QcSolver = Solver<QcNullEphemeris, QcOrbitProvider, QcEnvironment, QcClockProvider, QcTime>;

/// Result of one navigation attempt: rover identity and solution, or failure reason
pub type QcNavigationResult = Result<(QcIndexing, PVTSolution), String>;

/// [QcNavigator] is the navigation core: it deploys the GNSS-RTK solver over the
/// synchronized context, epoch after epoch. It is used by the [QcNavigationProcessor],
/// and by the processors that post-process P.V.T solutions (CGGTTS..).
pub struct QcNavigator {
    cfg: QcNavigationConfig,
    solver: QcSolver,
    data: Rc<RefCell<QcNavigationData>>,
    rover: Option<QcIndexing>,
    base: Option<QcRtkBase>,
    candidates: Vec<Candidate>,
    report: Option<QcNavigationReport>,
}

impl QcNavigator {
    /// Deploys a new [QcNavigator]. The ANISE [Almanac] is used to
    /// describe the Earth and the Sun (eclipses).
    pub fn new(almanac: Almanac, cfg: QcNavigationConfig) -> Result<Self, QcNavigationError> {
        let earth = almanac
            .frame_info(EARTH_J2000)
            .map_err(|e| QcNavigationError::EarthFrame(e.to_string()))?;

        let data = Rc::new(RefCell::new(QcNavigationData::new(
            cfg.orbits,
            cfg.clocks,
            cfg.solver.method,
        )));

        let solver = Solver::new(
            almanac,
            earth,
            cfg.solver.clone(),
            Rc::new(QcNullEphemeris),
            Rc::new(QcOrbitProvider { data: data.clone() }),
            Rc::new(QcClockProvider { data: data.clone() }),
            Rc::new(QcEnvironment),
            QcTime,
            cfg.apriori_ecef_m,
        );

        let base = match &cfg.technique {
            QcNavigationTechnique::Absolute => None,
            QcNavigationTechnique::Rtk {
                base,
                position_ecef_m,
            } => Some(QcRtkBase {
                indexing: base.clone(),
                position_ecef_m: position_ecef_m.unwrap_or_default(),
                candidates: Vec::new(),
            }),
        };

        Ok(Self {
            rover: cfg.rover.clone(),
            cfg,
            solver,
            data,
            base,
            candidates: Vec::with_capacity(64),
            report: None,
        })
    }

    /// Navigation configuration
    pub fn config(&self) -> &QcNavigationConfig {
        &self.cfg
    }

    /// Satellite clock offset (s) that was applied for `sv` at `t`
    /// (relativistic correction included), from the selected clock product.
    /// Valid for the satellites of the latest resolution attempt.
    pub fn satellite_clock_offset_s(&self, sv: SV, t: Epoch) -> Option<f64> {
        self.data.borrow().clock_s(sv, t)
    }

    /// Orbital product used for `sv` in the latest resolution attempt
    pub fn orbit_kind(&self, sv: SV) -> Option<QcOrbitKind> {
        self.data.borrow().orbit_kind(sv)
    }

    /// Clock product used for `sv` in the latest resolution attempt
    pub fn clock_kind(&self, sv: SV) -> Option<QcClockKind> {
        self.data.borrow().clock_kind(sv)
    }

    fn base_indexing(&self) -> Option<&QcIndexing> {
        self.base.as_ref().map(|base| &base.indexing)
    }

    fn failure(&mut self, reason: String) -> Option<QcNavigationResult> {
        if let Some(report) = &mut self.report {
            *report.failures.entry(reason.clone()).or_default() += 1;
        }
        Some(Err(reason))
    }

    /// Resolution attempt at this epoch. Returns None when the rover did not sample
    /// this epoch (nothing to solve).
    pub fn solve(&mut self, ctx: &QcEpochContext) -> Option<QcNavigationResult> {
        // rover selection (first epoch)
        if self.rover.is_none() {
            let base = self.base_indexing().cloned();
            self.rover = ctx
                .receivers
                .iter()
                .map(|rx| &rx.receiver)
                .find(|rx| Some(*rx) != base.as_ref())
                .cloned();
        }

        let rover = self.rover.clone()?;
        let rx = ctx.receivers.iter().find(|rx| rx.receiver == rover)?;

        let report = self
            .report
            .get_or_insert_with(|| QcNavigationReport::new(rover.clone()));

        report.epochs += 1;
        report.last_epoch = Some(ctx.epoch);

        let primary_only = self.cfg.solver.method == Method::SPP;

        build_candidates(
            ctx.epoch,
            &rx.signals,
            &self.cfg.constellations,
            primary_only,
            &mut self.candidates,
        );

        // RTK base: same epoch
        if let Some(base) = &mut self.base {
            match ctx.receivers.iter().find(|rx| rx.receiver == base.indexing) {
                Some(base_rx) => {
                    if let QcNavigationTechnique::Rtk {
                        position_ecef_m: None,
                        ..
                    } = &self.cfg.technique
                    {
                        match base_rx.reference_position_ecef_m {
                            Some(position) => base.position_ecef_m = position,
                            None => return self.failure("rtk base position unknown".to_string()),
                        }
                    }

                    let mut base_candidates = std::mem::take(&mut base.candidates);
                    build_candidates(
                        ctx.epoch,
                        &base_rx.signals,
                        &self.cfg.constellations,
                        primary_only,
                        &mut base_candidates,
                    );
                    base.candidates = base_candidates;
                }
                None => return self.failure("rtk base missing".to_string()),
            }
        }

        // navigation data snapshot
        let mut svs = self.candidates.iter().map(|cd| cd.sv).collect::<Vec<SV>>();

        if let Some(base) = &self.base {
            svs.extend(base.candidates.iter().map(|cd| cd.sv));
        }

        svs.sort();
        svs.dedup();

        self.data.borrow_mut().update(ctx, svs.into_iter());

        let result = match &self.base {
            Some(base) => self
                .solver
                .rtk(ctx.epoch, self.cfg.user, &self.candidates, base),
            None => self.solver.ppp(ctx.epoch, self.cfg.user, &self.candidates),
        };

        match result {
            Ok(solution) => {
                self.report.as_mut().unwrap().solutions += 1;
                Some(Ok((rover, solution)))
            }
            Err(e) => {
                #[cfg(feature = "log")]
                debug!("{} - {} navigation failure: {}", ctx.epoch, rover, e);
                self.failure(e.to_string())
            }
        }
    }

    /// Navigation summary (once, at the end of the session)
    pub fn take_report(&mut self) -> Option<QcNavigationReport> {
        let mut report = self.report.take()?;
        let counters = self.data.borrow().counters();
        report.precise_orbits = counters.precise;
        report.broadcast_orbits = counters.broadcast;
        report.precise_clocks = counters.precise_clocks;
        report.precise_state_clocks = counters.precise_state_clocks;
        report.broadcast_clocks = counters.broadcast_clocks;
        Some(report)
    }
}

/// Resolves P.V.T solutions for one rover (absolute or differential navigation),
/// and streams them out. A [QcNavigationReport] is emitted once all signal sources ended.
pub struct QcNavigationProcessor {
    navigator: QcNavigator,
}

impl QcNavigationProcessor {
    /// Deploys a new [QcNavigationProcessor]. The ANISE [Almanac] is used to
    /// describe the Earth and the Sun (eclipses).
    pub fn new(almanac: Almanac, cfg: QcNavigationConfig) -> Result<Self, QcNavigationError> {
        Ok(Self {
            navigator: QcNavigator::new(almanac, cfg)?,
        })
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcNavigationProcessor {
    fn name(&self) -> &str {
        "navigation"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        if let Some(Ok((receiver, solution))) = self.navigator.solve(ctx) {
            output.push(
                ctx.epoch,
                QcOutput::PVTSolution(QcPvtSolution { receiver, solution }),
            );
        }
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<O>) {
        if let Some(report) = self.navigator.take_report() {
            let epoch = report.last_epoch.unwrap_or_default();
            output.push(epoch, QcOutput::NavigationReport(report));
        }
    }
}
