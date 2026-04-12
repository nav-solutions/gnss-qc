#[cfg(feature = "navigation")]
mod nav;

#[cfg(feature = "cggtts")]
mod cggtts;

#[cfg(feature = "navigation")]
pub use nav::NavigationSolutions;

#[cfg(feature = "cggtts")]
pub use cggtts::CggttsSolutions;

/// All supported [Solutions] that our solver may resolve.
pub enum Solutions {
    /// [CGGTTS] solutions, obtained by solving [Algorithms::CggttsSolutions]
    #[cfg(feature = "cggtts")]
    CggttsSolutions,

    #[cfg(feature = "navigation")]
    /// Solutions for a specific rover.
    NavigationSolutions,
}
