use crate::pipeline::{errors::PipelineError, types::QcDataType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QcElementPortDirection {
    /// [QcElementPortDirection::Input] port
    Input,

    /// [QcElementPortDirection::Output] port
    Output,
}

impl std::str::FromStr for QcElementPortDirection {
    type Err = PipelineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();

        match s.as_str() {
            "in" | "input" => Ok(Self::Input),
            "out" | "output" => Ok(Self::Output),
            _ => Err(PipelineError::InvalidPortDirection),
        }
    }
}

pub struct QcElementPort {
    data_type: QcDataType,
    direction: QcElementPortDirection,
}

impl QcElementPort {
    pub fn can_connect(&self, rhs: &Self) -> bool {
        self.data_type == rhs.data_type && self.direction != rhs.direction
    }

    /// Creates a new RX [QcElementPort]
    pub fn rx_port(dtype: QcDataType) -> Self {
        Self {
            data_type: dtype,
            direction: QcElementPortDirection::Input,
        }
    }

    /// Creates a new TX [QcElementPort]
    pub fn tx_port(dtype: QcDataType) -> Self {
        Self {
            data_type: dtype,
            direction: QcElementPortDirection::Output,
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::str::FromStr;

    #[test]
    fn port_directions_parsing() {
        for (dtype, expected) in [
            ("in", QcElementPortDirection::Input),
            ("input", QcElementPortDirection::Input),
            ("out", QcElementPortDirection::Output),
            ("output", QcElementPortDirection::Output),
        ] {
            let parsed = QcElementPortDirection::from_str(dtype).unwrap();

            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn port_connections() {
        let tx_port_a = QcElementPort::tx_port(QcDataType::QcEphemerisData);
        let rx_port_a = QcElementPort::rx_port(QcDataType::QcEphemerisData);

        assert!(tx_port_a.can_connect(&rx_port_a));
        assert!(rx_port_a.can_connect(&tx_port_a)); // reciprocal

        let tx_port_b = QcElementPort::tx_port(QcDataType::QcEphemerisData);
        assert!(!tx_port_a.can_connect(&tx_port_b));
        assert!(!tx_port_b.can_connect(&tx_port_a)); // reciprocal

        let rx_port_b = QcElementPort::tx_port(QcDataType::QcObservationData);
        assert!(!tx_port_a.can_connect(&rx_port_b));
        assert!(!rx_port_b.can_connect(&tx_port_a)); // reciprocal
    }
}
