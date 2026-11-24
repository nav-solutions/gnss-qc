//! Precise clocks and meteo observations passthrough.
use std::collections::HashMap;

use hifitime::{Duration, Epoch};

use rinex::prelude::Observable;

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcMeteoOutput, QcOutput, QcOutputQueue, QcPreciseClockOutput},
        source::QcClockId,
    },
};

/// [QcPreciseClockProcessor] configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcPreciseClockConfig {
    /// Stream the onboard clocks of the satellites
    pub satellites: bool,

    /// Stream the clocks of the ground stations
    pub stations: bool,

    /// Minimal interval between two states of the same clock (decimation).
    /// Every state by default.
    pub sampling: Option<Duration>,
}

impl Default for QcPreciseClockConfig {
    fn default() -> Self {
        Self {
            satellites: true,
            stations: true,
            sampling: None,
        }
    }
}

impl QcPreciseClockConfig {
    /// Satellite clocks only
    pub fn satellites_only() -> Self {
        Self {
            stations: false,
            ..Default::default()
        }
    }

    /// Ground station clocks only
    pub fn stations_only() -> Self {
        Self {
            satellites: false,
            ..Default::default()
        }
    }

    /// Decimates the clock states
    pub fn with_sampling(mut self, sampling: Duration) -> Self {
        self.sampling = Some(sampling);
        self
    }
}

/// Streams the precise clock states (Clock RINEX analysis results: satellites `AS`,
/// ground stations `AR`) out, once, in chronological order of each clock.
/// Results are time tagged with the epoch of the clock state.
#[derive(Debug, Default, Clone)]
pub struct QcPreciseClockProcessor {
    cfg: QcPreciseClockConfig,
    emitted: HashMap<(QcIndexing, QcClockId), Epoch>,
}

impl QcPreciseClockProcessor {
    /// New [QcPreciseClockProcessor]
    pub fn new(cfg: QcPreciseClockConfig) -> Self {
        Self {
            cfg,
            emitted: Default::default(),
        }
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcPreciseClockProcessor {
    fn name(&self) -> &str {
        "precise-clocks"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let mut sources = ctx.precise_clocks.sources().cloned().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string());

        let mut states = Vec::new();

        for source in sources {
            for clock in ctx.precise_clocks.clocks_of(&source) {
                let selected = match clock {
                    QcClockId::Satellite(_) => self.cfg.satellites,
                    QcClockId::Station(_) => self.cfg.stations,
                };

                if !selected {
                    continue;
                }

                let key = (source.clone(), clock.clone());
                let mut last = self.emitted.get(&key).copied();

                for state in ctx
                    .precise_clocks
                    .samples(&source, &clock)
                    .into_iter()
                    .flatten()
                {
                    if state.epoch > ctx.epoch {
                        break;
                    }

                    let due = match (last, self.cfg.sampling) {
                        (None, _) => true,
                        (Some(last), None) => state.epoch > last,
                        (Some(last), Some(dt)) => state.epoch >= last + dt,
                    };

                    if due {
                        states.push(QcPreciseClockOutput {
                            source: source.clone(),
                            state: state.clone(),
                        });
                        last = Some(state.epoch);
                    }
                }

                if let Some(last) = last {
                    self.emitted.insert(key, last);
                }
            }
        }

        states.sort_by(|a, b| {
            (a.state.epoch, &a.source.to_string(), &a.state.clock).cmp(&(
                b.state.epoch,
                &b.source.to_string(),
                &b.state.clock,
            ))
        });

        for state in states {
            output.push(state.state.epoch, QcOutput::PreciseClock(state));
        }
    }
}

/// [QcMeteoProcessor] configuration
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcMeteoConfig {
    /// Observables to stream, all of them by default
    pub observables: Option<Vec<Observable>>,
}

impl QcMeteoConfig {
    /// Streams these observables only
    pub fn with_observables(mut self, observables: &[Observable]) -> Self {
        self.observables = Some(observables.to_vec());
        self
    }
}

/// Streams the meteo observations (Meteo RINEX, meteo sensors) out, once,
/// in chronological order of each observable.
/// Results are time tagged with the epoch of the observation.
#[derive(Debug, Default, Clone)]
pub struct QcMeteoProcessor {
    cfg: QcMeteoConfig,
    emitted: HashMap<(QcIndexing, Observable), Epoch>,
}

impl QcMeteoProcessor {
    /// New [QcMeteoProcessor]
    pub fn new(cfg: QcMeteoConfig) -> Self {
        Self {
            cfg,
            emitted: Default::default(),
        }
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcMeteoProcessor {
    fn name(&self) -> &str {
        "meteo"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let mut sources = ctx.meteo.sources().cloned().collect::<Vec<_>>();
        sources.sort_by_key(|s| s.to_string());

        let mut samples = Vec::new();

        for source in sources {
            for observable in ctx.meteo.observables_of(&source) {
                if let Some(selected) = &self.cfg.observables {
                    if !selected.contains(&observable) {
                        continue;
                    }
                }

                let key = (source.clone(), observable.clone());
                let mut last = self.emitted.get(&key).copied();

                for (epoch, value) in ctx
                    .meteo
                    .samples(&source, &observable)
                    .into_iter()
                    .flatten()
                {
                    if *epoch > ctx.epoch {
                        break;
                    }

                    if last.is_none_or(|last| *epoch > last) {
                        samples.push((
                            *epoch,
                            QcMeteoOutput {
                                source: source.clone(),
                                observable: observable.clone(),
                                value: *value,
                            },
                        ));
                        last = Some(*epoch);
                    }
                }

                if let Some(last) = last {
                    self.emitted.insert(key, last);
                }
            }
        }

        samples.sort_by(|a, b| {
            (a.0, a.1.source.to_string(), &a.1.observable).cmp(&(
                b.0,
                b.1.source.to_string(),
                &b.1.observable,
            ))
        });

        for (epoch, sample) in samples {
            output.push(epoch, QcOutput::Meteo(sample));
        }
    }
}
