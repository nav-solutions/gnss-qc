#[cfg(feature = "sp3")]
mod compatible;

#[cfg(feature = "sp3")]
pub use compatible::Projection;

#[cfg(not(feature = "sp3"))]
mod incompatible;

#[cfg(not(feature = "sp3"))]
pub use incompatible::Projection;
