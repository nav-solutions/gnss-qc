//! SP3 file source.
use std::vec::IntoIter;

use sp3::prelude::SP3;

use crate::io::{
    input::indexing::QcIndexing,
    source::{QcInputItem, QcPoll, QcSource, QcSourceDescriptor, QcSourceError},
};

/// Precise states source from an [SP3] file.
/// SP3 records are indexed by satellite first: they are sorted in
/// chronological order here, so the engine receives a time ordered stream.
pub struct QcSp3Source {
    descriptor: QcSourceDescriptor,
    items: IntoIter<QcInputItem>,
}

impl QcSp3Source {
    /// Builds a new [QcSp3Source], identified by `indexing` (usually the agency).
    pub fn new(indexing: QcIndexing, sp3: SP3) -> Self {
        let mut items = sp3
            .data
            .into_iter()
            .filter(|(_, entry)| {
                // missing positions are encoded as null vectors
                let (x, y, z) = entry.position_km;
                x != 0.0 || y != 0.0 || z != 0.0
            })
            .map(|(key, entry)| {
                QcInputItem::precise_state(
                    key.epoch,
                    key.sv,
                    entry.position_km,
                    entry.clock_us.map(|us| us * 1.0E-6),
                )
            })
            .collect::<Vec<_>>();

        items.sort_by_key(|item| item.epoch);

        Self {
            descriptor: QcSourceDescriptor::new(indexing).with_precise_states(),
            items: items.into_iter(),
        }
    }
}

impl QcSource for QcSp3Source {
    fn descriptor(&self) -> &QcSourceDescriptor {
        &self.descriptor
    }

    fn poll(&mut self) -> Result<QcPoll<QcInputItem>, QcSourceError> {
        Ok(match self.items.next() {
            Some(item) => QcPoll::Ready(item),
            None => QcPoll::EndOfStream,
        })
    }
}
