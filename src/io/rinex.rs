//! RINEX file sources.
//!
//! These sources take ownership of a parsed [Rinex] and *drain* its record
//! progressively: memory is released as data points are pulled by the engine.
//! When a streamed RINEX parser becomes available, only these adapters will change.
use std::collections::btree_map::IntoIter as BTreeIntoIter;

use rinex::{
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
    source::{QcInputItem, QcPoll, QcSource, QcSourceDescriptor, QcSourceError},
};

/// Errors when converting a [Rinex] into a [QcSource].
#[derive(Debug, Error, PartialEq)]
pub enum QcRinexSourceError {
    #[error("not an observation RINEX ({0})")]
    NotObservation(RinexType),

    #[error("not a navigation RINEX ({0})")]
    NotNavigation(RinexType),
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
