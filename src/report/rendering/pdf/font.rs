use genpdf::fonts::{FontData, FontFamily};

pub struct QcPdfFontFamily {}

impl QcPdfFontFamily {
    /// Creates a [FontFamily] ready to use in Qc PDF report synthesis.
    pub fn new() -> FontFamily<FontData> {
        FontFamily {
            regular: FontData::new(
                include_bytes!("../../../../fonts/DejaVuSerifCondensed-Regular.ttf").to_vec(),
                None,
            )
            .expect("oops"),
            italic: FontData::new(
                include_bytes!("../../../../fonts/DejaVuSerifCondensed-Italic.ttf").to_vec(),
                None,
            )
            .expect("oops"),
            bold: FontData::new(
                include_bytes!("../../../../fonts/DejaVuSerifCondensed-BoldItalic.ttf").to_vec(),
                None,
            )
            .expect("oops"),
            bold_italic: FontData::new(
                include_bytes!("../../../../fonts/DejaVuSerifCondensed-Italic.ttf").to_vec(),
                None,
            )
            .expect("oops"),
        }
    }
}
