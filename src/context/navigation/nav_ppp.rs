use std::collections::HashMap;

use gnss_rtk::prelude::{
    AbsoluteTime, Bias, BiasRuntime, Candidate, Config as PPPConfig, Duration, Epoch, Error,
    Observation, OrbitSource, PVTSolution, StaticPPP, User as UserProfile, PPP, SV,
};

use crate::context::{
    navigation::{
        buffer::{
            ephemeris::QcEphemerisData,
            signals::{QcMeasuredData, QcSignalBuffer, QcSignalData},
            QcNavigationBuffer,
        },
        NavTimeSolver, SolutionsIter,
    },
    QcContext,
};

pub struct NullBias {}

impl Bias for NullBias {
    fn ionosphere_bias_m(&self, _: &BiasRuntime) -> f64 {
        0.0
    }

    fn troposphere_bias_m(&self, _: &BiasRuntime) -> f64 {
        0.0
    }
}

/// [Solver] wrapps the two kinds of PPP solvers
enum PPPSolver<O: OrbitSource, B: Bias, T: AbsoluteTime> {
    /// [StaticPPP] solver
    Static(StaticPPP<O, B, T>),
    /// Dynamic [PPP] solver
    Dynamic(PPP<O, B, T>),
}

impl<O: OrbitSource, B: Bias, T: AbsoluteTime> PPPSolver<O, B, T> {
    fn resolve(
        &mut self,
        user: UserProfile,
        epoch: Epoch,
        candidates: &[Candidate],
    ) -> Result<PVTSolution, Error> {
        match self {
            Self::Static(solver) => solver.resolve(user, epoch, candidates),
            Self::Dynamic(solver) => solver.resolve(user, epoch, candidates),
        }
    }
}

/// [NavPPPSolver] is used to resolve [PVTSolution]s from a [QcContext].
pub struct NavPPPSolver<'a> {
    /// [QcSignalBuffer]
    signals: QcSignalBuffer<'a>,

    /// Possibly stored "next" data
    next_signal: Option<QcSignalData>,

    /// Buffered [QcEphemerisData]
    buffered_ephemeris: Vec<QcEphemerisData>,

    /// [Observation]s
    sv_observations: HashMap<SV, Vec<Observation>>,

    /// [Candidate]s buffer
    candidates: Vec<Candidate>,

    /// Internal [PPPSolver]
    solver: PPPSolver<QcNavigationBuffer<'a>, NullBias, NavTimeSolver>,
}

#[cfg(feature = "navigation")]
#[cfg_attr(docsrs, doc(cfg(feature = "navigation")))]
impl QcContext {
    /// Obtain [NavPvtSolver] from this [QcContext], ready to solve PVT solutions.
    /// Current [QcContext] needs to be navigation compatible.
    /// ```
    /// use gnss_qc::prelude::QcContext;
    ///
    /// let mut ctx = QcContext::new();
    ///
    /// // Data from a Geodetic reference
    /// ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // Data for that day
    /// ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// // Deployment: static application
    /// let mut ppp = ctx.nav_static_ppp_solver()
    ///     .expect("This context is navigation compatible!");
    ///
    /// ```
    pub fn nav_static_ppp_solver<'a>(&'a self, cfg: PPPConfig) -> Option<NavPPPSolver<'a>> {
        // gather all ephemeris
        let buffered_ephemeris = self.buffered_ephemeris_data();

        let nav_time = self.nav_time_solver();
        let nav_buffer = self.navigation_buffer()?;

        let mut signals = self.signals_buffer()?;

        let null_bias = NullBias {};

        let solver = PPPSolver::Static(StaticPPP::new(
            self.almanac.clone(),
            self.earth_cef,
            cfg,
            nav_buffer,
            nav_time,
            null_bias,
            None,
        ));

        let next_signal = signals.next()?;

        let mut sv_observations = HashMap::with_capacity(8);

        // latch first observation
        if let Ok(observation) = next_signal.to_observation() {
            sv_observations.insert(next_signal.sv, vec![observation]);
        }

        Some(NavPPPSolver {
            signals,
            solver,
            sv_observations,
            buffered_ephemeris,
            next_signal: Some(next_signal),
            candidates: Vec::with_capacity(8),
        })
    }
}

impl<'a> SolutionsIter for NavPPPSolver<'a> {
    type Error = Error;
    type Solution = PVTSolution;

    /// Iterate [NavPPPSolver] and try to obtain a new [PVTSolution].
    fn next(&mut self, user_profile: UserProfile) -> Option<Result<Self::Solution, Self::Error>> {
        let mut pending_t = Epoch::default();

        if self.next_signal.is_none() {
            // reached end of stream
            return None;
        }

        let next_signal = self.next_signal.as_ref().unwrap();

        // try to gather a complete epoch
        loop {
            let signal = self.signals.next()?;

            if signal.t > next_signal.t {
                // new Epoch
                self.next_signal = Some(signal.clone());
                break;
            }

            pending_t = signal.t;

            let observation = signal.to_observation();
            if observation.is_err() {
                continue; // can't process
            }

            let observation = observation.unwrap();

            // append to pending list
            if let Some((_, observations)) = self
                .sv_observations
                .iter_mut()
                .filter(|(sv, _)| **sv == signal.sv)
                .reduce(|k, _| k)
            {
                if let Some(observation) = observations
                    .iter_mut()
                    .filter(|obs| obs.carrier == observation.carrier)
                    .reduce(|k, _| k)
                {
                    match signal.measurement {
                        QcMeasuredData::PseudoRange(pr) => {
                            *observation = observation.with_pseudo_range_m(pr);
                        }
                        QcMeasuredData::DopplerShift(dop) => {
                            *observation = observation.with_doppler(dop);
                        }
                        QcMeasuredData::PhaseRange(cp) => {
                            *observation = observation.with_ambiguous_phase_range_m(cp);
                        }
                    }
                } else {
                    // new frequency
                    observations.push(observation);
                }
            } else {
                if let Ok(observation) = next_signal.to_observation() {
                    self.sv_observations.insert(signal.sv, vec![observation]);
                }
            }
        }

        self.candidates.clear();

        for (sv, observations) in self.sv_observations.iter() {
            let mut cd = Candidate::new(*sv, pending_t, observations.clone());

            if let Some(tgd) = self.group_delay(pending_t, *sv) {
                cd.set_group_delay(tgd);
            }

            self.candidates.push(cd);
        }

        // resolution attempt
        let ret = self
            .solver
            .resolve(user_profile, pending_t, &self.candidates);

        // discard outdated to gain future time
        self.buffered_ephemeris
            .retain(|k| k.ephemeris.is_valid(k.sv, pending_t, k.toe));

        // clear consumed observations
        self.sv_observations.clear();

        // latch pending observation
        if let Some(next_signal) = &self.next_signal {
            if let Ok(observation) = next_signal.to_observation() {
                self.sv_observations
                    .insert(next_signal.sv, vec![observation]);
            }
        }

        Some(ret)
    }
}

impl<'a> NavPPPSolver<'a> {
    fn group_delay(&self, t: Epoch, sv: SV) -> Option<Duration> {
        let buffered = self
            .buffered_ephemeris
            .iter()
            .filter(|k| k.ephemeris.is_valid(sv, t, k.toe))
            .min_by_key(|k| k.toe - t)?;

        buffered.ephemeris.tgd()
    }
}
