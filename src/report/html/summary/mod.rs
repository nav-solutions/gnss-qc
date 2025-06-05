use itertools::Itertools;
use maud::{html, Markup, PreEscaped, Render};

use crate::report::summaries::{QcContextSummary, QcFileSummary};

mod rinex_sum;

#[cfg(feature = "sp3")]
mod sp3_sum;

impl QcContextSummary {
    pub(crate) fn javascript() -> String {
        "
        // first file shown
        const files_summary = document.querySelectorAll('.file-summary');

        function selectFileSummary(filename) {
            console.log('clicked: ' + filename);
            const targetId = filename + '-sum';
            const files_summary = document.querySelectorAll('.file-summary');

            files_summary.forEach(summary => {
                const sum_id = summary.getAttribute('id');
                if (sum_id == targetId) {
                    summary.setAttribute('style', 'display: block');
                } else {
                    summary.setAttribute('style', 'display: none');
                }
            });
        }
    "
        .to_string()
    }
}

impl Render for QcContextSummary {
    fn render(&self) -> Markup {
        html! {
            script {
                (PreEscaped(Self::javascript()))
            }

            div class="styled-table" {
                table class="table is-bordered" {
                    tr {
                        th {
                            "Product Type"
                        }
                        th {
                            "Indexing"
                        }
                        th {
                            "File"
                        }
                    }

                    @ for descriptor in self.summaries.keys().sorted() {
                        tr {
                            td onclick=(&format!("selectFileSummary('{}')", descriptor.filename)) {
                                (descriptor.product_type)
                            }
                            td onclick=(&format!("selectFileSummary('{}')", descriptor.filename)) {
                                (descriptor.indexing)
                            }
                            td onclick=(&format!("selectFileSummary('{}')", descriptor.filename)) {
                                (descriptor.filename)
                            }
                        }
                    }
                }
            }

            @ for (index, descriptor) in self.summaries.keys().sorted().enumerate() {
                @ if index == 0 {
                    div id=(&format!("{}-sum", descriptor.filename)) class="file-summary" style="display: block" {
                        @ if let Some(summary) = self.summaries.get(&descriptor) {
                            h2 {
                                (descriptor.filename)
                            }
                            p {
                                (summary.render())
                            }
                        }
                    }
                } @ else {
                    div id=(&format!("{}-sum", descriptor.filename)) class="file-summary" style="display: none" {
                        @ if let Some(summary) = self.summaries.get(&descriptor) {
                            h2 {
                                (descriptor.filename)
                            }
                            p {
                                (summary.render())
                            }
                        }
                    }
                }
            }

        }
    }
}

impl Render for QcFileSummary {
    fn render(&self) -> Markup {
        html! {
            @ match self {
                Self::RINEX(file) => (file.render()),
                Self::SP3(file) => (file.render()),
            }
        }
    }
}
