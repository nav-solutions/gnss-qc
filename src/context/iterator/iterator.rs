use crate::prelude::{QcContext, Epoch};

use super::{ephemeris::QcEphemerisData, State};

// Synchronous [QcContext] Iterator
pub struct QcContextIterator<'a> {
    state: State,
    current_t: Epoch,
    ephemeris_buffer: QcEphemerisBuffer<'a>,
}

impl QcContext {
    pub fn iterator<'a>(&'a self) -> QcContextIterator<'a> {
        QcContextIterator {
            state: Default::default(),
            ephemeris_iter: if let Some(brdc) = &self.brdc_navigation {
                QcSynchronousIterator::new(Box::new(brdc.nav_ephemeris_frames_iter().filter_map(
                    |(k, v)| {
                        let sv_ts = k.sv.constellation.timescale()?;
                        let toe = v.toe(sv_ts)?;
                        Some(QcEphemerisData {
                            toe,
                            toc: k.epoch,
                            sv: k.sv,
                            ephemeris: v.clone(),
                        })
                    },
                )))
            } else {
                QcSynchronousIterator::null()
            },
        }
    }
}

impl<'a> Iterator for QcContextIterator<'a> {

    type Item = None;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Ephemeris => {
                let next = self.ephemeris_iter.next();
            }
        }
    }

}
