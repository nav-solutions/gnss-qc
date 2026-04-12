//! Input source

/// [QcInputSource] defines the data source uniquely.
/// For signal observations, it is the GNSS receiver.
/// For data products, it is the production agency.
/// We use this to differentiate data source precisely,
/// it is most useful in 2D algorithms, like RTK (Real Time Kinematic)
/// where we need to differentiate the "rover" from a reference station.
///
/// This is either automatically defined by the library (on parsing + loading),
/// or can be overwritten by the user to manually select how they want
/// data to be indexed and referenced to. This then creates customized
/// sessions and user experience.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum QcInputSource {
    /// This data is referenced to the following GNSS receiver.
    /// This is the prefered referencing method for signal sources.
    /// This is commonly used in RTK setups to easily differentiate rovers.
    Receiver(String),

    /// This data is referenced to the following receiver antenna.
    /// This can be useful in special setups or production environment,
    /// for example a similar agencies with many buildings, one antenna per building.
    Antenna(String),

    /// This data is referenced to the following production agency (data provider).
    Agency(String),

    /// This data is referenced to the following operator or user name.
    /// It can be used to differentiate data producers
    /// under the same agency.
    Operator(String),

    /// This data is referenced to the following alias, which can serve many roles
    /// depending on your application. It can be used to differentiate data producers
    /// under the same agency.
    Alias(String),
}
