use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;
use rodio::{OutputStream, OutputStreamHandle, Source};
use tama_core::buzzer::BuzzerTrait;

// Square wave generator
struct SquareWave {
    frequency: f32,
    sample_rate: u32,
    num_samples: usize,
    current_sample: usize,
}

impl SquareWave {
    fn new(frequency: f32, sample_rate: u32) -> Self {
        Self {
            frequency,
            sample_rate,
            num_samples: 0,
            current_sample: 0,
        }
    }
    
    fn take_duration(mut self, duration: Duration) -> Self {
        self.num_samples = (duration.as_secs_f32() * self.sample_rate as f32) as usize;
        self
    }
}

impl Iterator for SquareWave {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.num_samples > 0 && self.current_sample >= self.num_samples {
            return None;
        }
        
        let sample_position = self.current_sample as f32 / self.sample_rate as f32;
        let cycle_position = (sample_position * self.frequency) % 1.0;
        
        self.current_sample += 1;
        
        // Square wave: high for first half of cycle, low for second half
        if cycle_position < 0.5 {
            Some(0.15)  // Amplitude
        } else {
            Some(-0.15)
        }
    }
}

impl Source for SquareWave {
    fn current_frame_len(&self) -> Option<usize> {
        if self.num_samples > 0 {
            Some(self.num_samples - self.current_sample)
        } else {
            None
        }
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        if self.num_samples > 0 {
            Some(Duration::from_secs_f32(self.num_samples as f32 / self.sample_rate as f32))
        } else {
            None
        }
    }
}

pub struct BuzzerCommand {
    pub frequency_hz: u32,
    pub duration_ms: u32,
}

pub struct DesktopBuzzer {
    command_tx: Sender<BuzzerCommand>,
}

impl DesktopBuzzer {
    pub fn new() -> Self {
        let (tx, rx) = channel::<BuzzerCommand>();
        
        // Spawn a thread to handle audio playback
        thread::spawn(move || {
            buzzer_thread(rx);
        });
        
        Self {
            command_tx: tx,
        }
    }
}

impl BuzzerTrait for DesktopBuzzer {
    fn beep(&self, frequency_hz: u32, duration_ms: u32) {
        // Send the beep command asynchronously, ignore errors if channel is closed
        let _ = self.command_tx.send(BuzzerCommand {
            frequency_hz,
            duration_ms,
        });
    }
}

fn buzzer_thread(rx: Receiver<BuzzerCommand>) {
    // Initialize audio output once for the thread
    let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
        eprintln!("Failed to initialize audio output for buzzer");
        return;
    };
    
    // Process beep commands from the channel
    while let Ok(cmd) = rx.recv() {
        play_beep(&stream_handle, cmd.frequency_hz, cmd.duration_ms);
    }
}

fn play_beep(stream_handle: &OutputStreamHandle, frequency_hz: u32, duration_ms: u32) {
    let sample_rate = 48000; // Standard audio sample rate
    let source = SquareWave::new(frequency_hz as f32, sample_rate)
        .take_duration(Duration::from_millis(duration_ms as u64));
    
    // Play the sound (non-blocking)
    if let Err(e) = stream_handle.play_raw(source.convert_samples()) {
        eprintln!("Failed to play beep: {}", e);
        return;
    }
    
    // Sleep to allow the sound to complete before processing next command
    thread::sleep(Duration::from_millis(duration_ms as u64));
}
