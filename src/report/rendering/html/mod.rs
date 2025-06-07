//! HTML rendition of the QcRunReport
use crate::report::QcRunReport;

use maud::{html, Markup, PreEscaped, Render, DOCTYPE};

mod css;
mod javascript;
mod observations;
mod orbit_proj;
pub(crate) mod plot;
mod rtk;
mod summary;

#[cfg(feature = "navigation")]
mod nav;

impl QcRunReport {
    /// Render this [QcRunReport] to HTML
    pub fn render_html(&self) -> Markup {
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
                    (PreEscaped(Self::css()))
                }

                body {
                    nav id="sidebar" {

                        h1 {
                            "GNSS-QC Report"
                        }

                        a class="active" data-target="run-report" {
                            "Run Report"
                        }

                        @ if self.summary.is_some() {
                            a data-target="summary" {
                                "Summary"
                            }
                        }

                        @ if self.rtk_summary.is_some() {
                            a data-target="rtk-summary" {
                                span {
                                    "RTK Summary"
                                }
                            }
                        }

                        @ if self.observations.is_some() {
                            a data-target="observations" {
                                span {
                                    "Observations"
                                }
                            }
                        }

                        @ if self.sp3_orbits_proj.is_some() {
                            a data-target="sp3-orbit-proj" {
                                span {
                                    "SP3 Orbit Projections"
                                }
                            }
                        }

                        @ if self.navi_report.is_some() {
                            a data-target="nav-report" {
                                span {
                                    "Navigation"
                                }
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
                        section id="run-report" class="section active" {
                            h2 {
                                "Run Report"
                            }
                            p {
                                (self.run_summary.render())
                            }
                        }

                        @ if let Some(summary) = &self.summary {
                            section id="summary" class="section" {
                                h2 {
                                    "Summary"
                                }
                                p {
                                    (summary.render())
                                }
                            }
                        }

                        @ if let Some(summary) = &self.rtk_summary {
                            section id="rtk-summary" class="section" {
                                h2 {
                                    "RTK Summary"
                                }
                                p {
                                    (summary.render())
                                }
                            }
                        }

                        @ if let Some(orbit_proj) = &self.sp3_orbits_proj {
                            section id="sp3-orbit-proj" class="section" {
                                h2 {
                                    "SP3 Orbit Projections"
                                }
                                p {
                                    (orbit_proj.to_html().render())
                                }
                            }
                        }

                        @ if let Some(observations) = &self.observations {
                            section id="observations" class="section" {
                                h2 {
                                    "Observations"
                                }
                                p {
                                    (observations.render())
                                }
                            }
                        }

                        @ if let Some(report) = &self.navi_report {
                            section id="nav-report" class="section" {
                                h2 {
                                    "NAV Report"
                                }
                                p {
                                    (report.render())
                                }
                            }
                        }

                        section id="documentation" class="section" {
                            h2 {
                                "Documentation"
                            }
                            p {
                                "TODO"
                            }
                        }

                        section id="sources" class="section" {
                            h2 {
                                "Code sources"
                            }
                            p {
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
                        (PreEscaped(Self::javascript()))
                    }

                }//body

            }
        }
    }
}

#[cfg(test)]
mod test {
    /**
     * Test HTML rendition using meaningful setups
     */
    use std::fs::File;
    use std::io::Write;

    use crate::{prelude::QcContext, tests::init_logger};

    use crate::prelude::QcAnalysisBuilder;

    #[test]
    fn html_no_sp3() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        ctx.load_gzip_rinex_file("data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }

    #[test]
    fn html_full_run() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/OBS/V3/LARM0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/LARM0630.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0010.22O").unwrap();

        ctx.load_rinex_file("data/OBS/V3/VLNS0630.22O").unwrap();

        ctx.load_gzip_rinex_file("data/MET/V3/POTS00DEU_R_20232540000_01D_05M_MM.rnx.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }

    #[test]
    fn html_full_run_24h() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_gzip_rinex_file("data/NAV/V3/ESBC00DNK_R_20201770000_01D_MN.rnx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/ESBC00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_rinex_file("data/CRNX/V3/MOJN00DNK_R_20201770000_01D_30S_MO.crx.gz")
            .unwrap();

        ctx.load_gzip_sp3_file("data/SP3/C/GRG0MGXFIN_20201770000_01D_15M_ORB.SP3.gz")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }

    #[test]
    fn html_jmf_longterm() {
        init_logger();

        let mut ctx = QcContext::new();

        // load data
        ctx.load_rinex_file("data/DataJMF/2024-09-18_00-00-00_GNSS-1.24o")
            .unwrap();

        ctx.load_rinex_file("data/DataJMF/2024-09-19_00-00-00_GNSS-1.obs")
            .unwrap();

        ctx.load_rinex_file("data/DataJMF/2025-04-29_19-53-50_GNSS-1.obs")
            .unwrap();

        ctx.load_rinex_file("data/DataJMF/240428survey.obs")
            .unwrap();

        let builder = QcAnalysisBuilder::all();

        let report = ctx.process(builder).unwrap();

        let html = report.render_html().into_string();
        let mut fd = File::create("index.html").unwrap();
        write!(fd, "{}", html).unwrap();
    }
}
