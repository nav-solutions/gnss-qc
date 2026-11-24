//! Clock residuals: A - B between two clock products (precise clock products,
//! clock states of precise products, broadcast clocks), from given agencies or sources.
use std::collections::{BTreeMap, BTreeSet};

use gnss_rs::prelude::Constellation;
use hifitime::{Duration, Epoch};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcClockResidual, QcOutput, QcOutputQueue},
        source::QcClockId,
    },
    orbit::{QcClockKind, QcKepler},
};

/// One side of a clock residual
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcClockProduct {
    /// Precise clock products (Clock RINEX) of this agency, or of any agency (None).
    /// Satellite and ground station clocks.
    Clock(Option<QcIndexing>),

    /// Clock states of the precise products (SP3) of this agency, or of any agency (None).
    /// Satellite clocks only.
    PreciseState(Option<QcIndexing>),

    /// Clocks of the radio messages (NAV) of this source, or of any source (None).
    /// Satellite clocks only.
    Broadcast(Option<QcIndexing>),
}

impl QcClockProduct {
    /// Kind of clock this product provides
    pub fn kind(&self) -> QcClockKind {
        match self {
            Self::Clock(_) => QcClockKind::PreciseClock,
            Self::PreciseState(_) => QcClockKind::PreciseState,
            Self::Broadcast(_) => QcClockKind::Broadcast,
        }
    }

    /// Source (agency) of this product, when specified
    pub fn source(&self) -> Option<&QcIndexing> {
        match self {
            Self::Clock(source) | Self::PreciseState(source) | Self::Broadcast(source) => {
                source.as_ref()
            }
        }
    }
}

/// How the clock datum of each product is handled
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum QcClockAlignment {
    /// Raw residuals only: each analysis center has its own clock datum (reference clock),
    /// so the raw residuals of two centers share a common offset, at each epoch,
    /// of a few nanoseconds.
    None,

    /// One datum per epoch: median of the satellite residuals of this epoch.
    Epoch,

    /// One datum per epoch and per constellation: median of the satellite residuals
    /// of this constellation (inter system biases may differ between centers).
    /// Station clocks are aligned with the GPS datum. Default.
    #[default]
    Constellation,
}

/// Clock residuals configuration
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct QcClockResidualConfig {
    /// Product A (minuend)
    pub lhs: QcClockProduct,

    /// Product B (subtrahend)
    pub rhs: QcClockProduct,

    /// Satellite clocks
    pub satellites: bool,

    /// Ground station clocks (precise clock products only)
    pub stations: bool,

    /// Minimal interval between two evaluations. Default: every epoch.
    pub sampling: Option<Duration>,

    /// Constellations. Default: all constellations.
    pub constellations: Option<Vec<Constellation>>,

    /// Clock datum alignment
    pub alignment: QcClockAlignment,

    /// Minimal number of satellites to estimate a datum
    pub min_datum_satellites: usize,
}

impl Default for QcClockResidualConfig {
    /// Precise clock products of any agency - precise products (SP3) of any agency
    fn default() -> Self {
        Self::new(
            QcClockProduct::Clock(None),
            QcClockProduct::PreciseState(None),
        )
    }
}

impl QcClockResidualConfig {
    /// A - B between these two products
    pub fn new(lhs: QcClockProduct, rhs: QcClockProduct) -> Self {
        Self {
            lhs,
            rhs,
            satellites: true,
            stations: true,
            sampling: None,
            constellations: None,
            alignment: QcClockAlignment::default(),
            min_datum_satellites: 3,
        }
    }

    /// Precise clock products (Clock RINEX) of two agencies: A - B
    pub fn clock_products(lhs: QcIndexing, rhs: QcIndexing) -> Self {
        Self::new(
            QcClockProduct::Clock(Some(lhs)),
            QcClockProduct::Clock(Some(rhs)),
        )
    }

    /// Minimal interval between two evaluations
    pub fn with_sampling(mut self, sampling: Duration) -> Self {
        self.sampling = Some(sampling);
        self
    }

