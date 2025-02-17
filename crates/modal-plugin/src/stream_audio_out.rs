use std::sync::{
    atomic::{AtomicU32, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc,
};

use modal_lib::remote::AudioOut;

pub struct StreamReader {
    rx: Receiver<Vec<f32>>,
    enqueued: Arc<AtomicU32>,
}

impl StreamReader {
    pub fn read(&self) -> Vec<f32> {
        let samples = self.rx.recv().unwrap();
        self.enqueued.fetch_sub(1, Ordering::Relaxed);
        samples
    }
}

pub struct StreamAudioOut {
    tx: Sender<Vec<f32>>,
    enqueued: Arc<AtomicU32>,
}

impl StreamAudioOut {
    pub fn new() -> (Self, StreamReader) {
        let enqueued = Arc::new(AtomicU32::new(0));
        let (tx, rx) = channel();
        (
            StreamAudioOut {
                tx,
                enqueued: Arc::clone(&enqueued),
            },
            StreamReader { rx, enqueued },
        )
    }
}

impl AudioOut for StreamAudioOut {
    fn queue_len(&self) -> usize {
        self.enqueued.load(Ordering::Relaxed) as usize
    }

    fn feed(&mut self, samples: &[f32]) {
        self.enqueued.fetch_add(1, Ordering::Relaxed);
        self.tx.send(samples.to_vec()).unwrap();
    }

    fn start(&mut self) {}
}
