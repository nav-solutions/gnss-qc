#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum State {
    #[default]
    Parsing,
    Indexing,
    Iterating,
    Analysis,
    Synthesis,
}

pub struct QcPipeline {

}

impl QcPipeline {
    pub fn process(ctx: &mut QcContext) -> Result<QcAnalysis, QcError> {

        self.
    }
}