    /// Restricts to these constellations
    pub fn with_constellations(mut self, constellations: &[Constellation]) -> Self {
        self.constellations = Some(constellations.to_vec());
        self
    }

    /// Clock datum alignment
    pub fn with_alignment(mut self, alignment: QcClockAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Satellite clocks only
    pub fn satellites_only(mut self) -> Self {
        self.satellites = true;
        self.stations = false;
        self
    }

    /// Ground station clocks only
    pub fn stations_only(mut self) -> Self {
        self.satellites = false;
        self.stations = true;
        self
    }
}

/// Clock offset (s) of `clock` at `t`, from this product: (source, offset)
fn resolve(
    ctx: &QcEpochContext,
    product: &QcClockProduct,
    clock: &QcClockId,
    t: Epoch,
) -> Option<(QcIndexing, f64)> {
    match product {
        QcClockProduct::Clock(Some(source)) => Some((
            source.clone(),
            ctx.precise_clocks.interpolate_from(source, clock, t)?,
        )),
        QcClockProduct::Clock(None) => {
            let (source, offset) = ctx.precise_clocks.interpolate(clock, t)?;
            Some((source.clone(), offset))
        }
        QcClockProduct::PreciseState(source) => {
            let sv = clock.as_satellite()?;
            let (source, state) = match source {
                Some(source) => (
                    source.clone(),
                    ctx.precise_states.interpolate_from(source, sv, t)?,
                ),
                None => {
                    let (source, state) = ctx.precise_states.interpolate(sv, t)?;
                    (source.clone(), state)
                }
            };
            Some((source, state.clock_offset_s?))
        }
        QcClockProduct::Broadcast(source) => {
            let sv = clock.as_satellite()?;
            let (source, eph) = match source {
                Some(source) => (source.clone(), ctx.ephemerides.select_from(source, sv, t)?),
                None => {
                    let (source, eph) = ctx.ephemerides.select(sv, t)?;
                    (source.clone(), eph)
                }
            };
            let kepler = QcKepler::from_ephemeris(sv, eph.toc, &eph.ephemeris)?;
            // same definition as the precise clocks: no relativistic correction
            Some((source, kepler.clock_offset_s(t)))
        }
    }
}

/// Clocks described by this product at this epoch
fn clocks(ctx: &QcEpochContext, product: &QcClockProduct) -> BTreeSet<QcClockId> {
    let mut clocks = BTreeSet::new();

    match product {
        QcClockProduct::Clock(source) => {
            for s in ctx.precise_clocks.sources() {
                if source.as_ref().is_none_or(|source| source == s) {
                    clocks.extend(ctx.precise_clocks.clocks_of(s));
                }
            }
        }
        QcClockProduct::PreciseState(source) => {
            for s in ctx.precise_states.sources() {
                if source.as_ref().is_none_or(|source| source == s) {
                    clocks.extend(
                        ctx.precise_states
                            .satellites_of(s)
                            .into_iter()
                            .map(QcClockId::Satellite),
                    );
                }
            }
        }
        QcClockProduct::Broadcast(source) => {
            for s in ctx.ephemerides.sources() {
                if source.as_ref().is_none_or(|source| source == s) {
                    clocks.extend(
                        ctx.ephemerides
                            .satellites_of(s)
                            .into_iter()
                            .map(QcClockId::Satellite),
                    );
                }
            }
        }
    }

    clocks
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = values.len();

    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    })
}

/// Differentiates the clocks described by two products (A - B): satellite clocks,
/// ground station clocks, at every epoch (or at the requested sampling) where both
/// products describe the clock (no extrapolation). Precise clocks and SP3 clock states
/// are linearly interpolated, broadcast clocks are evaluated from their polynomial.
///
/// Each analysis center has its own clock datum: the raw residuals of two centers
/// share a common offset at each epoch. This datum is estimated (median of the
/// satellite residuals) and reported with each residual, see [QcClockAlignment].
#[derive(Debug, Clone)]
pub struct QcClockResidualProcessor {
    cfg: QcClockResidualConfig,
    last: Option<Epoch>,
}

