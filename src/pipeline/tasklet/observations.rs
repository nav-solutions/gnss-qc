use flume::Receiver;

use crate::{
    serializer::serializer::QcSerializedDataPoint,
    pipeline::QcTasklet,
    report::observations::QcSignalsObservationReport,
};

pub struct QcSignalObservationTask {
    name: String,
    /// [Receiver]
    rx: Receiver<QcSerializedDataPoint>,
}

impl QcSignalObservationTask {
    pub fn new(name: &str, rx: Receiver<QcSerializedDataPoint>) -> Self {
        Self {
            rx,
            name: name.to_string(),
        }
    }
}

impl QcTasklet for QcSignalObservationTask {
    type Output = QcSignalsObservationReport;

    fn run(&mut self) -> Self::Output {
        let mut report = QcSignalsObservationReport::new(&self.name);

        // consume all data points
        loop {
            match self.rx.recv() {
                Ok(data) => match data {
                    QcSerializedDataPoint::ObservationHeader(header) => {
                        if let Some(header) = header.content.obs.as_ref() {
                            report.time_of_first_obs = header.timeof_first_obs;
                            report.time_of_last_obs = header.timeof_last_obs;
                        }
                    },
                    QcSerializedDataPoint::SignalObservation(signal) => {
                        report.latch_observation(signal);
                    }
                    _ => {}
                },
                Err(_) => break,
            }
        }

        report
    }
}
