
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
pub enum MidiMessage {
    NoteOn(Note),
    NoteOff(Note),
    ProgramChange(u8, u8),
    ControlChange(u8, u8, u8)
}

impl MidiMessage {
    pub fn try_new(raw_message: &[u8]) -> Result<Self, &str> {
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

const DEBUG_MIDI: bool = false;

use std::error::Error;
use midir::{MidiInput, MidiInputPort, MidiInputConnection, Ignore};

fn get_ports(midi_in: &MidiInput, print: bool) -> Result<std::vec::Vec<midir::MidiInputPort>, Box<dyn Error>> {
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
    midi_in: Option<MidiInput>
}

impl MidiConnection {
    pub fn try_new(port: usize) -> Result<MidiConnection, Box<dyn Error>> {
        let mut midi_in = MidiInput::new("rust-synth input")?;
        midi_in.ignore(Ignore::None);
        
        let ports = get_ports(&midi_in, true)?;
    
        let port = ports.get(port).ok_or("port not found")?.clone();
        let port_name = midi_in.port_name(&port)?;
    
        println!("Selecting: {}", port_name);
    
        Ok( MidiConnection{
            port,
            port_name,
            midi_in: Some(midi_in)
        })
    }

    pub fn connect<F, T: Send + 'static>(&mut self, mut callback: F, context: T) -> Result<MidiInputConnection<T>, Box<dyn Error>>
    where
        F: FnMut(MidiMessage, &mut T) + Send + 'static,
    {
        let connection_status = 
        self.midi_in.take()
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
                    Ok(message) => callback(message, context)
                }
            },
            context
        );
        Ok(connection_status?)
    }

}