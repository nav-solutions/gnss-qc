use itertools::Itertools;
use maud::{html, Markup, Render};

#[cfg(feature = "navigation")]
use maud::PreEscaped;

use crate::report::orbit_proj::{QcConstellationOrbitProj, QcOrbitProjections};

impl QcOrbitProjections {
    #[cfg(feature = "navigation")]
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
