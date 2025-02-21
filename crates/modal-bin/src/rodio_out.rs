use modal_editor::AudioOut;
use rodio::{OutputStreamHandle, Sink};

pub struct RodioOut {
    sink: Sink,
    _handle: OutputStreamHandle,
}

impl Default for RodioOut {
    fn default() -> Self {
        let (stream, handle) = rodio::OutputStream::try_default().unwrap();
        std::mem::forget(stream);

        let sink = Sink::try_new(&handle).unwrap();

        RodioOut {
            sink,
            _handle: handle,
        }
    }
}

impl AudioOut for RodioOut {
    fn feed(&mut self, samples: &[f32]) {
        let source = rodio::buffer::SamplesBuffer::new(1, 44100, samples.to_vec());
        self.sink.append(source);
    }

    fn queue_len(&self) -> usize {
        self.sink.len()
    }

    fn start(&mut self) {
        self.sink.play();
    }
}
