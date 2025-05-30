use itertools::Itertools;
use maud::{html, Markup, Render};

use crate::report::{html::plot::Plot, QcObservationsReport};

impl QcObservationsReport {
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

impl Render for QcObservationsReport {
    fn render(&self) -> Markup {
        let mut pseudo_range_plot =
            Plot::timedomain_plot("pseudo_range", "Pseudo Range", "meters", true);

        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Timeframe"
                        }
                        td {
                            @ if let Some(time_of_first_obs) = &self.time_of_first_obs {
                                tr {
                                    th {
                                        "Time of first observation"
                                    }
                                    td {
                                        (time_of_first_obs.to_string())
                                    }
                                }
                            }
                            @ if let Some(time_of_last_obs) = &self.time_of_last_obs {
                                tr {
                                    th {
                                        "Time of last observation"
                                    }
                                    td {
                                        (time_of_last_obs.to_string())
                                    }
                                }
                            }
                        }
                        td {
                            @ if let Some(time_of_last_obs) = &self.time_of_last_obs {
                                @ if let Some(time_of_first_obs) = &self.time_of_first_obs {
                                    tr {
                                        th {
                                            "Duration"
                                        }
                                        td {
                                            ((*time_of_last_obs - *time_of_first_obs).to_string())
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tr {
                        th {
                            "Data Sources"
                        }
                        tr {
                            @ for source in self.data.keys().map(|(index, _)| index).unique().sorted() {
                                td {
                                    (source)
                                }
                                tr {
                                    @ for constellation in self.data.keys().filter_map(|(index, constell)| {
                                        if index == source {
                                            Some(constell)
                                        } else {
                                            None
                                        }
                                    }).unique().sorted() {
                                        td {
                                            (constellation)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
