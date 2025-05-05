use crate::{
    // context::QcContext,
    error::QcError,
    // report::QcRunReport,
    // serializer::{
    //     ephemeris::QcEphemerisData,
    //     serializer::{QcSerializedDataPoint, QcSerializer},
    //     signal::QcSignalDataPoint,
    // },
};

#[cfg(feature = "multi-threading")]
use tokio::{
    sync::broadcast::Sender as BroadcastSender,
    task::spawn_blocking,
};

use log::debug;

pub mod node;
pub mod tx_buffer;

pub mod job;
use job::QcJob;

mod tasklet;
use tasklet::{observations::QcSignalObservationTask, QcTasklet};

/// [QcPipeline], deployed according to user specs
/// and containing several tasklets (=algorithm) to be executed.
pub struct QcPipeline<'a> {
    /// [QcSerializer] to pull data from
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
    #[cfg(feature = "multi-threading")]
    pub async fn run(&mut self) -> Result<(), QcError> {

        // build
        debug!("jobs definition..");
        let (tx, rx) = flume::bounded(256);
        let mut test_observation_task = QcSignalObservationTask::new("Test", rx);

        // spawn
        debug!("deploying");

        tokio::task::spawn_blocking(move || {
            let observation_report = test_observation_task.run();
            info!("pipeline completed");
        });

        // forward all data points
        loop {
            match self.serializer.next() {
                Some(data) => match tx.send(data) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("send failure: {}", e);
                    }
                },
                None => break,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::{
        pipeline::{QcJob, QcPipeline},
        prelude::QcContext,
        tests::init_logger,
    };

    #[tokio::test]
    async fn basic_pipeline() {
        init_logger();

        let mut ctx = QcContext::new();

        // load NAV
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        // load data
        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        let mut pipeline = ctx.qc_processing_pipeline();

        let _ = pipeline.run().await;
    }
}
