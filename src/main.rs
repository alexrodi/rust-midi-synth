use std::error::Error;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use priomutex::spin_one::Mutex;
mod synth;
use synth::Synth;
mod midi;
use midi::{MidiConnection, MidiMessage};

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => eprintln!("Error: {}", err)
    }
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

    let mut midi_connection = MidiConnection::new(0)?;

    midi_connection.connect(
        |message, context| {
            match message {
                MidiMessage::NoteOn(note) => {
                    let mut context = context.lock(1).unwrap(); // Lock mutex with reduced priority
                    context.frequency(note.frequency());
                    context.gain(note.gain());
                },
                message => println!("Were're still working on {}!", remove_debug_content(format!("{:?}", message)))
            }
        },
        Arc::clone(&synth)
    )?;
    
    std::thread::park();
    Ok(())
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