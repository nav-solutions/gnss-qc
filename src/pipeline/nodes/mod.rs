use log::error;

use crossbeam_channel::{Receiver, Sender};

mod ephemeris;
mod filters;
mod observations;

/// [Node] defines an abstract box with a single input [Receiver] port
/// and a single output [Sender] port.
pub trait Node<I, O> {
    /// Readable name
    fn name(&self) -> &str;

    /// Mutable [Receiver] handle
    fn receiver(&mut self) -> &mut Receiver<I>;

    /// Mutable [Sender] handle
    fn sender(&mut self) -> &mut Sender<O>;

    fn task(&mut self, input: I) -> Option<O>;

    /// Deploy & Execute this [Node] task
    fn run(&mut self) {
        loop {
            match self.receiver().recv() {
                Ok(value) => {
                    if let Some(output) = self.task(value) {
                        // post results
                        match self.sender().send(output) {
                            Ok(_) => {}
                            Err(e) => {
                                error!("{} - failed to propagate data: {}", self.name(), e);
                            }
                        }
                    }
                }
                Err(_) => {
                    // a message could not be received because the channel is disconnected.
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::{
        pipeline::nodes::ephemeris::QcEphemerisStreamer,
        pipeline::nodes::observations::QcObservationsStreamer, pipeline::nodes::Node,
        prelude::QcContext, tests::init_logger,
    };

    #[test]
    fn pipeline_test() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();
        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        let mut serializer = ctx.serializer();

        // let n_workers = 4;
        // let mut pool = ThreadPool::new(n_workers);

        let (serializer_tx, entrypoints_rx) = crossbeam_channel::unbounded();
        let (obs_tx, obs_rx) = crossbeam_channel::unbounded();
        let (ephemeris_tx, ephemeris_rx) = crossbeam_channel::unbounded();

        let mut ephemeris_streamer =
            QcEphemerisStreamer::new("eph-streamer", entrypoints_rx.clone(), ephemeris_tx);

        let mut obs_streamer =
            QcObservationsStreamer::new("obs-streamer", entrypoints_rx.clone(), obs_tx);

        let eph_tasklet = std::thread::spawn(move || {
            ephemeris_streamer.run();
        });

        let obs_tasklet = std::thread::spawn(move || {
            obs_streamer.run();
        });

        let eph_watcher = std::thread::spawn(move || loop {
            match ephemeris_rx.recv() {
                Ok(value) => {
                    info!(
                        "received ephemeris: {}-{} toc={} toe={}",
                        value.filename, value.indexing, value.data.toc, value.data.toe,
                    );
                }
                Err(_) => break,
            }
        });

        let obs_watcher = std::thread::spawn(move || loop {
            match obs_rx.recv() {
                Ok(value) => {
                    info!(
                        "received observation: {}-{} {} {:?}",
                        value.filename, value.indexing, value.data.carrier, value.data.observation
                    );
                }
                Err(_) => break,
            }
        });

        // stream data in
        while let Some(data) = serializer.next() {
            let _ = serializer_tx.send(data);
        }

        drop(serializer_tx);

        eph_tasklet.join().unwrap();
        obs_tasklet.join().unwrap();
        eph_watcher.join().unwrap();
        obs_watcher.join().unwrap();
    }
}
