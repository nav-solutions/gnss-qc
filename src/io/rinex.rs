//! RINEX file sources.
//!
//! These sources take ownership of a parsed [Rinex] and *drain* its record
//! progressively: memory is released as data points are pulled by the engine.
//! When a streamed RINEX parser becomes available, only these adapters will change.
use std::collections::btree_map::IntoIter as BTreeIntoIter;

use std::collections::VecDeque;

use rinex::{
    clock::{ClockKey, ClockProfile, ClockType},
    meteo::MeteoKey,
    navigation::{NavFrame, NavKey},
    prelude::{
        obs::{ClockObservation, EpochFlag, ObsKey, Observations, SignalObservation},
        Rinex, RinexType,
    },
    record::Record,
};

use thiserror::Error;

use crate::geodesy::enu_to_ecef_vector;

use crate::io::{
    input::indexing::QcIndexing,
    source::{
        QcClockId, QcInputItem, QcPayload, QcPoll, QcPreciseClock, QcSource, QcSourceDescriptor,
        QcSourceError,
    },
};

/// Errors when converting a [Rinex] into a [QcSource].
#[derive(Debug, Error, PartialEq)]
pub enum QcRinexSourceError {
    #[error("not an observation RINEX ({0})")]
    NotObservation(RinexType),

    #[error("not a navigation RINEX ({0})")]
    NotNavigation(RinexType),

    #[error("not a clock RINEX ({0})")]
    NotClock(RinexType),

    #[error("not a meteo RINEX ({0})")]
    NotMeteo(RinexType),
}

/// Signal source from an Observation (or CRINEX) [Rinex].
/// Delivers, epoch after epoch, the receiver clock state (if any)
/// then all signal observations.
pub struct QcRinexObsSource {
    descriptor: QcSourceDescriptor,
    record: BTreeIntoIter<ObsKey, Observations>,
    current: Option<CurrentEpoch>,
}

struct CurrentEpoch {
    key: ObsKey,
    clock: Option<ClockObservation>,
    signals: std::vec::IntoIter<SignalObservation>,
}

impl QcRinexObsSource {
    /// Builds a new [QcRinexObsSource], identified by `indexing`.
    pub fn new(indexing: QcIndexing, rinex: Rinex) -> Result<Self, QcRinexSourceError> {
        let rinex_type = rinex.header.rinex_type;

        let mut descriptor = QcSourceDescriptor::new(indexing).with_signals();

        if let Some(arp) = Self::antenna_reference_point(&rinex) {
            descriptor = descriptor.with_reference_position_ecef_m(arp);
        }

        match rinex.record {
            Record::ObsRecord(record) => Ok(Self {
                descriptor,
                record: record.into_iter(),
                current: None,
            }),
            _ => Err(QcRinexSourceError::NotObservation(rinex_type)),
        }
    }

    /// Antenna Reference Point (ECEF m): marker position (header)
    /// plus antenna eccentricities (height, east, north), when described.
    pub fn antenna_reference_point(rinex: &Rinex) -> Option<(f64, f64, f64)> {
        let marker = rinex.header.rx_position?;

        let (h, e, n) = match &rinex.header.rcvr_antenna {
            Some(antenna) => (
                antenna.height.unwrap_or_default(),
                antenna.eastern.unwrap_or_default(),
                antenna.northern.unwrap_or_default(),
            ),
            None => (0.0, 0.0, 0.0),
        };

        let delta = enu_to_ecef_vector(marker, (e, n, h));
        Some((marker.0 + delta.0, marker.1 + delta.1, marker.2 + delta.2))
    }

    fn epoch_item(key: &ObsKey, flag: EpochFlag, signal: SignalObservation) -> QcInputItem {
        QcInputItem::signal(key.epoch, flag, signal)
    }
}

impl QcSource for QcRinexObsSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        loop {
            if let Some(current) = &mut self.current {
                if let Some(clock) = current.clock.take() {
                    return Ok(QcPoll::Ready(QcInputItem::receiver_clock(
                        current.key.epoch,
                        clock,
                    )));
                }

                if let Some(signal) = current.signals.next() {
                    return Ok(QcPoll::Ready(Self::epoch_item(
                        &current.key,
                        current.key.flag,
                        signal,
                    )));
                }

                self.current = None;
            }

            match self.record.next() {
                Some((key, observations)) => {
                    self.current = Some(CurrentEpoch {
                        key,
                        clock: observations.clock,
                        signals: observations.signals.into_iter(),
                    });
                }
                None => return Ok(QcPoll::EndOfStream),
            }
        }
    }
}

