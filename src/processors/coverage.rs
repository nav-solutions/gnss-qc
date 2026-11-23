use std::collections::HashMap;

use gnss_rs::prelude::SV;
use hifitime::Epoch;

use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::{
        input::indexing::QcIndexing,
        output::{QcEphemerisCoverage, QcOutput, QcOutputQueue},
    },
};

type Selection = Option<(QcIndexing, Epoch)>;

/// Verifies that every satellite in sight is covered by a valid radio ephemeris,
/// for every receiver. Results are emitted on change only:
/// coverage gained, coverage lost, or switch to a new ephemeris.
#[derive(Debug, Default, Clone)]
pub struct QcEphemerisCoverageProcessor {
    states: HashMap<QcIndexing, HashMap<SV, Selection>>,
    in_sight: Vec<SV>,
}

impl<O: From<QcOutput>> QcProcessor<O> for QcEphemerisCoverageProcessor {
    fn name(&self) -> &str {
        "ephemeris-coverage"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        for rx in ctx.receivers {
            self.in_sight.clear();
            self.in_sight.extend(rx.signals.iter().map(|sig| sig.sv));
            self.in_sight.sort();
            self.in_sight.dedup();

            if !self.states.contains_key(&rx.receiver) {
                self.states.insert(rx.receiver.clone(), Default::default());
            }

            let states = self.states.get_mut(&rx.receiver).unwrap();

            for sv in self.in_sight.iter() {
                let selected = ctx.ephemerides.select(*sv, ctx.epoch);

                let changed = match (states.get(sv), selected) {
                    (None, _) => true,
                    (Some(None), None) => false,
                    (Some(Some((src, toc))), Some((new_src, eph))) => {
                        *toc != eph.toc || src != new_src
                    }
                    _ => true,
                };

                if changed {
                    let selection = selected.map(|(src, eph)| (src.clone(), eph.toc));

                    output.push(
                        ctx.epoch,
                        QcOutput::EphemerisCoverage(QcEphemerisCoverage {
                            receiver: rx.receiver.clone(),
                            sv: *sv,
                            source: selection.as_ref().map(|(src, _)| src.clone()),
                            toc: selection.as_ref().map(|(_, toc)| *toc),
                        }),
                    );

                    states.insert(*sv, selection);
                }
            }
        }
    }
}
