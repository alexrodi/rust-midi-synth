/// A struct containing the 4 parameters of an ADSR
///  - Attack (ms)
///  - Decay (ms)
///  - Sustain (gain)
///  - Release (ms)
#[derive(Debug)]
pub struct Adsr {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Adsr {
            attack: 1000.0,
            decay: 100.0,
            sustain: 0.75,
            release: 1000.0
        }
    }
}

pub enum Message {
    On { velocity: f32 },
    Off,
}

/// A logarithmic Adsr - Envelope with smooth retrigger operation
#[derive(Debug)]
pub struct Envelope {
    params: Adsr,
    factors: [f32; 3],
    state: State,
    velocity: f32,
    prev: f32,
    exp_factor: f32,
}

#[derive(Debug)]
enum State {
    Rest,
    Attack,
    Decay,
    Release,
}

impl Envelope {
    pub fn new(sample_rate: u32, params: Adsr) -> Self {
        let mut instance = Envelope {
            params,
            factors: [0.0, 0.0, 0.0],
            state: State::Rest,
            velocity: 0.0,
            prev: 0.0,
            exp_factor: -std::f32::consts::TAU * 1000.0 / (sample_rate as f32),
        };

        instance.update_factors();
        instance
    }

    /// Writes the envelope output to `buffer`, replacing all values  
    ///
    /// Use this function as an envelope generator
    /// to be applied to multiple destinations  
    ///
    /// If you only need to apply this envelope as gain
    /// to an interleaved stereo buffer, use [`process_apply`](#method.process_apply)
    ///
    /// # Warning
    /// - Call only once per vector
    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer {
            *sample = self.process_sample();
        }
    }

    /// Multiplies all values in a `buffer` (interleaved stereo) by the envelope
    ///
    /// If you want to apply the same envelope
    /// to multiple buffers, use [`process`](#method.process)
    pub fn process_apply(&mut self, buffer: &mut [f32]) {
        buffer.chunks_mut(2).for_each(|frame| {
            let envelope = self.process_sample();
            frame[0] *= envelope;
            frame[1] *= envelope;
        });
    }

    /// Messages the Envelope to enter one of two states:
    ///  - `On{ velocity: f32 }` -> Begins attack phase (from any value/state - without jumping)
    ///  - `Off` -> Begins release phase (from any value/state - without jumping)
    pub fn message(&mut self, message: Message) {
        match message {
            Message::On { velocity } => {
                self.state = State::Attack;
                self.velocity = velocity;
            }
            Message::Off => {
                self.state = State::Release;
            }
        }
    }

    fn params(&mut self, params: Adsr) -> &mut Self {
        self.params = params;
        self.update_factors();
        self
    }

    fn update_factors(&mut self) {
        self.factors = [
            self.calculate_limited_cte(self.params.attack),   // up
            -self.calculate_limited_cte(self.params.decay),   // down
            -self.calculate_limited_cte(self.params.release), // down
        ]
    }

    fn calculate_limited_cte(&mut self, time_ms: f32) -> f32 {
        if time_ms < 0.001 {
            return 0.0;
        }
        (self.exp_factor / time_ms).exp()
    }

    fn process_sample(&mut self) -> f32 {
        if let State::Rest = self.state {
            self.prev
        } else {
            let (factor, target) = match self.state {
                State::Attack => {
                    let velocity = self.velocity;
                    if self.prev >= velocity {
                        if self.prev > velocity + 0.001 {
                            (-self.factors[0], velocity)
                        }else {
                            self.state = State::Decay;
                            self.prev = velocity;
                            (1.0, velocity)
                        }
                    } else {
                        (self.factors[0], velocity)
                    }
                }
                State::Decay => {
                    let sustain = self.params.sustain * self.velocity;
                    if self.prev <= sustain {
                        self.state = State::Rest;
                        self.prev = sustain;
                        (1.0, sustain)
                    } else {
                        (self.factors[1], sustain)
                    }
                }
                State::Release => {
                    if self.prev <= 0.0 {
                        self.state = State::Rest;
                        self.prev = 0.0;
                        (1.0, 0.0)
                    } else {
                        (self.factors[2], 0.0)
                    }
                }
                _ => (1.0, self.prev),
            };

            let diff = (self.prev - target).abs();

            // Assure target if diff is small enough
            let diff = if diff < 0.0001 { 0.0 } else { diff };

            let envelope = target + factor * -diff;
            self.prev = envelope;

            envelope
        }
    }
}