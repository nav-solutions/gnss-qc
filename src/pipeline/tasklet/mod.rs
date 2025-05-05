pub mod observations;


pub trait QcTasklet {
    type Output;

    /// Execute [QcTasklet]
    fn run(&mut self) -> Self::Output;
}
