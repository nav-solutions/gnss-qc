use std::collections::HashMap;

use gnss_rtk::prelude::{
    Bias, Candidate, Config as PVTConfig, Observation, PVTSolution, Solver, SV,
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

pub struct NullBias {}

impl Bias for NullBias {
    fn ionosphere_bias_m(&self, rtm: &gnss_rtk::prelude::BiasRuntime) -> f64 {
        0.0
    }

    fn troposphere_bias_m(&self, rtm: &gnss_rtk::prelude::BiasRuntime) -> f64 {
        0.0
    }
}

/// [NavPvtSolver] is used to resolve [PVTSolution]s from a [QcContext].
pub struct NavPvtSolver<'a> {
    /// [QcSignalBuffer]
    signals: QcSignalBuffer<'a>,

    /// Possibly stored "next" data
    next_signal: Option<QcSignalData>,

    /// [Observation]s
    sv_observations: HashMap<SV, Observation>,

    /// [Candidate]s buffer
    candidates: Vec<Candidate>,

    /// Internal [Solver]
    solver: Solver<QcNavigationBuffer<'a>, NullBias, NavTimeSolver>,
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
    /// let mut nav_pvt = ctx.nav_pvt_solver()
    ///     .expect("This context is navigation compatible!");
    ///
    /// ```
    pub fn nav_pvt_solver<'a>(&'a self, cfg: PVTConfig) -> Option<NavPvtSolver<'a>> {
        let mut signals = self.signals_buffer()?;
        let nav_buffer = self.navigation_buffer()?;
        let nav_time = self.nav_time_solver()?;

        let null_bias = NullBias {};

        let solver = Solver::new_almanac_frame(
            cfg,
            self.almanac.clone(),
            self.earth_cef,
            nav_buffer,
            nav_time,
            null_bias,
            None,
        );

        let next_signal = signals.next()?;

        Some(NavPvtSolver {
            signals,
            solver,
            next_signal: Some(next_signal),
            candidates: Vec::with_capacity(8),
            sv_observations: HashMap::with_capacity(8),
        })
    }
}

impl<'a> Iterator for NavPvtSolver<'a> {
    type Item = Option<PVTSolution>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_signal.is_none() {
            return None;
        }

        // try to complete ongoing epoch
        loop {
            let signal = self.signals.next()?;
            let next_signal = self.next_signal.as_ref().unwrap();

            if signal.t > next_signal.t {
                // new Epoch
                self.next_signal = Some(signal.clone());
                break;
            }

            // append to pending list
            if let Some(sv_observations) = self
                .sv_observations
                .iter_mut()
                .filter(|(sv, _)| **sv == signal.sv)
                .reduce(|k, _| k)
            {
            } else {
                let observation = next_signal.to_observation();
                // self.sv_observations.insert(signal.sv, observation);
            }

            self.next_signal = Some(signal.clone());
        }

        None
    }
}
