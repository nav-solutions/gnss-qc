use log::error;

use crossbeam_channel::{Receiver, RecvError, Sender};

pub trait Job {
    /// [Job::Input] type
    type Input;

    /// [Job::Output] type
    type Output;

    /// Process number of available [Job::Input] symbols, possibly returning an [Job::Output]
    fn process(&mut self, size: usize, samples: &[Self::Input]) -> Option<Self::Output>;
}

pub struct Node<I: Clone, O: Clone, J: Job<Input = I, Output = O>> {
    job: J,
    name: String,
    tx: Sender<O>,
    rx: Receiver<I>,
    rx_fifo: Vec<I>,
    rx_size: usize,
    rx_capacity: usize,
}

impl<I: Clone, O: Clone, J: Job<Input = I, Output = O>> Node<I, O, J> {
    /// Create a new [Node]
    pub fn new(name: &str, rx: Receiver<I>, rx_capacity: usize, tx: Sender<O>, job: J) -> Self {
        Self {
            job,
            tx,
            rx,
            rx_size: 0,
            rx_capacity,
            name: name.to_string(),
            rx_fifo: Vec::with_capacity(rx_capacity),
        }
    }

    /// Execute this task
    pub fn run(&mut self) {
        loop {
            match self.rx.recv() {
                Ok(value) => {
                    // will increase as long as processing does not consume
                    // maybe we should introduce a max limit
                    self.rx_fifo.push(value);
                    self.rx_size += 1;
                }
                Err(_) => {
                    // a message could not be received because the channel is disconnected.
                    break;
                }
            }

            // will not propose until initial capacity is reached
            if self.rx_size >= self.rx_capacity {
                if let Some(output) = self.job.process(self.rx_size, self.rx_fifo.as_slice()) {
                    // post results
                    match self.tx.send(output) {
                        Ok(_) => {}
                        Err(e) => {
                            error!("{} - failed to propagate data: {}", self.name, e);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

    use crate::tests::init_logger;

    use super::{Job, Node};
    use crossbeam_channel::{Receiver, Sender};
    use threadpool::ThreadPool;

    struct Job1 {}

    impl Job for Job1 {
        type Input = f64;
        type Output = f64;

        fn process(&mut self, size: usize, _: &[Self::Input]) -> Option<Self::Output> {
            if size > 10 {
                Some(1.0)
            } else {
                None
            }
        }
    }

    struct Job2 {}

    impl Job for Job2 {
        type Input = f64;
        type Output = u64;

        fn process(&mut self, size: usize, _: &[Self::Input]) -> Option<Self::Output> {
            if size > 10 {
                Some(2)
            } else {
                None
            }
        }
    }

    #[test]
    fn node_test() {
        init_logger();

        // let n_workers = 4;
        // let mut pool = ThreadPool::new(n_workers);

        let (tx, node1_rx) = crossbeam_channel::unbounded();
        let (node1_tx, node2_rx) = crossbeam_channel::unbounded();
        let (node2_tx, output_rx) = crossbeam_channel::unbounded();

        let job1 = Job1 {};
        let mut node_1 = Node::new("node1", node1_rx, 128, node1_tx, job1);

        let job2 = Job2 {};
        let mut node_2 = Node::new("node2", node2_rx, 128, node2_tx, job2);

        let task_1 = std::thread::spawn(move || {
            node_1.run();
            println!("node_1: is done");
        });

        let task_2 = std::thread::spawn(move || {
            node_2.run();
            println!("node_2: is done");
        });

        let task_3 = std::thread::spawn(move || loop {
            match output_rx.recv() {
                Ok(value) => println!("pipeline output: {}", value),
                Err(_) => break,
            }
        });

        // simule data (input)
        for i in 0..1024 {
            let _ = tx.send(i as f64).unwrap();
        }

        drop(tx);

        task_1.join().unwrap();
        task_2.join().unwrap();
        task_3.join().unwrap();
    }
}
