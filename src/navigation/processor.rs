//! Navigation processor
use std::{cell::RefCell, rc::Rc};

use gnss_rtk::prelude::{
    Almanac, Candidate, ClockProfile, Config, Constellation, Method, Solver, UserParameters,
    UserProfile, EARTH_J2000, SV,
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
            QcClockProvider, QcEnvironment, QcNavigationData, QcNullEphemeris, QcOrbitPreference,
            QcOrbitProvider, QcTime,
        },
        rtk::QcRtkBase,
    },
};

/// Navigation technique
#[derive(Debug, Clone, PartialEq)]
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

/// Resolves P.V.T solutions for one rover (absolute or differential navigation),
/// and streams them out. A [QcNavigationReport] is emitted once all signal sources ended.
pub struct QcNavigationProcessor {
    cfg: QcNavigationConfig,
    solver: QcSolver,
    data: Rc<RefCell<QcNavigationData>>,
    rover: Option<QcIndexing>,
    base: Option<QcRtkBase>,
    candidates: Vec<Candidate>,
    report: Option<QcNavigationReport>,
}

impl QcNavigationProcessor {
    /// Deploys a new [QcNavigationProcessor]. The ANISE [Almanac] is used to
    /// describe the Earth and the Sun (eclipses).
    pub fn new(almanac: Almanac, cfg: QcNavigationConfig) -> Result<Self, QcNavigationError> {
        let earth = almanac
            .frame_info(EARTH_J2000)
            .map_err(|e| QcNavigationError::EarthFrame(e.to_string()))?;

        let data = Rc::new(RefCell::new(QcNavigationData::new(
            cfg.orbits,
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

    fn base_indexing(&self) -> Option<&QcIndexing> {
        self.base.as_ref().map(|base| &base.indexing)
    }

    fn failure(&mut self, reason: String) {
        if let Some(report) = &mut self.report {
            *report.failures.entry(reason).or_default() += 1;
        }
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcNavigationProcessor {
    fn name(&self) -> &str {
        "navigation"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
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

        let rover = match &self.rover {
            Some(rover) => rover.clone(),
            None => return,
        };

        let rx = match ctx.receivers.iter().find(|rx| rx.receiver == rover) {
            Some(rx) => rx,
            None => return,
        };

        let report = self
            .report
            .get_or_insert_with(|| QcNavigationReport::new(rover.clone()));

        report.epochs += 1;
        report.last_epoch = Some(ctx.epoch);

        build_candidates(
            ctx.epoch,
            &rx.signals,
            &self.cfg.constellations,
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
                            None => {
                                self.failure("rtk base position unknown".to_string());
                                return;
                            }
                        }
                    }

                    let mut base_candidates = std::mem::take(&mut base.candidates);
                    build_candidates(
                        ctx.epoch,
                        &base_rx.signals,
                        &self.cfg.constellations,
                        &mut base_candidates,
                    );
                    base.candidates = base_candidates;
                }
                None => {
                    self.failure("rtk base missing".to_string());
                    return;
                }
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
                let report = self.report.as_mut().unwrap();
                report.solutions += 1;

                output.push(
                    ctx.epoch,
                    QcOutput::PVTSolution(QcPvtSolution {
                        receiver: rover,
                        solution,
                    }),
                );
            }
            Err(e) => {
                #[cfg(feature = "log")]
                debug!("{} - {} navigation failure: {}", ctx.epoch, rover, e);
                self.failure(e.to_string());
            }
        }
    }

    fn finalize(&mut self, output: &mut QcOutputQueue<O>) {
        if let Some(mut report) = self.report.take() {
            let counters = self.data.borrow().counters();
            report.precise_orbits = counters.precise;
            report.broadcast_orbits = counters.broadcast;

            if let Some(last) = report.last_epoch {
                output.push(last, QcOutput::NavigationReport(report));
            } else {
                output.push(
                    hifitime::Epoch::default(),
                    QcOutput::NavigationReport(report),
                );
            }
        }
    }
}
