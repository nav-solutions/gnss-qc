use crate::report::QcContextSummary;
use itertools::Itertools;

use maud::{html, Markup, PreEscaped, Render};

impl QcContextSummary {
    pub(crate) fn javascript() -> String {
        "
        // first file shown
        const files_summary = document.querySelectorAll('.file-summary');

        // files_summary[0].styleList.remove('display: none');
        // files_summary[0].styleList.add('display: block');

        // const first_id = files_summary[0].getAttribute('id');
        // console.log('file_sum: ' + first_id);

        // const first_style = files_summary[0].getAttribute('style');
        // console.log('file_sum: ' + first_style);

        // files_summary[0].setAttribute('style', 'display: block');

        // files_summary.forEach(summary => {
        //     const id = summary.getAttribute('id');
        //     console.log('file_sum: ' + id);
        // });

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

                    @ for category in self.summaries.keys().map(|desc| desc.product_type).unique().sorted() {
                        @ for indexing in self.summaries.keys().map(|desc| &desc.indexing).unique().sorted() {
                            @ for  desc in self.summaries.keys() {
                                @ if desc.product_type == category {
                                    @ if desc.indexing == *indexing {
                                        tr {
                                            td onclick=(&format!("selectFileSummary('{}')", desc.filename)) {
                                                (category)
                                            }
                                            td onclick=(&format!("selectFileSummary('{}')", desc.filename)) {
                                                (indexing)
                                            }
                                            td onclick=(&format!("selectFileSummary('{}')", desc.filename)) {
                                                (desc.filename)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            @ for (index, descriptor) in self.summaries.keys().sorted().rev().enumerate() {
                @ if index == 0 {
                    div id=(&format!("{}-sum", descriptor.filename)) class="file-summary" style="display: block" {
                        h2 {
                            (descriptor.filename)
                        }
                    }
                } @ else {
                    div id=(&format!("{}-sum", descriptor.filename)) class="file-summary" style="display: none" {
                        h2 {
                            (descriptor.filename)
                        }
                    }
                }
            }

        }
    }
}
