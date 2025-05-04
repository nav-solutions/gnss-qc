use crate::prelude::{html, Markup, QcContext, QcIndexing, Render, TimeScale};

mod bias;
use bias::BiasSummary;

// mod orbital_proj;
// use orbital_proj::Projection as OrbitalProjection;

// mod observations;
// use observations::Report as ObservationsReport;

enum IndexedBy {
    None,
    Antenna,
    Agency,
    Operator,
    Custom,
    GnssReceiver,
    GeodeticMarker,
}

impl IndexedBy {
    pub fn new(value: &QcIndexing) -> Self {
        match value {
            QcIndexing::Agency(_) => Self::Agency,
            QcIndexing::Custom(_) => Self::Custom,
            QcIndexing::GeodeticMarker(_) => Self::GeodeticMarker,
            QcIndexing::GnssReceiver(_) => Self::GnssReceiver,
            QcIndexing::None => Self::None,
            QcIndexing::Operator(_) => Self::Operator,
            QcIndexing::RxAntenna(_) => Self::Antenna,
        }
    }
}

impl std::fmt::Display for IndexedBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agency => write!(f, "Agency"),
            Self::Antenna => write!(f, "RX Antenna"),
            Self::GeodeticMarker => write!(f, "Geodetic Marker"),
            Self::GnssReceiver => write!(f, "GNSS reicever"),
            Self::None => write!(f, "Undefined"),
            Self::Custom => write!(f, "Custom"),
            Self::Operator => write!(f, "Operator"),
        }
    }
}

pub struct Report {
    name: String,
    bias: BiasSummary,
    indexed_by: IndexedBy,
    timescale: Option<TimeScale>,
    // orbit_proj: OrbitalProjection,
    // observations: ObservationsReport,
}

impl Report {
    pub fn new(ctx: &QcContext, indexing: &QcIndexing) -> Self {
        let observations = ctx
            .observations_data(indexing)
            .expect("internal error: data should exist");

        Self {
            name: indexing.to_string(),
            indexed_by: IndexedBy::new(indexing),
            bias: BiasSummary::new(ctx, observations),
            timescale: ctx.observations_timescale(indexing),
            // orbit_proj: OrbitalProjection::new(&ctx, &observations),
            // observations: ObservationsReport::new(&observations),
        }
    }
}

impl Render for Report {
    fn render(&self) -> Markup {
        html! {
            table class="styled-table" {
                tbody {
                    tr {
                        th {
                            "Name"
                        }
                        td {
                            (self.name)
                        }
                    }
                    tr {
                        th {
                            "Indexed by"
                        }
                        td {
                            (self.indexed_by)
                        }
                    }
                    tr {
                        th {
                            button aria-label="Timescale in which observations are expressed" data-balloon-pos="right" {
                                "Timescale"
                            }
                        }
                        @ if let Some(timescale) = &self.timescale {
                            td {
                                (timescale.to_string())
                            }
                        } @ else {
                            td {
                                "Undefined"
                            }
                        }
                    }
                    tr {
                        th {
                            "Bias"
                        }
                        td {
                            (self.bias.render())
                        }
                    }
                }
            }
        }
    }
}