/// Ephemeris source from a Navigation [Rinex].
/// Only ephemeris frames are delivered (other frames are not supported yet),
/// in chronological order of ToC.
pub struct QcRinexNavSource {
    descriptor: QcSourceDescriptor,
    record: BTreeIntoIter<NavKey, NavFrame>,
}

impl QcRinexNavSource {
    /// Builds a new [QcRinexNavSource], identified by `indexing`.
    pub fn new(indexing: QcIndexing, rinex: Rinex) -> Result<Self, QcRinexSourceError> {
        let rinex_type = rinex.header.rinex_type;

        match rinex.record {
            Record::NavRecord(record) => Ok(Self {
                descriptor: QcSourceDescriptor::new(indexing).with_ephemerides(),
                record: record.into_iter(),
            }),
            _ => Err(QcRinexSourceError::NotNavigation(rinex_type)),
        }
    }
}

impl QcSource for QcRinexNavSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        for (key, frame) in self.record.by_ref() {
            if let NavFrame::EPH(ephemeris) = frame {
                return Ok(QcPoll::Ready(QcInputItem::ephemeris(
                    key.epoch, key.sv, ephemeris,
                )));
            }
        }

        Ok(QcPoll::EndOfStream)
    }
}

/// Precise clocks source from a Clock [Rinex].
/// Delivers, epoch after epoch, the state of every clock described by the file:
/// satellite clocks and ground station clocks (as [QcPayload::PreciseClock]).
pub struct QcRinexClockSource {
    descriptor: QcSourceDescriptor,
    record: BTreeIntoIter<hifitime::Epoch, std::collections::BTreeMap<ClockKey, ClockProfile>>,
    pending: VecDeque<QcInputItem>,
}

impl QcRinexClockSource {
    /// Builds a new [QcRinexClockSource], identified by `indexing` (usually the agency).
    pub fn new(indexing: QcIndexing, rinex: Rinex) -> Result<Self, QcRinexSourceError> {
        let rinex_type = rinex.header.rinex_type;

        match rinex.record {
            Record::ClockRecord(record) => Ok(Self {
                descriptor: QcSourceDescriptor::new(indexing).with_precise_clocks(),
                record: record.into_iter(),
                pending: VecDeque::new(),
            }),
            _ => Err(QcRinexSourceError::NotClock(rinex_type)),
        }
    }

    fn item(epoch: hifitime::Epoch, key: ClockKey, profile: ClockProfile) -> QcInputItem {
        let clock = match key.clock_type {
            ClockType::SV(sv) => QcClockId::Satellite(sv),
            ClockType::Station(name) => QcClockId::Station(name),
        };

        QcInputItem {
            epoch,
            payload: QcPayload::PreciseClock(QcPreciseClock {
                clock,
                profile: key.profile_type,
                epoch,
                offset_s: profile.bias,
                offset_dev_s: profile.bias_dev,
                drift_s_s: profile.drift,
            }),
        }
    }
}

impl QcSource for QcRinexClockSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        loop {
            if let Some(item) = self.pending.pop_front() {
                return Ok(QcPoll::Ready(item));
            }

            match self.record.next() {
                Some((epoch, clocks)) => {
                    self.pending.extend(
                        clocks
                            .into_iter()
                            .map(|(key, profile)| Self::item(epoch, key, profile)),
                    );
                }
                None => return Ok(QcPoll::EndOfStream),
            }
        }
    }
}

/// Meteo observations source from a Meteo [Rinex].
/// Delivers all sensor observations (as [QcPayload::Meteo]) in chronological order.
pub struct QcRinexMeteoSource {
    descriptor: QcSourceDescriptor,
    record: BTreeIntoIter<MeteoKey, f64>,
}

impl QcRinexMeteoSource {
    /// Builds a new [QcRinexMeteoSource], identified by `indexing`.
    pub fn new(indexing: QcIndexing, rinex: Rinex) -> Result<Self, QcRinexSourceError> {
        let rinex_type = rinex.header.rinex_type;

        match rinex.record {
            Record::MeteoRecord(record) => Ok(Self {
                descriptor: QcSourceDescriptor::new(indexing).with_meteo(),
                record: record.into_iter(),
            }),
            _ => Err(QcRinexSourceError::NotMeteo(rinex_type)),
        }
    }
}

impl QcSource for QcRinexMeteoSource {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        Ok(match self.record.next() {
            Some((key, value)) => {
                QcPoll::Ready(QcInputItem::meteo(key.epoch, key.observable, value))
            }
            None => QcPoll::EndOfStream,
        })
    }
}
