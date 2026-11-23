//! Observation residuals: A - B between two receivers, for physically identical observations.
use std::collections::HashMap;

use gnss_rs::prelude::SV;
use hifitime::{Duration, Epoch};

use rinex::prelude::{obs::SignalObservation, Observable};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcObservationResidual, QcOutput, QcOutputQueue},
    },
};

/// Observation residuals configuration
#[derive(Debug, Clone, PartialEq)]
pub struct QcObservationResidualConfig {
    /// Receiver A (minuend). Default: the first receiver encountered.
    pub lhs: Option<QcIndexing>,

    /// Receiver B (subtrahend). Default: the first other receiver encountered.
    pub rhs: Option<QcIndexing>,

    /// Differentiate pseudo ranges (m). Default: true.
    pub pseudo_range: bool,

    /// Differentiate phase ranges (cycles). Default: true.
    pub phase_range: bool,

    /// Differentiate Doppler shifts (Hz). Default: false.
    pub doppler: bool,

    /// Differentiate signal strengths. Default: false.
    pub ssi: bool,

    /// Maximal time difference between the epochs of both receivers.
    /// Default: half the sampling interval of receiver A.
    pub tolerance: Option<Duration>,
}

impl Default for QcObservationResidualConfig {
    fn default() -> Self {
        Self {
            lhs: None,
            rhs: None,
            pseudo_range: true,
            phase_range: true,
            doppler: false,
            ssi: false,
            tolerance: None,
        }
    }
}

impl QcObservationResidualConfig {
    /// A - B between these two receivers
    pub fn new(lhs: QcIndexing, rhs: QcIndexing) -> Self {
        Self {
            lhs: Some(lhs),
            rhs: Some(rhs),
            ..Default::default()
        }
    }

    /// Differentiate every observation (pseudo range, phase, Doppler, signal strength)
    pub fn with_all_observables(mut self) -> Self {
        self.pseudo_range = true;
        self.phase_range = true;
        self.doppler = true;
        self.ssi = true;
        self
    }

    fn selected(&self, observable: &Observable) -> bool {
        match observable {
            Observable::PseudoRange(_) => self.pseudo_range,
            Observable::PhaseRange(_) => self.phase_range,
            Observable::Doppler(_) => self.doppler,
            Observable::SSI(_) => self.ssi,
            _ => false,
        }
    }
}

/// Latest epoch sampled by one receiver
#[derive(Debug, Default, Clone)]
struct Latest {
    epoch: Option<Epoch>,
    previous: Option<Epoch>,
    signals: HashMap<(SV, Observable), f64>,
}

impl Latest {
    fn update(
        &mut self,
        epoch: Epoch,
        signals: &[SignalObservation],
        cfg: &QcObservationResidualConfig,
    ) {
        self.previous = self.epoch;
        self.epoch = Some(epoch);
        self.signals.clear();
        for signal in signals.iter() {
            if cfg.selected(&signal.observable) {
                self.signals
                    .insert((signal.sv, signal.observable.clone()), signal.value);
            }
        }
    }

    fn sampling(&self) -> Option<Duration> {
        Some(self.epoch? - self.previous?)
    }
}

/// Observation residuals between two receivers: for each epoch of receiver A, each
/// observation is differentiated with the observation of receiver B, for the same satellite
/// and the same observable (A - B), at the closest epoch of B within the tolerance.
/// Observations without counterpart are discarded.
///
/// This is the streamed counterpart of `Rinex::observations_substract`: it applies to files
/// as well as real-time streams.
#[derive(Debug, Default, Clone)]
pub struct QcObservationResidualProcessor {
    cfg: QcObservationResidualConfig,
    lhs: Option<QcIndexing>,
    rhs: Option<QcIndexing>,
    lhs_latest: Latest,
    rhs_latest: Latest,
    last_emitted: Option<Epoch>,
}

impl QcObservationResidualProcessor {
    /// Builds a new [QcObservationResidualProcessor]
    pub fn new(cfg: QcObservationResidualConfig) -> Self {
        Self {
            lhs: cfg.lhs.clone(),
            rhs: cfg.rhs.clone(),
            cfg,
            ..Default::default()
        }
    }

    fn select_receivers(&mut self, ctx: &QcEpochContext) {
        for rx in ctx.receivers {
            if self.lhs.is_none() && self.rhs.as_ref() != Some(&rx.receiver) {
                self.lhs = Some(rx.receiver.clone());
            } else if self.rhs.is_none() && self.lhs.as_ref() != Some(&rx.receiver) {
                self.rhs = Some(rx.receiver.clone());
            }
        }
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcObservationResidualProcessor {
    fn name(&self) -> &str {
        "observation-residuals"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        self.select_receivers(ctx);

        let (lhs, rhs) = match (&self.lhs, &self.rhs) {
            (Some(lhs), Some(rhs)) => (lhs.clone(), rhs.clone()),
            _ => return,
        };

        let mut updated = false;

        for rx in ctx.receivers {
            if rx.receiver == lhs {
                self.lhs_latest.update(ctx.epoch, &rx.signals, &self.cfg);
                updated = true;
            } else if rx.receiver == rhs {
                self.rhs_latest.update(ctx.epoch, &rx.signals, &self.cfg);
                updated = true;
            }
        }

        if !updated {
            return;
        }

        let (t_lhs, t_rhs) = match (self.lhs_latest.epoch, self.rhs_latest.epoch) {
            (Some(t_lhs), Some(t_rhs)) => (t_lhs, t_rhs),
            _ => return,
        };

        // one residual epoch per epoch of A
        if self.last_emitted == Some(t_lhs) {
            return;
        }

        let tolerance = self
            .cfg
            .tolerance
            .or(self.lhs_latest.sampling().map(|dt| dt / 2))
            .unwrap_or(Duration::ZERO);

        if (t_lhs - t_rhs).abs() > tolerance {
            return;
        }

        let mut keys = self.lhs_latest.signals.keys().collect::<Vec<_>>();
        keys.sort();

        for key in keys {
            if let Some(reference) = self.rhs_latest.signals.get(key) {
                let value = self.lhs_latest.signals[key];

                output.push(
                    t_lhs,
                    QcOutput::ObservationResidual(QcObservationResidual {
                        lhs: lhs.clone(),
                        rhs: rhs.clone(),
                        sv: key.0,
                        observable: key.1.clone(),
                        residual: value - reference,
                        rhs_epoch: t_rhs,
                    }),
                );
            }
        }

        self.last_emitted = Some(t_lhs);
    }
}
