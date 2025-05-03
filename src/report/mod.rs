//! Generic analysis report
use std::collections::HashMap;

use maud::{html, Markup, PreEscaped, Render, DOCTYPE};

use crate::{
    context::QcIndexing,
    prelude::{Epoch, QcContext},
};

mod css;
mod javascript;
mod selector;

// shared analysis, that may apply to several products
mod shared;

mod summary;
use summary::QcSummary;

mod rovers;
use rovers::Report as RoversReport;

mod orbit_residuals;
use orbit_residuals::Projection as OrbitResidualsProjection;

mod sp3;
use sp3::Report as SP3Report;

pub(crate) use selector::{AxisSelector, ConstellationSelector, PosVelSelector};

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

    /// SP3 report (when feasible)
    sp3_files_report: SP3Report,

    /// Orbital residuals (when feasible)
    orbit_residuals_proj: OrbitResidualsProjection,

    /// Reported rovers (when feasible)
    rovers: RoversReport,

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
            custom_chapters: Vec::new(),
            rovers: Default::default(),
            sp3_files_report: Default::default(),
            orbit_residuals_proj: Default::default(),
        }
    }

    /// Synthesize a complete [QcReport] from current [QcContext].
    /// ## Input
    /// - now: [Epoch] of synthesis
    pub fn report(&self, now: Epoch) -> QcReport {
        QcReport {
            summary: QcSummary::new(now, self),
            custom_chapters: Vec::new(),
            rovers: RoversReport::new(self),
            sp3_files_report: SP3Report::new(self),
            orbit_residuals_proj: OrbitResidualsProjection::new(self),
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
                    script src="https://unpkg.com/lucide@latest" {};
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

                        @ if self.rovers.has_content() {
                            // Create a nav menu
                            a data-target="observations" {
                                "Observations"
                            }
                        }

                        @ if self.sp3_files_report.has_content() {
                            // Create nav menu
                            a data-target="sp3" {
                                span {
                                    "SP3 Precise Orbits "
                                }
                                i data-lucide="satellite" {}
                            }
                        }

                        @ if self.orbit_residuals_proj.has_content() {
                            // Create a nav menu
                            a data-target="orbit-residuals" {
                                span {
                                    "Orbital Residuals "
                                }
                                i data-lucide="satellite" {}
                            }
                        }

                        a data-target="documentation" {
                            span {
                                "Documentation "
                            }
                            i data-lucide="book" {}
                        }

                        a data-target="sources" {
                            span {
                                "Sources "
                            }
                            i data-lucide="book" {}
                        }

                        a data-target="credits" {
                            span {
                                "Credits "
                            }
                            i data-lucide="radio" {}
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

                        // rovers section
                        @ if self.rovers.has_content() {
                            section id="rovers" class="section" {
                                (self.rovers.render())
                            }
                        }

                        // SP3 files section
                        @ if self.sp3_files_report.has_content() {
                            section id="sp3-files" class="section" {
                                (self.sp3_files_report.render())
                            }
                        }

                        // orbital residuals section
                        @ if self.orbit_residuals_proj.has_content() {
                            section id="orbit-residuals" class="section" {
                                (self.orbit_residuals_proj.render())
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
