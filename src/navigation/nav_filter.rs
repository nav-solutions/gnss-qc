//! NAV filter
use crate::error::QcError;
use gnss_rs::prelude::Constellation;

/// [QcNavFilterType] describes complex Navigation conditions
/// we may apply.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum QcNavFilterType {
    /// Healthy SV (suitable for navigation)
    Healthy,
    /// Unhealthy SV (not suitable for navigation)
    Unhealthy,
    /// (In-) testing SV (usually not suitable for navigation)
    Testing,
}

/// [QcNavFilter] is used to apply complex Navigation status conditions.
#[derive(Debug, Clone, PartialEq)]
pub struct QcNavFilter {
    /// [NavFilterType] we support.
    pub filter: QcNavFilterType,
    /// Possible targetted constellations
    pub constellations: Vec<Constellation>,
}

impl QcNavFilter {
    /// Build a [QcNavFilter] status condition that applys to any [Constellation] encountered.
    pub fn any(filter_type: QcNavFilterType) -> Self {
        Self {
            filter: filter_type,
            constellations: Default::default(),
        }
    }
}

impl std::str::FromStr for QcNavFilter {
    type Err = QcError;

    fn from_str(s: &str) -> Result<QcNavFilter, QcError> {
        let mut constellations = Vec::new();

        for item in s.split(':') {
            let trimmed = item.trim();

            for csv in trimmed.split(',') {
                if let Ok(parsed) = Constellation::from_str(csv.trim()) {
                    constellations.push(parsed);
                }
            }

            match trimmed {
                "healthy" => {
                    return Ok(QcNavFilter {
                        constellations,
                        filter: QcNavFilterType::Healthy,
                    });
                }
                "unhealthy" => {
                    return Ok(QcNavFilter {
                        constellations,
                        filter: QcNavFilterType::Unhealthy,
                    });
                }
                "testing" => {
                    return Ok(QcNavFilter {
                        constellations,
                        filter: QcNavFilterType::Testing,
                    });
                }
                _ => {}
            }
        }

        Err(QcError::InvalidNavFilter)
    }
}

#[cfg(test)]
mod test {
    use super::{QcNavFilter, QcNavFilterType};
    use gnss_rs::prelude::Constellation;
    use std::str::FromStr;

    #[test]
    fn nav_filter_parsing() {
        for (value, expected) in [
            (
                "healthy",
                QcNavFilter {
                    filter: QcNavFilterType::Healthy,
                    constellations: vec![],
                },
            ),
            (
                "unhealthy",
                QcNavFilter {
                    filter: QcNavFilterType::Unhealthy,
                    constellations: vec![],
                },
            ),
            (
                "testing",
                QcNavFilter {
                    filter: QcNavFilterType::Testing,
                    constellations: vec![],
                },
            ),
            (
                "gps:testing",
                QcNavFilter {
                    filter: QcNavFilterType::Testing,
                    constellations: vec![Constellation::GPS],
                },
            ),
            (
                "gps,gal:testing",
                QcNavFilter {
                    filter: QcNavFilterType::Testing,
                    constellations: vec![Constellation::GPS, Constellation::Galileo],
                },
            ),
        ] {
            let parsed = QcNavFilter::from_str(value)
                .unwrap_or_else(|e| panic!("Failed to parse from \"{}\": {}", value, e));

            assert_eq!(parsed, expected);
        }
    }
}
