use std::vec::Drain;

pub trait Job {
    /// [Job::Input] type
    type Input;

    /// [Job::Output] type
    type Output;

    /// Process number of available [Job::Input] symbols, possibly returning an [Job::Output]
    fn process(&mut self, size: usize, samples: Drain<Self::Input>) -> Option<Self::Output>;
}

/// [QcObsSyncBuffer]
pub struct QcObsSyncBuffer {

    /// Source filter
    pub source_filter: QcIndexing,

    /// RX handle
    pub rx: BroadcastReceiver<QcSerializedItem>,

    /// TX handle
    pub tx: Sender<QcSerializedSignal>,
}

impl QcObsExtractor {
    pub fn run(&mut self) {
        loop {
            match self.rx.blocking_recv() {
                Ok(QcSerializedItem::Signal(signal)) => {
                    if signal.indexing == self.source_filter {
                        match self.tx.blocking_send(signal) {
                            Ok(_) => {}
                            Err(e) => {
                                error!(
                                    "{} (obs-task) - send error {} (=data loss!)",
                                    self.source_filter, e
                                );
                            }
                        }
                    }
                }
                Err(RecvError::Closed) => {
                    // channel is closed, all points forwarded
                    break;
                }
                Err(RecvError::Lagged(lost)) => {
                    error!(
                        "{} (obs-task) - queue overflow {} messages lost",
                        self.source_filter, lost
                    );
                }
                _ => {} // filtered out
            }
        }
        debug!("{} (obs-task) - completed", self.source_filter);
    }
}