use hifitime::{Duration, Epoch};

pub enum QcSynchronousItem<I> {
    /// Item provided or streamed at specifiec [Epoch].
    Item(I),

    /// Stream is "paused" because pointer is already in the future
    /// and does not matched the desired point in time.
    Paused,

    /// End of stream, will terminate this I/O handle:
    /// the processor will drop it.
    EndOfStream,
}

/// [QcStream] is a generic Trait implemented by all I/O of this library.
/// It must be implemented by all custom I/Os.
///
/// - Input: whether they are input files or input streams
/// - Output: data points we can stream (out)
pub trait QcStream {
    /// Item to return in case a data point is available.
    type Item;

    /// Error type possibly returned.
    type Error;

    /// Attempts to pull a new [QcStream::Item],
    /// returning None on the end of stream.
    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error>;
}

/// [QcSynchronousStream] needs to be implemented by all Input providing
/// temporal data points to the following process.
/// The upstream part of the pipeline needs to implement some kind
/// of buffering mechanism to match the requested Epoch.
pub trait QcSynchronousStream {
    /// Item to return in case a data point is available.
    type Item;

    /// Error type possibly returned.
    type Error;

    /// Attempts to pull a new [QcSynchronousStream::Item] at specified [Epoch].
    ///
    /// - Duration::ZERO is used if the [Epoch] must be matched _exactly_
    fn next_at_epoch(
        &mut self,
        epoch: Epoch,
        margin: Duration,
    ) -> Result<Option<Self::Item>, Self::Error>;
}
