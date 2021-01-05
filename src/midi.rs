use crate::io_utils::read_input;
#[derive(Debug)]
pub struct Note(u8, u8, u8);

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
pub enum ControlChange {
    Normal(u8, u8, u8),
    ChannelMode(u8, u8, u8),
}

#[derive(Debug)]
pub enum MidiMessage {
    NoteOn(Note),
    NoteOff(Note),
    ProgramChange(u8, u8),
    ControlChange(ControlChange),
    PitchBend(u8, u16),
}

const NOTE_OFF: u8 = 0b1000;
const NOTE_ON: u8 = 0b1001;
const _POLYPHONIC_AFTER_TOUCH: u8 = 0b1010;
const CONTROL_CHANGE: u8 = 0b1011;
const PROGRAM_CHANGE: u8 = 0b1100;
const _AFTER_TOUCH: u8 = 0b1101;
const PITCH_BEND_CHANGE: u8 = 0b1110;

fn split_status_and_channel(status_byte: u8) -> (u8, u8) {
    let channel = status_byte & 0b00001111;
    let status = status_byte >> 4;
    (status, channel)
}

impl MidiMessage {
    pub fn try_new(raw_message: &[u8]) -> Result<Self, &str> {
        use MidiMessage::*;
        let (status, channel) = split_status_and_channel(raw_message[0]);

        match status {
            NOTE_OFF => Ok(NoteOff(Note(channel, raw_message[1], raw_message[2]))),
            NOTE_ON => Ok(NoteOn(Note(channel, raw_message[1], raw_message[2]))),
            CONTROL_CHANGE => {
                use self::ControlChange::*;
                let cc_number = raw_message[1];
                Ok(ControlChange(if cc_number <= 119 {
                    Normal(channel, raw_message[1], raw_message[2])
                } else {
                    ChannelMode(channel, raw_message[1], raw_message[2])
                }))
            }
            PITCH_BEND_CHANGE => {
                let msb = (raw_message[2] as u16) << 7;
                let lsb = raw_message[1] as u16;
                let pitchbend_value = msb | lsb;
                Ok(PitchBend(channel, pitchbend_value))
            }
            PROGRAM_CHANGE => Ok(ProgramChange(channel, raw_message[1])),
            _ => Err("Unrecognized message"),
        }
    }
}

const DEBUG_MIDI: bool = false;

use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use std::error::Error;

fn get_ports(
    midi_in: &MidiInput,
    print: bool,
) -> Result<std::vec::Vec<midir::MidiInputPort>, Box<dyn Error>> {
    if print {
        println!("Available input ports:");
        for (i, p) in midi_in.ports().iter().enumerate() {
            println!("{}: {}", i, midi_in.port_name(p)?);
        }
    }

    Ok(midi_in.ports())
}

pub struct MidiConnection {
    port: MidiInputPort,
    port_name: String,
    midi_in: Option<MidiInput>,
}

fn port_prompt(ports: &Vec<MidiInputPort>, re_ask: Option<()>) -> usize {
    let mut choice: usize = 0;
    if let None = re_ask {
        println!("Choose your midi input.");
    }
    match read_input().expect("Couldn't read input.").parse::<usize>() {
        Ok(c) => {
            if c < ports.len() {
 
                choice = c;
            } else {
                println!("{}: Not a valid port number", c);
                port_prompt(ports, Some(()));
            }
        }
        Err(_) => {
            println!("{}: Not a valid port number", c);
            port_prompt(ports, Some(()));
        }
    };
    choice
}

impl MidiConnection {
    pub fn try_new() -> Result<MidiConnection, Box<dyn Error>> {
        let mut midi_in = MidiInput::new("rust-synth input")?;
        midi_in.ignore(Ignore::None);

        let ports = get_ports(&midi_in, true)?;
        let port = port_prompt(&ports, None);

        let port = ports.get(port).ok_or("port not found")?.clone();
        let port_name = midi_in.port_name(&port)?;

        println!("Selecting: {}", port_name);

        Ok(MidiConnection {
            port,
            port_name,
            midi_in: Some(midi_in),
        })
    }

    pub fn connect<F, T: Send + 'static>(
        &mut self,
        mut callback: F,
        context: T,
    ) -> Result<MidiInputConnection<T>, Box<dyn Error>>
    where
        F: FnMut(MidiMessage, &mut T) + Send + 'static,
    {
        let connection_status = self
            .midi_in
            .take()
            .ok_or("A connection is already open")?
            .connect(
                &self.port,
                &self.port_name,
                move |micros, raw_message, context| {
                    let message = MidiMessage::try_new(raw_message);
                    if DEBUG_MIDI {
                        println!("=============================\n");
                        println!("Microseconds: {}\n", micros);
                        println!("Raw Message: {:?}\n", raw_message);
                        println!("Message: {:#?}\n", message);
                    }
                    match MidiMessage::try_new(raw_message) {
                        Err(err) => eprintln!("{}", err),
                        Ok(message) => callback(message, context),
                    }
                },
                context,
            );
        Ok(connection_status?)
    }
}
