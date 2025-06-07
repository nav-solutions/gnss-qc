use crate::report::rendering::html::plot::Plot;
use itertools::Itertools;
use maud::{html, Markup, Render};
use std::collections::HashMap;

use maud::PreEscaped;

use crate::report::orbit_proj::{
    QcConstellationOrbitProj, QcOrbitProjections, QcOrbitProjectionsKey,
};

pub struct QcHtmlOrbitProjections {
    plots: HashMap<QcOrbitProjectionsKey, Plot>,
}

impl Render for QcHtmlOrbitProjections {
    fn render(&self) -> Markup {
        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                }
            }
        }
    }
}

impl QcOrbitProjections {
    #[cfg(feature = "html")]
    pub fn to_html(&self) -> QcHtmlOrbitProjections {
        QcHtmlOrbitProjections {
            plots: {
                let mut map = HashMap::<QcOrbitProjectionsKey, Plot>::new();
                for (k, v) in self.projections.iter() {
                    let mut plot = Plot::timedomain_plot(
                        &format!("{}-{}-orbit-proj", k.indexing, k.constellation),
                        "Orbit Projection",
                        "coordinates (km)",
                        true,
                    );

                    map.insert(k.clone(), plot);
                }
                map
            },
        }
    }

    pub(crate) fn javascript() -> String {
        "
        const agency_form = document.getElementById('orbit-proj-agencies');

        agency_form.addEventListener('change', function(event) {
            const value = event.target.value;
            console.log('selected : ' + value);
        });

        const constell_form = document.getElementById('orbit-proj-constellations');

        constell_form.addEventListener('change', function(event) {
            const value = event.target.value;
            console.log('selected : ' + value);
        });

    "
        .to_string()
    }
}

#[cfg(not(feature = "navigation"))]
impl Render for QcOrbitProjections {
    fn render(&self) -> Markup {
        html! {}
    }
}

#[cfg(feature = "navigation")]
impl Render for QcOrbitProjections {
    fn render(&self) -> Markup {
        html! {

            div class="styled-table" {
                table class="table is-bordered" {
                    // Agency selector
                    tr {
                        th {
                            "Agency"
                        }
                        td {
                            form id="orbit-proj-agencies" {
                                @ for (index, item) in self.projections.keys().map(|k| &k.indexing).unique().sorted().enumerate() {
                                    @ if index == 0 {
                                        label {
                                            input type="radio" name="orbit-proj-agencies" value=(item) checked {}
                                            span {
                                                (item.to_string())
                                            }
                                        }
                                        br;

                                    } @ else {
                                        label {
                                            input type="radio" name="orbit-proj-agencies" value=(item) {}
                                            span {
                                                (item.to_string())
                                            }
                                        }
                                        br;
                                    }
                                }
                                @ if self.projections.keys().map(|k| &k.indexing).unique().count() > 1 {
                                    label {
                                        input type="radio" name="orbit-proj-agencies" value="all" {}
                                        span {
                                            "All"
                                        }
                                    }
                                    br;
                                }
                            }
                        }
                    }
                    tr {
                        th {
                            "Constellation"
                        }
                        td {
                            form id="orbit-proj-constellations" {
                                @ for (index, item) in self.projections.keys().map(|k| k.constellation).unique().sorted().enumerate() {
                                    @ if index == 0 {
                                        label {
                                            input type="radio" name="orbit-proj-constellations" value=(item) checked {}
                                            span {
                                                (item.to_string())
                                            }
                                        }
                                        br;

                                    } @ else {
                                        label {
                                            input type="radio" name="orbit-proj-constellations" value=(item) {}
                                            span {
                                                (item.to_string())
                                            }
                                        }
                                        br;
                                    }
                                }
                                @ if self.projections.keys().map(|k| &k.constellation).unique().count() > 1 {
                                    label {
                                        input type="radio" name="orbit-proj-constellations" value="all" {}
                                        span {
                                            "All"
                                        }
                                    }
                                    br;
                                }
                            }
                        }
                    }
                }

                @ for (index, key) in self.projections.keys().sorted().enumerate() {
                    tr {
                        td {
                            @ if let Some(proj) = self.projections.get(&key) {
                                @ if index == 0 {
                                    div id=(&format!("{}-{}", key.indexing, key.constellation)) style="display: block"  {
                                        (proj.render())
                                    }
                                } @ else {
                                    div id=(&format!("{}-{}", key.indexing, key.constellation)) style="display: none" {
                                        (proj.render())
                                    }
                                }
                            }
                        }
                    }
                }

                script {
                    (PreEscaped(Self::javascript()))
                }
            }
        }
    }
}

impl Render for QcConstellationOrbitProj {
    fn render(&self) -> Markup {
        html! {}
    }
}
