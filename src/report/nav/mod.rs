pub mod elev_snr;
pub mod navi_graph;

use elev_snr::QcElevationSNRReport;
use navi_graph::QcNaviGraph;

use crate::serializer::data::{QcSerializedEphemeris, QcSerializedSignal};

#[derive(Clone, Default)]
pub struct QcNavReport {
    /// Elevation + SNR report
    pub elev_snr: QcElevationSNRReport,

    /// NAVI graph
    pub navi_graph: QcNaviGraph,
}

impl QcNavReport {
    pub fn add_signal_contribution(&mut self, signal: &QcSerializedSignal) {
        self.elev_snr.add_signal_contribution(signal);
        self.navi_graph.add_signal_contribution(signal);
    }

    pub fn add_ephemeris_message(&mut self, msg: &QcSerializedEphemeris) {
        self.elev_snr.add_ephemeris_message(msg);
        self.navi_graph.add_ephemeris_message(msg);
    }
}