impl QcClockResidualProcessor {
    /// New [QcClockResidualProcessor]
    pub fn new(cfg: QcClockResidualConfig) -> Self {
        Self { cfg, last: None }
    }
}

impl Default for QcClockResidualProcessor {
    fn default() -> Self {
        Self::new(QcClockResidualConfig::default())
    }
}

impl<O: From<QcOutput>> QcProcessor<O> for QcClockResidualProcessor {
    fn name(&self) -> &str {
        "clock-residuals"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        let t = ctx.epoch;

        if let (Some(last), Some(sampling)) = (self.last, self.cfg.sampling) {
            if t - last < sampling {
                return;
            }
        }

        let lhs_clocks = clocks(ctx, &self.cfg.lhs);
        let rhs_clocks = clocks(ctx, &self.cfg.rhs);

        let mut residuals = Vec::new();

        for clock in lhs_clocks.intersection(&rhs_clocks) {
            match clock {
                QcClockId::Satellite(sv) => {
                    if !self.cfg.satellites {
                        continue;
                    }
                    if let Some(constellations) = &self.cfg.constellations {
                        if !constellations.contains(&sv.constellation) {
                            continue;
                        }
                    }
                }
                QcClockId::Station(_) => {
                    if !self.cfg.stations {
                        continue;
                    }
                }
            }

            let (Some((lhs_source, a)), Some((rhs_source, b))) = (
                resolve(ctx, &self.cfg.lhs, clock, t),
                resolve(ctx, &self.cfg.rhs, clock, t),
            ) else {
                continue;
            };

            // a product compared to itself
            if lhs_source == rhs_source && self.cfg.lhs.kind() == self.cfg.rhs.kind() {
                continue;
            }

            residuals.push((clock.clone(), lhs_source, rhs_source, a - b));
        }

        if residuals.is_empty() {
            return;
        }

        self.last = Some(t);

        // clock datum
        let mut per_constellation = BTreeMap::<Constellation, Vec<f64>>::new();
        let mut all = Vec::new();

        for (clock, _, _, residual) in residuals.iter() {
            if let Some(sv) = clock.as_satellite() {
                per_constellation
                    .entry(sv.constellation)
                    .or_default()
                    .push(*residual);
                all.push(*residual);
            }
        }

        let min = self.cfg.min_datum_satellites.max(1);

        let epoch_datum = if all.len() >= min {
            median(&mut all)
        } else {
            None
        };

        let constellation_datum = per_constellation
            .into_iter()
            .filter(|(_, values)| values.len() >= min)
            .filter_map(|(c, mut values)| Some((c, median(&mut values)?)))
            .collect::<BTreeMap<_, _>>();

        let datum = |clock: &QcClockId| -> Option<f64> {
            match self.cfg.alignment {
                QcClockAlignment::None => None,
                QcClockAlignment::Epoch => epoch_datum,
                QcClockAlignment::Constellation => match clock {
                    QcClockId::Satellite(sv) => constellation_datum.get(&sv.constellation).copied(),
                    QcClockId::Station(_) => constellation_datum.get(&Constellation::GPS).copied(),
                },
            }
        };

        for (clock, lhs_source, rhs_source, residual_s) in residuals {
            let datum_s = datum(&clock);

            output.push(
                t,
                QcOutput::ClockResidual(QcClockResidual {
                    datum_s,
                    clock,
                    lhs_source,
                    lhs_kind: self.cfg.lhs.kind(),
                    rhs_source,
                    rhs_kind: self.cfg.rhs.kind(),
                    residual_s,
                }),
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::median;

    #[test]
    fn medians() {
        assert_eq!(median(&mut []), None);
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&mut [4.0, 1.0, 2.0, 3.0]), Some(2.5));
    }
}
