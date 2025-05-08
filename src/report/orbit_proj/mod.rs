#[cfg(feature = "navigation")]
mod compatible;

#[cfg(feature = "navigation")]
pub use compatible::*;

#[cfg(not(feature = "navigation"))]
mod incompatible;

#[cfg(not(feature = "navigation"))]
pub use incompatible::QcOrbitProjections;
