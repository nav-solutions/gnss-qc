use itertools::Itertools;
use std::collections::HashMap;

use crate::prelude::{html, Markup, QcContext, Render};

#[cfg(feature = "sp3")]
mod compatible;

#[cfg(feature = "sp3")]
use compatible::FileReport;

#[cfg(not(feature = "sp3"))]
mod incompatible;

#[cfg(not(feature = "sp3"))]
use incompatible::FileReport;

/// One page per loaded product
pub struct Report {
    file_report: HashMap<String, FileReport>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            file_report: Default::default(),
        }
    }
}

impl Report {
    pub fn new(ctx: &QcContext) -> Self {
        Self {
            file_report: {
                let mut map = HashMap::new();
                if let Some(sp3) = &ctx.sp3 {
                    map.insert(
                        ctx.sp3_filename
                            .as_ref()
                            .expect("internal error: filename should be determined")
                            .to_string(),
                        FileReport::new(sp3),
                    );
                }
                map
            },
        }
    }

    pub fn has_content(&self) -> bool {
        !self.file_report.is_empty()
    }
}

impl Render for Report {
    fn render(&self) -> Markup {
        html! {
            @ for file in self.file_report.keys().sorted() {
                @ if let Some(page) = &self.file_report.get(file) {
                    (page.render())
                }
            }
        }
    }
}
