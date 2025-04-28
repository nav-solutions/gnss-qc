use std::collections::HashMap;

use gnss_rtk::prelude::{
    Bias, Candidate, Config as PPPConfig, Error as SolverError, Observation, PPPSolver,
    PVTSolution, SV, Epoch, BiasRuntime,
};

use crate::context::{
    navigation::{
        buffer::{
            signals::{QcSignalBuffer, QcSignalData},
            QcNavigationBuffer,
        },
        NavTimeSolver,
    },
    QcContext,
};

use super::buffer::signals::QcMeasuredData;

pub struct NullBias {}

impl Bias for NullBias {
    fn ionosphere_bias_m(&self, _: &BiasRuntime) -> f64 {
        0.0
    }

    fn troposphere_bias_m(&self, _: &BiasRuntime) -> f64 {
        0.0
    }
}

/// [NavPPPSolver] is used to resolve [PVTSolution]s from a [QcContext].
pub struct NavPPPSolver<'a> {
    /// [QcSignalBuffer]
    signals: QcSignalBuffer<'a>,

    /// [QcNavigationBuffer]
    nav_buffer: QcNavigationBuffer<'a>,

    /// Possibly stored "next" data
    next_signal: Option<QcSignalData>,

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
    /// // Load some data
    /// ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
    ///     .unwrap();
    ///
    /// // Navigation compatible contexts greatly enhance the reporting capability.
    /// // We can report
    /// // - the type of navigation process the data set would allow.
    /// ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
    ///     .unwrap();
    ///
    /// let mut nav_ppp = ctx.nav_ppp_solver()
    ///     .expect("This context is navigation compatible!");
    ///
    /// ```
    pub fn nav_ppp_solver<'a>(&'a self, cfg: PPPConfig) -> Option<NavPPPSolver<'a>> {
        let mut signals = self.signals_buffer()?;
        let nav_buffer = self.navigation_buffer()?;
        let nav_time = self.nav_time_solver()?;

        let null_bias = NullBias {};

        let solver = PPPSolver::new(
            self.almanac.clone(),
            self.earth_cef,
            cfg,
            nav_buffer,
            nav_time,
            null_bias,
            None,
        );

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
            nav_buffer,
            next_signal: Some(next_signal),
            candidates: Vec::with_capacity(8),
        })
    }
}

impl<'a> Iterator for NavPPPSolver<'a> {
    type Item = Option<Result<PVTSolution, SolverError>>;

    /// Iterate [NavPPPSolver] and try to obtain a new [PVTSolution].
    fn next(&mut self) -> Option<Self::Item> {

        let mut pending_t = Epoch::default();

        if self.next_signal.is_none() {
            // reached end of stream
            return None;
        }

        // try to gather a complete epoch
        loop {
            let signal = self.signals.next()?;
            let next_signal = self.next_signal.as_ref().unwrap();

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

            self.next_signal = Some(signal.clone());
        }

        self.candidates.clear();

        for (sv, observations) in self.sv_observations.iter() {
            let mut cd = Candidate::new(*sv, pending_t, observations.clone());

            if let Some(tgd) = self.nav_buffer.

            self.candidates.push(cd);
        }

        //TODO: clear sv_observations
        // and latch pending observation described by "next_signal"

        None
    }
}
