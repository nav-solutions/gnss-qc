//! Generic analysis report
use itertools::Itertools;
use std::collections::HashMap;

use maud::{html, Markup, PreEscaped, Render, DOCTYPE};

use crate::{
    context::QcIndexing,
    prelude::{Epoch, QcContext},
};

mod constellations_sel;
mod css;
mod javascript;

// shared analysis, that may apply to several products
mod shared;

mod summary;
use summary::QcSummary;

mod observations;
use observations::Report as ObservationsReport;

mod orbital;
use orbital::OrbitalProjections;

pub(crate) use constellations_sel::ConstellationsSelector;

/// [QcExtraPage] you can add to customize [QcReport]
pub struct QcExtraPage {
    /// tab for pagination
    pub tab: Box<dyn Render>,
    /// content
    pub content: Box<dyn Render>,
    /// HTML id
    pub html_id: String,
}

/// [QcReport] is a generic structure to report complex analysis results
pub struct QcReport {
    /// Report Summary (always present)
    summary: QcSummary,

    /// Orbital projections (when feasible)
    orbital_proj: OrbitalProjections,

    // orbital_proj: Option<OrbitalProjections>,
    /// Reported observations, for each data source
    observations: HashMap<QcIndexing, ObservationsReport>,

    // /// IONEX TEC page when it exists
    // ionex_page: Option<IonexPage>,
    /// Custom chapters
    custom_chapters: Vec<QcExtraPage>,
}

impl QcContext {
    /// Synthesize a shortened [QcSummary] from current [QcContext].
    /// ## Input
    /// - now: [Epoch] of synthesis
    pub fn summary_report(&self, now: Epoch) -> QcReport {
        QcReport {
            summary: QcSummary::new(now, self),
            orbital_proj: Default::default(),
            custom_chapters: Vec::new(),
            observations: Default::default(),
        }
    }

    /// Synthesize a complete [QcReport] from current [QcContext].
    /// ## Input
    /// - now: [Epoch] of synthesis
    pub fn report(&self, now: Epoch) -> QcReport {
        QcReport {
            summary: QcSummary::new(now, self),
            custom_chapters: Vec::new(),
            observations: {
                let mut tabbed = HashMap::new();

                for source in self.observations.keys() {
                    if let Some(observations) = self.observations.get(&source) {
                        tabbed.insert(
                            source.clone(),
                            ObservationsReport::new(self, source, observations),
                        );
                    }
                }

                tabbed
            },
            orbital_proj: OrbitalProjections::new(self),
        }
    }
}

impl QcReport {
    /// Add a custom chapter, in form of a [QcExtraPage] to this report.
    pub fn add_custom_chapter(&mut self, chapter: QcExtraPage) {
        self.custom_chapters.push(chapter);
    }
}

impl Render for QcReport {
    fn render(&self) -> Markup {
        html! {
            (DOCTYPE)
            html {
                head {
                    meta charset="UTF-8";
                    meta http-equip="X-UA-Compatible" content="IE-edge";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    link rel="icon" type="image/x-icon" href="https://raw.githubusercontent.com/rtk-rs/.github/master/logos/logo2.jpg";
                    script src="https://cdn.plot.ly/plotly-2.12.1.min.js" {};
                    script defer="true" src="https://use.fontawesome.com/releases/v5.3.1/js/all.js" {};
                    script src="https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/tex-svg.js" {};
                    link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.2/css/all.min.css";
                    link rel="stylesheet" href="https://unpkg.com/balloon-css/balloon.min.css";
                }//head

                style {
                    (PreEscaped(self.page_css()))
                }

                body {
                    nav id="sidebar" {
                        h1 {
                            "GNSS-QC Report"
                        }
                        a class="active" data-target="summary" {
                            "Summary"
                        }

                        @ if !self.observations.is_empty() {
                            // Create a nav menu
                            a data-target="observations" {
                                "Observations"
                            }
                        }

                        @ if self.orbital_proj.has_content() {
                            // Create a nav menu
                            a data-target="orbit-proj" {
                                "Orbital Projections"
                            }
                        }

                        a data-target="documentation" {
                            "Documentation"
                        }

                        a data-target="sources" {
                            "Sources"
                        }

                        a data-target="credits" {
                            "Credits"
                        }
                    }

                    div class="content" {
                        section id="summary" class="section active" {
                            h2 {
                                "Summary report"
                            }
                            p {
                                (self.summary.render())
                            }
                        }

                        // observations section
                        @ if !self.observations.is_empty() {
                            section id="observations" class="section" {
                                h2 {
                                    "RINEX Observations"
                                }

                                // one tab to select observation source
                                div class="tabs" id="observation-sources" {
                                    @ for (num, source) in self.observations.keys().sorted().enumerate() {
                                        @ if num == 0 {
                                            div class="tab active" data-target=(source.to_string()) {
                                                (source.to_string())
                                            }
                                        } @ else {
                                            div class="tab" data-target=(source.to_string()) {
                                                (source.to_string())
                                            }
                                        }
                                    }
                                }

                                // one section per source
                                @ for (num, source) in self.observations.keys().enumerate() {
                                    @ if num == 0 {
                                        @ if let Some(report) = self.observations.get(&source) {
                                            div class="content-section active" id=(source.to_string()) {
                                                (report.render())
                                            }
                                        }
                                    } @ else {
                                        @ if let Some(report) = self.observations.get(&source) {
                                            div class="content-section" id=(source.to_string()) {
                                                (report.render())
                                            }
                                        }
                                    }
                                }

                            }
                        }

                        @ if self.orbital_proj.has_content() {
                            // Render content
                            section id="orbit-proj" class="section" {
                                (self.orbital_proj.render())
                            }
                        }

                        section id="documentation" class="section" {
                            h2 {
                                "Test"
                            }
                        }

                        section id="sources" class="section" {
                            h2 {
                                "GNSS-QC is part of the RTK-rs framework for advanced GNSS and Geodesy applications"
                            }
                            p {
                                "The framework is hosted on github.com"
                            }
                            p {
                                a href="https://github.com/rtk-rs/gnss-qc" {
                                    "GNSS-QC: Geodesy and GNSS post-processing"
                                }
                            }
                            p {
                                a href="https://github.com/rtk-rs/gnss-rtk" {
                                    "GNSS-RTK: P.V.T solution solver"
                                }
                            }
                            p {
                                a href="https://github.com/rtk-rs/rinex" {
                                    "RINEX parser"
                                }
                            }
                            p {
                                a href="https://github.com/rtk-rs/sp3" {
                                    "SP3 parser"
                                }
                            }
                            p {
                                a href="https://github.com/rtk-rs" {
                                    "CGGTTS for remote clock comparison & common-view time transfer"
                                }
                            }
                        }

                        section id="credits" class="section" {
                            h2 {
                                "GNSS-QC is part of the RTK-rs framework for advanced GNSS and Geodesy applications"
                            }
                            p {
                                "TODO"
                            }
                        }
                    }

                    script {
                        (PreEscaped(self.javascript()))
                    }

                }//body

            }
        }
    }
}
