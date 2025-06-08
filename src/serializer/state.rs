#[derive(Debug, Default, Copy, Clone)]
pub enum State {
    #[default]
    /// Serialize all contants and atemporal products
    Constants,

    /// Stream all header information
    RINEXHeaders,

    #[cfg(feature = "sp3")]
    /// Stream all header information
    SP3Header,

    /// Streaming temporal data
    Temporal(TemporalState),

    /// Done streaming
    Done,
}

#[derive(Debug, Default, Copy, Clone)]
pub enum TemporalState {
    #[default]
    /// [TemporalState::Ephemeris] when streaming a new Ephemeris message
    Ephemeris,

    /// [TemporalState::Observations] when streaming a new signal observation.
    Observations,

    #[cfg(feature = "sp3")]
    /// [TemporalState::Observations] when streaming a new precise state.
    PreciseOrbits,
}
