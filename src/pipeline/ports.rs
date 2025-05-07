use crate::pipeline::{
    types::QcPipelineTypes,
    errors::PipelineError,
};

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
    port_type: QcPipelineTypes,
    port_direction: QcElementPortDirection,
}

impl QcElementPort {
    pub fn can_connect(&self, rhs: &Self) -> bool {
        self.port_type == rhs.port_type && self.port_direction != rhs.port_direction
    }

    /// Creates a new RX [QcElementPort]
    pub fn rx_port(port_type: QcPipelineTypes) -> Self {
        Self {
            port_type,
            port_direction: QcElementPortDirection::Input,
        }
    }

    /// Creates a new TX [QcElementPort]
    pub fn tx_port(port_type: QcPipelineTypes) -> Self {
        Self {
            port_type,
            port_direction: QcElementPortDirection::Output,
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
            let parsed = QcElementPortDirection::from_str(dtype)
                .unwrap();

            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn port_connections() {
        let tx_port_a = QcElementPort::tx_port(QcPipelineTypes::QcEphemerisData);
        let rx_port_a = QcElementPort::rx_port(QcPipelineTypes::QcEphemerisData);

        assert!(tx_port_a.can_connect(&rx_port_a));
        assert!(rx_port_a.can_connect(&tx_port_a)); // reciprocal


        let tx_port_b = QcElementPort::tx_port(QcPipelineTypes::QcEphemerisData);
        assert!(!tx_port_a.can_connect(&tx_port_b));
        assert!(!tx_port_b.can_connect(&tx_port_a)); // reciprocal

        let rx_port_b = QcElementPort::tx_port(QcPipelineTypes::QcObservationData);
        assert!(!tx_port_a.can_connect(&rx_port_b));
        assert!(!rx_port_b.can_connect(&tx_port_a)); // reciprocal
    }
}