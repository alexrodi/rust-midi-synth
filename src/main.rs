use crossbeam::epoch::{pin, Atomic};
use std::error::Error;
use std::sync::Arc;
mod synth;
use synth::Synth;
mod midi;
use midi::{ControlChange, MidiConnection, MidiMessage};
mod audio;
use audio::Stream;
mod io_utils;

macro_rules! access_atomic {
    ($variable_name:ident) => {
        let guard = &pin();
        let mut p = $variable_name.load_consume(guard);
        let $variable_name = unsafe { p.deref_mut() };
    };
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => eprintln!("Error: {}", err),
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // Audio
    //=======================================================

    let mut stream = Stream::try_new()?;

    let sample_rate = stream.sample_rate();
    let channels = stream.channels();
    let synth = Arc::new(Atomic::new(Synth::new(sample_rate)));

    let clone = Arc::clone(&synth);
    stream.output_stream(move |buffer: &mut [f32], _: &cpal::OutputCallbackInfo| {
        access_atomic!(clone);
        clone.process(channels, buffer);
    })?;

    // MIDI
    //=======================================================

    let mut midi_connection = MidiConnection::try_new()?;

    let _connection = midi_connection.connect(
        |message, context| {
            match message {
                MidiMessage::NoteOn(note) => {
                    access_atomic!(context);
                    context.frequency(note.frequency());
                    context.message_envelope(synth::envelope::Message::On {
                        velocity: note.gain(),
                    });
                }
                MidiMessage::NoteOff(_note) => {
                    access_atomic!(context);
                    context.message_envelope(synth::envelope::Message::Off);
                }
                MidiMessage::ControlChange(control_change) => match control_change {
                    ControlChange::Normal(channel, cc_number, value) => {}
                    ControlChange::ChannelMode(channel, cc_number, value) => {
                        unimplemented!()
                    }
                },
                MidiMessage::PitchBend(channel, value) => {
                    access_atomic!(context);
                    // 0 - 32767 Range
                    context.pitchbend_cents((value as f32 - 8192.0) * 0.1);
                }
                message => println!(
                    "Were're still working on {}!",
                    debug_struct_name(format!("{:?}", message))
                ),
            }
        },
        Arc::clone(&synth),
    )?;

    // Thread
    //=======================================================

    std::thread::park();
    Ok(())
}

/// Small utility for retaining the struct name from a debug format.
/// - Disposes of all content and brackets or parenthesis
///
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
    let found_bracket = string.find(|c| c == '{' || c == '(');
    if let Some(index) = found_bracket {
        string = string.split_at(index).0.to_string();
    }
    string
}
