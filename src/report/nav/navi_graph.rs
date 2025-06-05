use std::collections::HashMap;

use rinex::{
    carrier::Carrier,
    prelude::{Constellation, Epoch, SV},
};

use crate::serializer::data::{QcSerializedEphemeris, QcSerializedSignal};

#[derive(Clone, Default)]
pub struct QcNaviSVNavMessage {
    /// [Epoch] of publication
    pub epoch: Epoch,
}

#[derive(Clone, Default)]
pub struct QcNaviConstellGraph {
    pub sv_nav_messages: HashMap<SV, Vec<QcNaviSVNavMessage>>,
}

impl QcNaviConstellGraph {
    pub fn add_signal_contribution(&mut self, signal: &QcSerializedSignal) {}

    pub fn add_ephemeris_message(&mut self, msg: &QcSerializedEphemeris) {}
}

#[derive(Clone, Default)]
pub struct QcNaviGraph {
    /// [QcNaviConstellGraph] per [Constellation]
    pub constellations: HashMap<Constellation, QcNaviConstellGraph>,
}

impl QcNaviGraph {
    pub fn add_signal_contribution(&mut self, signal: &QcSerializedSignal) {}

    pub fn add_ephemeris_message(&mut self, msg: &QcSerializedEphemeris) {}
}
