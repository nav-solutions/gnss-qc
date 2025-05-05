#[derive(Debug, Default, Copy, Clone)]
pub enum State {
    #[default]
    /// Serialize all atemporal data constants
    Constants,

    /// Done streaming
    Done,
}
