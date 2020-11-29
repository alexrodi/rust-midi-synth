use std::error::Error;
use std::sync::Arc;
use priomutex::spin_one::Mutex;
mod synth;
use synth::Synth;
mod midi;
use midi::{MidiConnection, MidiMessage};
mod audio;
use audio::Stream;

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => eprintln!("Error: {}", err)
    }
}

fn run() -> Result<(), Box<dyn Error>> {

    // Audio
    //=======================================================

    let mut stream = Stream::try_new()?;

    let sample_rate = stream.sample_rate();
    let channels = stream.channels();
    let synth = Arc::new(Mutex::new(Synth::new(sample_rate)));

    let synth_clone = Arc::clone(&synth);
    stream.output_stream(
        move |buffer: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut synth = synth_clone.lock(0).unwrap(); // Lock mutex with highest priority
            synth.process(channels, buffer);
        }
    )?;

    // MIDI
    //=======================================================

    let mut midi_connection = MidiConnection::try_new(0)?;

    let _connection = midi_connection.connect(
        |message, context| {
            match message {
                MidiMessage::NoteOn(note) => {
                    let mut context = context.lock(1).unwrap(); // Lock mutex with reduced priority
                    context.frequency(note.frequency());
                    context.gain(note.gain());
                },
                message => println!("Were're still working on {}!", debug_struct_name(format!("{:?}", message)))
            }
        },
        Arc::clone(&synth)
    )?;

    // Thread
    //=======================================================
    
    std::thread::park();
    Ok(())
}

/// A little utility for retaining the struct name from a debug format
/// # Usage
/// ```
/// #[derive(Debug)]
/// struct AGreatTuple(f32);
/// #[derive(Debug)]
/// struct AnAwesomeStruct{ v: f32 };
/// 
/// let a = AGreatTuple(0.0);
/// let b = AnAwesomeStruct{ v: 0.0 };
/// 
/// debug_struct_name(format!("{:?}", a)); // = "AGreatTuple"
/// debug_struct_name(format!("{:?}", b)); // = "AnAwesomeStruct"
/// ```
fn debug_struct_name(mut string: String) -> String {
    let mut found_bracket = false;
    string.retain(|c| 
        !found_bracket && {
            found_bracket = c == '{' || c == '(';
            !found_bracket
        }
    );
    string
}