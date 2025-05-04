use crate::pipeline::QcJob;
use flume::Receiver;
use std::future::Future;

pub struct QcTasklet {
    pub job: QcJob,
}

impl QcTasklet {
    pub fn new(job: QcJob) -> Self {
        Self { job }
    }

    // #[cfg(feature = "tokio")]
    // pub fn spawn(&self) -> impl Future<Output = f64> {

    // }
}
