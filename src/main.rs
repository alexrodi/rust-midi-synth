use std::error::Error;
use midir::{MidiInput, Ignore};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use priomutex::spin_one::Mutex;

const DEBUG: bool = false;

#[derive(Debug)]
struct Synth {
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

#[derive(Debug)]
struct Note(u8, u8, u8);

impl Note {
    pub fn frequency(&self) -> f32 {
        let note = self.1 as f32;
        let base_frequency = 440.0;
        2.0_f32.powf((note - 69.0) / 12.0) * base_frequency
    }

    pub fn gain(&self) -> f32 {
        let velocity = self.2 as f32;
        let db = velocity / 127.0 * 70.0 - 70.0;
        10.0_f32.powf(db / 20.0)
    }
}

#[derive(Debug)]
enum MidiMessage {
    NoteOn(Note),
    NoteOff(Note),
    ProgramChange(u8, u8),
    ControlChange(u8, u8, u8)
}

impl MidiMessage {
    pub fn new(raw_message: &[u8]) -> Result<Self, &str> {
        let channel = raw_message[0] & 0b00001111;
        let status = raw_message[0] >> 4;

        use MidiMessage::*;
        
        match status {
            0b1000 => Ok(NoteOff(Note(channel, raw_message[1], raw_message[2]))),
            0b1001 => Ok(NoteOn(Note(channel, raw_message[1], raw_message[2]))),
            0b1011 => Ok(ControlChange(channel, raw_message[1], raw_message[2])),
            0b1100 => Ok(ProgramChange(channel, raw_message[1])),
            _ => Err("Unrecognized message")
        }
    }
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => eprintln!("Error: {}", err)
    }
}

fn remove_debug_content(mut string: String) -> String {
    let mut found_bracket = false;
    string.retain(|c| 
        !found_bracket && {
            found_bracket = c == '{' || c == '(';
            !found_bracket
        }
    );
    string
}

fn run() -> Result<(), Box<dyn Error>> {

    //=======
    // AUDIO
    //=======

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or("failed to find a default output device")?;
    let config = device.default_output_config()?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let synth = Arc::new(Mutex::new(Synth::new(sample_rate)));

    let synth_clone = Arc::clone(&synth);

    let stream = device.build_output_stream(
        &cpal::StreamConfig::from(config),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut synth = synth_clone.lock(0).unwrap(); // Lock mutex with highest priority
            synth.process(channels, data);
        },
        |err| eprintln!("an error occurred on stream: {}", err),
    )?;
    stream.play()?;

    //=======
    // MIDI
    //=======

    // Port

    let mut midi_in = MidiInput::new("midir test input")?;
    midi_in.ignore(Ignore::None);
    
    println!("Available input ports:");
    for (i, p) in midi_in.ports().iter().enumerate() {
        println!("{}: {}", i, midi_in.port_name(p)?);
    }

    let ports = midi_in.ports();

    let port = ports.get(0).ok_or("port not found")?;
    let port_name = midi_in.port_name(port)?;

    println!("Selecting: {}", port_name);

    // Message

    let synth_clone = Arc::clone(&synth);
    
    let _connection_status = midi_in.connect(
        port,
        &port_name,
        |micros, message, context| {
            if DEBUG {
                println!("=============================\n");
                println!("Microseconds: {}\n", micros);
                println!("Raw Message: {:?}\n", message);
                println!("Context: {:#?}\n", context);
                println!("Message: {:#?}\n", MidiMessage::new(message));
            }
            match MidiMessage::new(message) {
                Ok(MidiMessage::NoteOn(note)) => {
                    let mut context = context.lock(1).unwrap(); // Lock mutex with reduced priority
                    context.frequency(note.frequency());
                    context.gain(note.gain());
                },
                Ok(message) => println!("Were're still working on {}!", remove_debug_content(format!("{:?}", message))),
                Err(err) => eprintln!("{}", err)
            }
        },
        synth_clone
    )?;

    std::thread::park();
    Ok(())
}