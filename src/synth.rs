#[derive(Debug)]
pub struct Synth {
    frequency: f32,
    gain: f32,
    phase: f32,
    increment: f32,
    sample_rate: u32
}

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        let frequency = 440.0;
        let increment = std::f32::consts::TAU / (sample_rate as f32 / frequency);
        Self {
            frequency,
            gain: 1.0,
            phase: 0.0,
            increment,
            sample_rate
        }
    }

    pub fn gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
        self.increment = std::f32::consts::TAU / (self.sample_rate as f32 / frequency);
    }

    pub fn process(&mut self, channels: usize, buffer: &mut [f32]) {
        for frame in buffer.chunks_mut(channels) {
            let next_sample = self.phase.sin() * self.gain;
            for sample in frame { *sample = next_sample };
            self.advance();
        }
    }

    fn advance(&mut self) {
        self.phase += self.increment;
        self.phase %= std::f32::consts::TAU;
    }
}