// Synchronous + buffered data
pub struct QcSynchronousIterator<'a, T: Clone> {
    pub eos: bool,
    pub buffer: Vec<T>,
    last_read: Option<Epoch>,
    oldest_buffered: Option<Epoch>,
    iter: Box<dyn Iterator<Item = T> + 'a>,
}

impl<'a, T: Clone + 'a> QcSynchronousIterator<'a, T> {
    pub fn null() -> Self {
        Self {
            eos: true,
            buffer: Default::default(),
            last_read: None,
            oldest_buffered: None,
            iter: Box::new([].into_iter()),
        }
    }

    pub fn new(iter: Box<dyn Iterator<Item = T> + 'a>) -> Self {
        Self {
            iter,
            eos: false,
            last_read: None,
            oldest_buffered: None,
            buffer: Vec::with_capacity(8),
        }
    }
}

impl<'a, T: Clone> Iterator for QcSynchronousIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        None
    }
}
