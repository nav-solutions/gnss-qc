use crate::{
    context::{QcContext, QcIndexing},
    error::QcError,
    prelude::Epoch,
    report::QcRunReport,
    serializer::{
        data::{QcSerializedItem, QcSerializedSignal, QcSignalData},
        serializer::QcSerializer,
    },
};

use hifitime::Duration;
use std::sync::{Arc, Mutex};

use qc_traits::MaskFilter;

use tokio::{
    sync::{
        broadcast::{
            channel as broadcast_channel, error::RecvError, Receiver as BroadcastReceiver,
            Sender as BroadcastSender,
        },
        mpsc::{channel as mpsc_channel, Receiver, Sender},
        RwLock,
    },
    task::spawn_blocking,
};

use log::debug;

// pub mod node;
// pub mod tx_buffer;

// mod tasklet;
// use tasklet::{observations::QcSignalObservationTask, QcTasklet};

// use flume::Receiver;

/// Generic [QcPipelineNode] that can receive synchronous data,
/// process it and stream data out.
pub trait Node {
    /// Read a new data point
    fn read(&mut self);

    /// Process data point, producing a result
    fn process(&mut self);
}

/// [QcObservationsDeserializer] deserializes an observation stream from a unique source!
pub struct QcObsExtractor {
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

pub struct QcRunReporter {
    /// Deployment datetime
    pub datetime: Epoch,

    /// Path 1 RX handle
    pub rx: Receiver<QcSerializedSignal>,

    /// Report being redacted
    pub report: Arc<Mutex<QcRunReport>>,
}

impl QcRunReporter {
    pub fn run(&mut self) {
        loop {
            // retrieve all contributions
            match self.rx.blocking_recv() {
                Some(item) => {
                    let mut report = self.report.lock().unwrap();
                    report.add_signal_contribution(item);
                }
                None => break,
            }
        }
        debug!("run reporting - completed");
    }
}

/// [QcPipeline], deployed according to user specs
/// and containing several tasklets (=algorithm) to be executed.
pub struct QcPipeline<'a> {
    /// [QcSerializer] to provide the data stream
    serializer: QcSerializer<'a>,
}

impl QcContext {
    /// Form a [QcPipeline], following user specifications (desired tasks and algorithms to deploy),
    /// ready to execute. You then have two execution scenarios, depending on your context and ecosystem
    /// capabilities:
    /// - serial: execute one task after the other by using the proposed [Iterator].
    /// The [QcPipeline] job is done when all tasks were performed sequentially.
    /// - multithreaded: tasks are performed in parallal, using tokio framework.
    pub fn qc_processing_pipeline<'a>(&'a self) -> QcPipeline<'a> {
        let serializer = self.serializer();

        QcPipeline { serializer }
    }
}

impl<'a> QcPipeline<'a> {
    /// Execute this [QcPipeline] asynchronously, deploying a dedicated tasklet for each job.
    /// Otherwise, prefer the serial / synchronous execution, using the proposed [Iterator].
    pub async fn run(&mut self) -> Result<QcRunReport, QcError> {
        debug!("channels creation..");

        let (brdc_tx, _) = broadcast_channel(16);
        let (path_tx, path_rx) = mpsc_channel(16);

        // build
        debug!("job definitions..");
        let mut pool = Vec::new();

        let now = Epoch::now().unwrap_or_else(|e| {
            error!("failed to report system time: {}", e);
            Epoch::default()
        });

        // example: Obs source extractor
        let mut obs_task = QcObsExtractor {
            tx: path_tx,
            rx: brdc_tx.subscribe(),
            source_filter: QcIndexing::GeodeticMarker("VLNS0630".to_string()),
        };

        let mut report = QcRunReport::default();

        report.run_summary.num_jobs = 1;
        report.run_summary.datetime = now.round(Duration::from_seconds(1.0));

        let report = Arc::new(Mutex::new(report));

        // Run reporter
        let mut reporter = QcRunReporter {
            rx: path_rx,
            datetime: now,
            report: Arc::clone(&report),
        };

        // spawn
        debug!("deployment..");

        pool.push(tokio::task::spawn_blocking(move || {
            obs_task.run();
        }));

        pool.push(tokio::task::spawn_blocking(move || {
            reporter.run();
        }));

        // serialize all data
        loop {
            match self.serializer.next() {
                Some(data) => match brdc_tx.send(data) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("send failure: {}", e);
                        break;
                    }
                },
                None => break,
            }

            tokio::time::sleep(std::time::Duration::from_micros(100)).await;
        }

        // will conclude all tasklets
        drop(brdc_tx);

        for thread in pool {
            thread.await.unwrap();
        }

        let mut report = Arc::try_unwrap(report)
            .unwrap_or_else(|_| {
                panic!("failed to obtained redacted report!");
            })
            .into_inner()
            .unwrap_or_else(|e| {
                panic!("failed to obtain redacted report! {}", e);
            });

        let now = Epoch::now().unwrap_or_else(|e| {
            error!("failed to report system time: {}", e);
            Epoch::default()
        });

        report.run_summary.run_duration = now - report.run_summary.datetime;

        info!("pipeline executed sucessfully!");
        Ok(report)
    }
}

#[cfg(test)]
mod test {

    use crate::{prelude::QcContext, tests::init_logger};

    #[tokio::test]
    async fn basic_pipeline() {
        init_logger();

        let mut ctx = QcContext::new();

        // // load data
        // ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
        //     .unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();
        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        let mut pipeline = ctx.qc_processing_pipeline();

        let _ = pipeline.run().await;
    }
}
