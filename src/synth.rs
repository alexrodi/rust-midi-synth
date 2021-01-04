pub mod envelope;
use envelope::Envelope;

#[derive(Debug)]
pub struct Synth {
    frequency: f32,
    gain: f32,
    phase: f32,
    increment: f32,
    sample_rate: u32,
    envelope: Envelope
}

const CENT_FACTOR: f32 = 1.000_577_8; // 2 to the power of (1.0 / 1200.0)

impl Synth {
    pub fn new(sample_rate: u32) -> Self {
        let frequency = 440.0;
        let increment = std::f32::consts::TAU / (sample_rate as f32 / frequency);
        let envelope = Envelope::new(sample_rate, envelope::Adsr::default());
        Self {
            frequency,
            gain: 1.0,
            phase: 0.0,
            increment,
            sample_rate,
            envelope
        }
    }

    pub fn gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    fn set_increment(&mut self, frequency: f32) {
        self.increment = std::f32::consts::TAU / (self.sample_rate as f32 / frequency);
    }

    pub fn frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
        self.set_increment(frequency);
    }

    pub fn pitchbend_cents(&mut self, shift_cents: f32) {
        let new_frequency = self.frequency * CENT_FACTOR.powf(shift_cents);
        self.set_increment(new_frequency);
    }

    pub fn process(&mut self, channels: usize, buffer: &mut [f32]) {
        for frame in buffer.chunks_mut(channels) {
            let next_sample = self.phase.sin() * self.gain;
            for sample in frame { *sample = next_sample };
            self.advance();
        }
        self.envelope.process_apply(buffer);
    }

    fn advance(&mut self) {
        self.phase += self.increment;
        self.phase %= std::f32::consts::TAU;
    }

    pub fn message_envelope(&mut self, message: envelope::Message) {
        self.envelope.message(message);
    }
}



// PASTED CODE FROM THE SITE!
