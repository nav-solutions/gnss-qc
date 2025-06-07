pub struct QcPdfLogo {}

impl QcPdfLogo {
    /// Creates the Logo
    pub fn new() -> image::DynamicImage {
        let logo_bitmap = include_bytes!("../../../../logo/logo.png");
        let image = image::load_from_memory(logo_bitmap).unwrap();

        image
    }
}
