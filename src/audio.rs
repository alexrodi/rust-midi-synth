use std::error::Error;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct Stream {
    device: cpal::Device,
    config: cpal::StreamConfig,
    stream: Option<cpal::Stream>
}

impl Stream {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or("failed to find a default output device")?;
        let config = device.default_output_config()?;

        Ok(Self {
            device,
            config: cpal::StreamConfig::from(config),
            stream: None
        })
    }

    pub fn output_stream<F: Send>(&mut self,  callback: F) -> Result<(), Box<dyn Error>>
    where
        F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static {
        
        self.stream = Some(self.device.build_output_stream(
            &self.config,
            callback,
            |err| eprintln!("an error occurred on stream: {}", err),
        )?);
        // It's safe to unwrap because we just created it (or returned early with an error)
        self.stream.as_ref().unwrap().play()?;

        Ok(())
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn channels(&self) -> usize {
        self.config.channels as usize
    }
}