// Synchronous + buffered data
pub struct QcSynchronousIterator<'a, T> {
    pub eos: bool,
    iter: Box<dyn Iterator<Item = T> + 'a>,
}

impl<'a, T: 'a> QcSynchronousIterator<'a, T> {
    pub fn null() -> Self {
        Self {
            eos: true,
            iter: Box::new([].into_iter()),
        }
    }

    pub fn new(iter: Box<dyn Iterator<Item = T> + 'a>) -> Self {
        Self { iter, eos: false }
    }
}

impl<'a, T> Iterator for QcSynchronousIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.eos {
            // consumed completely
            return None;
        }

        // try to pull new data
        match self.iter.next() {
            Some(pulled) => Some(pulled),
            None => {
                self.eos = true;
                None
            }
        }
    }
}
