use std::collections::{HashMap, HashSet};

use gnss_rs::prelude::SV;
use hifitime::{Duration, Epoch};

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcEphemerisOutput, QcOutput, QcOutputQueue},
    },
};

/// Streams every radio ephemeris out, once, as soon as it is available to the engine.
/// The output epoch is the epoch being processed when the ephemeris was first seen
/// (the ToC is part of the output).
#[derive(Debug, Default, Clone)]
pub struct QcEphemerisProcessor {
    emitted: HashMap<QcIndexing, HashSet<(SV, Epoch)>>,
}

impl<O: From<QcOutput>> QcProcessor<O> for QcEphemerisProcessor {
    fn name(&self) -> &str {
        "ephemerides"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        for (source, eph) in ctx.ephemerides.iter() {
            let key = (eph.sv, eph.toc);

            if self
                .emitted
                .get(source)
                .is_some_and(|emitted| emitted.contains(&key))
            {
                continue;
            }

            output.push(
                ctx.epoch,
                QcOutput::Ephemeris(QcEphemerisOutput {
                    source: source.clone(),
                    sv: eph.sv,
                    toc: eph.toc,
                    ephemeris: eph.ephemeris.clone(),
                }),
            );

            self.emitted.entry(source.clone()).or_default().insert(key);
        }

        // bounded memory: forget ephemerides that can no longer be buffered
        let horizon = ctx.epoch - Duration::from_days(2.0);
        for emitted in self.emitted.values_mut() {
            emitted.retain(|(_, toc)| *toc > horizon);
        }
    }
}
