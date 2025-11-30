use alloc::vec::Vec;
use embedded_graphics::{
    Drawable as _,
    mono_font::{MonoTextStyleBuilder, ascii::{FONT_6X10, FONT_10X20}},
    prelude::{DrawTarget, Point, RgbColor},
    text::{Alignment, Text},
};

use crate::{
    consts, 
    log_buffer::LogEntry,
    scenes::{Scene, SceneWrapper, UpdateResult, menu::MenuScene}
};

/// Duration to show logs before transitioning to RoughRat display
const LOG_DISPLAY_MS: u32 = 2000;
/// Duration to show RoughRat before transitioning to menu
const FINAL_DELAY_MS: u32 = 3000;
/// Maximum number of log lines we can display
const MAX_LOG_LINES: usize = 16;

pub struct SelfTestScene {
    elapsed_ms: u32,
    /// Cached log entries for display (updated during update())
    log_entries: Vec<LogEntry>,
}

impl SelfTestScene {
    pub fn new() -> Self {
        Self {
            elapsed_ms: 0,
            log_entries: Vec::new(),
        }
    }
}

fn get_music_samples() -> heapless::Vec<(u32, u32), 10> {
    let frequencies  = [293, 329, 349, 329, 293, 261, 261, 261, 261, 261];

    let mut samples: heapless::Vec<(u32, u32), 10> = heapless::Vec::new();
    for &f in frequencies.iter() {
        let _ = samples.push((f, 50));
    }

    samples
}

static mut NOTES_PLAYED: u32 = 0;


impl Scene for SelfTestScene {
    fn update(&mut self, ctx: &mut crate::engine::Context) -> UpdateResult {
        self.elapsed_ms += 32;
        
        // During log display phase, cache the log entries for draw()
        if self.elapsed_ms < LOG_DISPLAY_MS {
            // Take the most recent entries that fit on screen
            self.log_entries = ctx.log_entries.iter()
                .rev()
                .take(MAX_LOG_LINES)
                .cloned()
                .collect::<Vec<_>>();
            self.log_entries.reverse(); // Put back in chronological order
        }
        
        // After log display phase, play music and transition to menu
        if self.elapsed_ms >= LOG_DISPLAY_MS {
            let samples = get_music_samples();

            unsafe {
                ctx.output.play_tone(samples[(NOTES_PLAYED/3) as usize].0, samples[(NOTES_PLAYED/3) as usize].1);
                NOTES_PLAYED += 1;

                if NOTES_PLAYED >= (samples.len() as u32)*3 {
                    return UpdateResult::ChangeScene(SceneWrapper::from(MenuScene::new()));
                }
            }

            if self.elapsed_ms >= LOG_DISPLAY_MS + FINAL_DELAY_MS {
                return UpdateResult::ChangeScene(SceneWrapper::from(MenuScene::new()));
            }
        }
        
        UpdateResult::None
    }

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = consts::ColorType>,
    {
        target.clear(consts::ColorType::BLACK)?;

        if self.elapsed_ms < LOG_DISPLAY_MS {
            // Log display phase - show captured log entries
            let text_style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(consts::ColorType::RED)
                .build();

            let start_y = 12;
            let line_height = 11;
            let mut current_line = 0;
            
            // Draw each log entry, handling newlines within messages
            for entry in self.log_entries.iter() {
                // Split message on newlines
                for (part_idx, part) in entry.message.as_str().split('\n').enumerate() {
                    let y = start_y + (current_line as i32 * line_height);
                    
                    // Skip if off screen
                    if y > consts::HEIGHT as i32 {
                        break;
                    }
                    
                    // Format: "[L] message" for first part, "    message" for continuation
                    let mut line = heapless::String::<90>::new();
                    use core::fmt::Write;
                    if part_idx == 0 {
                        let _ = write!(line, "[{}] {}", entry.level.prefix(), part);
                    } else {
                        let _ = write!(line, "    {}", part);
                    }
                    
                    Text::new(
                        line.as_str(),
                        Point::new(2, y),
                        text_style,
                    )
                    .draw(target)?;
                    
                    current_line += 1;
                }
            }
            
        } else {
            // RoughRat display phase
            let large_text_style = MonoTextStyleBuilder::new()
                .font(&FONT_10X20)
                .text_color(consts::ColorType::RED)
                .build();

            Text::with_alignment(
                "Rough Rat",
                Point::new(consts::WIDTH as i32 / 2, consts::HEIGHT as i32 / 2 + 20),
                large_text_style,
                Alignment::Center,
            )
            .draw(target)?;
        }

        Ok(())
    }
}
