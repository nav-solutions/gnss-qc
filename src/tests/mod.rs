mod clock;
mod engine;

mod input;
#[cfg(feature = "ionex")]
mod ionex;

#[cfg(all(feature = "ionex", feature = "sp3"))]
mod ipp;
mod miso;
mod residuals;

#[cfg(feature = "rtcm")]
mod rtcm;

#[cfg(feature = "serde")]
mod serde;

#[cfg(all(feature = "navigation", feature = "sp3"))]
mod navigation;
pub mod toolkit;

/// JSON round trip, compared by value
#[cfg(feature = "serde")]
pub fn serde_round_trip<T>(value: &T)
where
    T: ::serde::Serialize + ::serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).unwrap();
    let parsed: T = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("failed to deserialize {}: {}", json, e));
    assert_eq!(&parsed, value, "{}", json);
}
