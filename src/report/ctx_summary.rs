use crate::context::{QcIndexing, QcProductType};

use itertools::Itertools;
use std::collections::HashMap;

use rinex::prelude::Header as RINEXHeader;

use crate::context::data::QcSourceDescriptor;

#[cfg(feature = "sp3")]
use sp3::prelude::Header as SP3Header;

/// [QcPipeline] run summary.
#[derive(Debug, Clone, Default)]
pub struct QcContextSummary {
    /// Descriptors
    pub descriptors: Vec<QcSourceDescriptor>,
}

impl QcContextSummary {
    pub fn latch_rinex(&mut self, descriptor: QcSourceDescriptor, data: RINEXHeader) {
        self.descriptors.push(descriptor);
    }

    #[cfg(feature = "sp3")]
    pub fn latch_sp3(&mut self, descriptor: QcSourceDescriptor, data: SP3Header) {
        self.descriptors.push(descriptor);
    }
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
                    @ for category in self.descriptors.iter().map(|desc| desc.product_type).unique().sorted() {
                        @ for indexing in self.descriptors.iter().map(|desc| &desc.indexing).unique().sorted() {
                            @ for desc in self.descriptors.iter() {
                                @ if desc.product_type == category {
                                    @ if desc.indexing == *indexing {
                                        td {
                                            (category)
                                        }
                                        td {
                                            (indexing)
                                        }
                                        td {
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
    }
}
