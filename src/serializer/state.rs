#[derive(Debug, Default, Copy, Clone)]
pub enum State {
    #[default]
    /// Serialize all atemporal data constants
    Constants,

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
