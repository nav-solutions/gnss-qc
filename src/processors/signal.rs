use crate::{
    engine::{QcEpochContext, QcProcessor},
    io::output::{QcOutput, QcOutputQueue, QcSignalOutput},
};

/// Streams all signal observations out, as is, for all receivers.
#[derive(Debug, Default, Clone, Copy)]
pub struct QcSignalProcessor;

impl<O: From<QcOutput>> QcProcessor<O> for QcSignalProcessor {
    fn name(&self) -> &str {
        "signals"
    }

    fn process(&mut self, ctx: &QcEpochContext, output: &mut QcOutputQueue<O>) {
        for rx in ctx.receivers {
            for signal in rx.signals.iter() {
                output.push(
                    ctx.epoch,
                    QcOutput::Signal(QcSignalOutput {
                        receiver: rx.receiver.clone(),
                        flag: rx.flag,
                        signal: signal.clone(),
                    }),
                );
            }
        }
    }
}
