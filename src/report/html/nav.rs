use maud::{html, Markup, PreEscaped, Render, DOCTYPE};

use crate::report::nav::QcNavReport;

impl Render for QcNavReport {
    fn render(&self) -> Markup {
        html! {
            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Elevation / SNR"
                        }
                    }

                    tr {
                        th {
                            "NAVI plot"
                        }
                    }
                }
            }
        }
    }
}
