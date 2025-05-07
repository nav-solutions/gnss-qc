#[derive(Debug, Default, Copy, Clone)]
pub enum State {
    #[default]
    /// Serialize all atemporal data constants
    Constants,

    /// Stream all header information
    RINEXHeaders,

    /// Stream all header information
    SP3Header,

    /// Serialize a new Ephemeris frame.
    Ephemeris,

    /// Serialize a Signal observation.
    Signal,

    /// Serialize a Meteo observation.
    Meteo,

    /// Serialize precise coordinates
    PreciseOrbit,

    /// Serialize precise clock
    PreciseClock,

    /// Done streaming
    Done,
}
