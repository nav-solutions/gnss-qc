#[cfg(feature = "html")]
#[cfg_attr(docsrs, doc(cfg(feature = "html")))]
mod html;

#[cfg(feature = "pdf")]
#[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
mod pdf;

#[cfg(feature = "pdf")]
#[cfg_attr(docsrs, doc(cfg(feature = "pdf")))]
mod plots;

#[cfg(doc)]
use crate::prelude::QcReport;

/// [QcRenderingColorMap] is used to define the color map
/// when drawing many curves on a single plot.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum QcRenderingColorMap {
    #[default]
    Vulcano,
}

impl QcRenderingColorMap {
    pub(crate) fn color(&self, unitary_value: f64) -> plotters::Color {
        match self {
            Self::Vulcano => VulcanoHSL.get_color(unitary_value),
        }
    }
}

impl std::str::FromStr for QcRenderingColorMap {
    type Err = ;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().lowercase() {
            "vulcano" => Ok(Self::Vulcano),
            _ => Err(QcError::InvalidColorMap),
        }
    }
}

/// [QcPlotsPerPage] controls plots presentation and
/// the number of plots per page.
pub enum QcPlotsPerPage {
    /// One plot per page (large)
    One,
    
    /// Two plots per page 1, 1 (smaller)
    Two,

    /// Four plots per page 2, 2 (even smaller)
    Four,
}

/// [QcRenderingOptions] to customize [QcReport] rendering 
pub struct QcRenderingOptions {
    /// [QcRenderingColorMap] used when plotting data
    pub color_map: QcRenderingColorMap,

    /// [QcPlotsPerPage] controls plots presentation and
    /// the number of plots per page.
    pub plots_per_page: QcPlotsPerPage,

    /// Plot different data sources (GNSS receivers) into the same plot,
    /// when reporting signals, to reduce the number of pages.
    /// By default, this is set to false.
    pub merge_data_sources: bool,

    /// Plot different constellations into the same plot, when
    /// reporting signals, to reduce the number of pages.
    /// By default, this is set to false.
    pub merge_constellations: bool,

    /// Plot different data publishers (agencies) into the same plot,
    /// when that applies, to reduce the number of plots.
    /// By default, this is set to false.
    /// All residual analysis will then be grouped in a unique plot.
    pub merge_data_publishers: bool,
}

impl Default for QcRenderingOptions {
    fn default() -> Self {
        Self {
            color_map: QcRenderingColorMap::default(),
            merge_data_sources: false,
            merge_constellations: false,
            merge_data_publishers: false,
        }
    }
}
