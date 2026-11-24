//! IONEX file source.
use std::vec::IntoIter;

use ionex::prelude::IONEX;

use crate::io::{
    input::indexing::QcIndexing,
    source::{QcInputItem, QcPoll, QcSource, QcSourceDescriptor, QcSourceError, QcTecPoint},
};

/// TEC maps source from an [IONEX] file: every node of every map
/// (as [QcPayload::Tec](crate::io::source::QcPayload::Tec)), map after map,
/// in chronological order. Omitted nodes are not delivered.
pub struct QcIonexSource {
    descriptor: QcSourceDescriptor,
    items: IntoIter<QcInputItem>,
}

impl QcIonexSource {
    /// Builds a new [QcIonexSource], identified by `indexing` (usually the agency).
    pub fn new(indexing: QcIndexing, ionex: IONEX) -> Self {
        // the record is sorted by epoch, then coordinates
        let items = ionex
            .record
            .iter()
            .map(|(key, tec)| {
                QcInputItem::tec(
                    key.epoch,
                    QcTecPoint {
                        latitude_deg: key.latitude_ddeg(),
                        longitude_deg: key.longitude_ddeg(),
                        altitude_km: key.altitude_km(),
                        tecu: tec.tecu(),
                        rms_tecu: tec.root_mean_square(),
                    },
                )
            })
            .collect::<Vec<_>>();

        Self {
            descriptor: QcSourceDescriptor::new(indexing).with_tec_maps(),
            items: items.into_iter(),
        }
    }
}

impl QcSource for QcIonexSource {
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
