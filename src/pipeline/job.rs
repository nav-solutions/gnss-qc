#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QcJob {
    /// Signal observation
    /// - signal extraction for visualization
    #[default]
    SignalObservation,
}
