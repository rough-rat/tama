use embedded_graphics::{
    Drawable as _,
    mono_font::{MonoTextStyleBuilder, ascii::{FONT_4X6, FONT_8X13}},
    prelude::{DrawTarget, Point, RgbColor},
    text::{Alignment, Text},
};
use heapless::vec;

use crate::{
    consts, scenes::{Scene, SceneWrapper, UpdateResult, menu::MenuScene}
};

struct TestEntry {
    name: &'static str,
    delay_ms: u32,
}

const TEST_ENTRIES: &[TestEntry] = &[
    TestEntry { name: "init display...", delay_ms: 300 },
    TestEntry { name: "init thermometer...", delay_ms: 100 },
    TestEntry { name: "calibrating light sensor...", delay_ms: 400 },
    TestEntry { name: "agressively rubbing the rat...", delay_ms: 1200 },
];

const FINAL_DELAY_MS: u32 = 3000;

pub struct SelfTestScene {
    elapsed_ms: u32,
    current_test: usize,
    test_start_time: u32,
}

impl SelfTestScene {
    pub fn new() -> Self {
        Self {
            elapsed_ms: 0,
            current_test: 0,
            test_start_time: 0,
        }
    }
}

fn get_music_samples() -> heapless::Vec<(u32, u32), 10> {
    let frequencies  = [293, 329, 349, 329, 293, 261, 261, 261, 261, 261];

    // build a heapless Vec of (frequency, duration_ms) pairs; capacity 16 is enough for 15 entries
    let mut samples: heapless::Vec<(u32, u32), 10> = heapless::Vec::new();
    for &f in frequencies.iter() {
        // use a fixed duration per note (adjust as needed)
        let _ = samples.push((f, 50));
    }

    samples
}

static mut notes_played: u32 = 0;


impl Scene for SelfTestScene {
    fn update(&mut self, _ctx: &mut crate::engine::Context) -> UpdateResult {
        self.elapsed_ms += 32;
        
        // Check if current test is complete
        if self.current_test < TEST_ENTRIES.len() {
            let current_entry = &TEST_ENTRIES[self.current_test];
            let test_elapsed = self.elapsed_ms - self.test_start_time;
            
            if test_elapsed >= current_entry.delay_ms {
                // Move to next test
                log::debug!("Self-test: {} completed", current_entry.name);
                self.current_test += 1;
                self.test_start_time = self.elapsed_ms;

                _ctx.output.play_tone(230, 32); // Play beep on test completion
            }
        } else {
            // All tests completed, wait for final delay then transition
            let test_elapsed = self.elapsed_ms - self.test_start_time;

            let samples = get_music_samples();

            unsafe{
                _ctx.output.play_tone(samples[(notes_played/3) as usize].0, samples[(notes_played/3) as usize].1);
                notes_played += 1;

                if notes_played >= (samples.len() as u32)*3 {
                    return UpdateResult::ChangeScene(SceneWrapper::from(MenuScene::new()));
                }
            }


            if test_elapsed >= FINAL_DELAY_MS {
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

        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_4X6)
            .text_color(consts::ColorType::RED)
            .build();

        // Title
        Text::with_alignment(
            "Self-Test",
            Point::new(consts::WIDTH as i32 / 2, 20),
            text_style,
            Alignment::Center,
        )
        .draw(target)?;

        let start_y = 50;
        let line_height = 15;

        // Draw test lines based on current progress
        for (i, test_entry) in TEST_ENTRIES.iter().enumerate() {
            if i < self.current_test {
                // Test completed - show with [ok]
                let mut text = heapless::String::<64>::new();
                let _ = text.push_str(test_entry.name);
                let _ = text.push_str(" [ok]");
                
                Text::new(
                    text.as_str(),
                    Point::new(20, start_y + (i as i32 * line_height)),
                    text_style,
                )
                .draw(target)?;
            } else if i == self.current_test {
                // Current test in progress - show without [ok]
                Text::new(
                    test_entry.name,
                    Point::new(20, start_y + (i as i32 * line_height)),
                    text_style,
                )
                .draw(target)?;
            }
        }

        // If all tests are complete, show "Rough Rat" in larger font
        if self.current_test >= TEST_ENTRIES.len() {
            let large_text_style = MonoTextStyleBuilder::new()
                .font(&FONT_8X13)
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
