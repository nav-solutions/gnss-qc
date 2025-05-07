use crate::context::{QcIndexing, QcProductType};

use itertools::Itertools;
use std::collections::HashMap;

/// [QcPipeline] run summary.
#[derive(Debug, Clone, Default)]
pub struct QcContextSummary {
    /// Input products
    pub input_products: HashMap<(QcProductType, QcIndexing), String>,
}

use maud::{html, Markup, Render};

impl Render for QcContextSummary {
    fn render(&self) -> Markup {
        html! {
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
                    @ for category in self.input_products.keys().map(|k| k.0).unique().sorted() {
                        @ for indexing in self.input_products.keys().map(|k| &k.1).unique().sorted() {
                            @ if let Some((_, filename)) = self.input_products.iter().find(|((cat, index), _)| *cat == category && index == indexing) {
                                tr {
                                    td {
                                        (category)
                                    }
                                    td {
                                        (indexing)
                                    }
                                    td {
                                        (filename)
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
