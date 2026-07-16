//! The shared LCD model.
//!
//! [`DisplayState`] is the data; [`Lcd`] is a 128x64 1-bpp framebuffer matching
//! a cheap SSD1306 OLED. [`render`] rasterizes one into the other. Firmware
//! ships the same `Lcd` bytes to the panel over I2C; the simulator draws the
//! same pixels on screen, so the lab view is faithful to the hardware.

use lofi_core::music::{Codename, Role};
use lofi_core::Micros;

use crate::font::{glyph, GLYPH_W};

pub const LCD_WIDTH: usize = 128;
pub const LCD_HEIGHT: usize = 64;
const STRIDE: usize = LCD_WIDTH / 8;

/// Everything the LCD shows, captured from a [`crate::device::Device`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayState {
    pub node_id: u32,
    pub playing: bool,
    pub bpm_milli: u32,
    /// This box's job in the band.
    pub role: Role,
    /// Coined name of the current arrangement combination.
    pub codename: Codename,
    /// The arrangement coming next phrase.
    pub next_codename: Codename,
    pub bars_to_next: u8,
    pub change_in_millis: u32,
    pub peers: u8,
    pub sync_error_us: Micros,
    /// Position within the current bar, 0..1000.
    pub beat_phase_milli: u16,
}

/// A 1-bit-per-pixel 128x64 framebuffer, row-major, MSB-left.
#[derive(Clone)]
pub struct Lcd {
    pixels: [u8; STRIDE * LCD_HEIGHT],
}

impl Default for Lcd {
    fn default() -> Self {
        Self::new()
    }
}

impl Lcd {
    pub const fn new() -> Self {
        Self {
            pixels: [0; STRIDE * LCD_HEIGHT],
        }
    }

    pub fn clear(&mut self) {
        self.pixels = [0; STRIDE * LCD_HEIGHT];
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn pixel(&self, x: usize, y: usize) -> bool {
        if x >= LCD_WIDTH || y >= LCD_HEIGHT {
            return false;
        }
        let byte = self.pixels[y * STRIDE + (x >> 3)];
        (byte >> (7 - (x & 7))) & 1 == 1
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, on: bool) {
        if x >= LCD_WIDTH || y >= LCD_HEIGHT {
            return;
        }
        let mask = 1u8 << (7 - (x & 7));
        let byte = &mut self.pixels[y * STRIDE + (x >> 3)];
        if on {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, on: bool) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, on);
            }
        }
    }

    pub fn frame_rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        for dx in 0..w {
            self.set_pixel(x + dx, y, true);
            self.set_pixel(x + dx, y + h - 1, true);
        }
        for dy in 0..h {
            self.set_pixel(x, y + dy, true);
            self.set_pixel(x + w - 1, y + dy, true);
        }
    }

    /// Draw one glyph at the given scale. Returns the x advance.
    fn draw_glyph(&mut self, x: usize, y: usize, c: char, scale: usize) -> usize {
        let rows = glyph(c);
        for (ry, bits) in rows.iter().enumerate() {
            for cx in 0..GLYPH_W {
                if (bits >> (GLYPH_W - 1 - cx)) & 1 == 1 {
                    self.fill_rect(x + cx * scale, y + ry * scale, scale, scale, true);
                }
            }
        }
        (GLYPH_W + 1) * scale
    }

    /// Draw text. Returns the x just past the last glyph.
    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, scale: usize) -> usize {
        let mut cursor = x;
        for c in text.chars() {
            cursor += self.draw_glyph(cursor, y, c, scale);
        }
        cursor
    }

    /// Draw a signed integer (no allocation). Returns the x advance.
    pub fn draw_int(&mut self, x: usize, y: usize, value: i64, scale: usize) -> usize {
        let mut cursor = x;
        if value < 0 {
            self.draw_glyph(cursor, y, '-', scale);
            cursor += (GLYPH_W + 1) * scale;
        }
        let mut magnitude = value.unsigned_abs();

        // Collect digits most-significant first into a fixed buffer.
        let mut digits = [0u8; 20];
        let mut count = 0;
        if magnitude == 0 {
            digits[0] = 0;
            count = 1;
        } else {
            while magnitude > 0 && count < digits.len() {
                digits[count] = (magnitude % 10) as u8;
                magnitude /= 10;
                count += 1;
            }
        }
        for ix in (0..count).rev() {
            let c = (b'0' + digits[ix]) as char;
            cursor += self.draw_glyph(cursor, y, c, scale);
        }
        cursor - x
    }
}

/// Rasterize a [`DisplayState`] into the framebuffer.
pub fn render(state: &DisplayState, lcd: &mut Lcd) {
    lcd.clear();

    // Header: device id, and play/stop on the right.
    let mut x = lcd.draw_text(2, 1, "LOFI #", 1);
    lcd.draw_int(x, 1, state.node_id as i64, 1);
    let status = if state.playing { "PLAY" } else { "STOP" };
    lcd.draw_text(LCD_WIDTH - 4 * (GLYPH_W + 1) - 2, 1, status, 1);
    for px in 0..LCD_WIDTH {
        lcd.set_pixel(px, 10, true);
    }

    // Primary role, large enough to be the main performance cue.
    lcd.draw_text(2, 14, state.role.label(), 2);

    // Current and next coined arrangement identities.
    x = lcd.draw_text(2, 32, "NOW ", 1);
    lcd.draw_text(x, 32, state.codename.as_str(), 1);
    x = lcd.draw_text(2, 41, "NEXT ", 1);
    x = lcd.draw_text(x, 41, state.next_codename.as_str(), 1);
    x = lcd.draw_text(x + 4, 41, "-", 1);
    x += lcd.draw_int(x, 41, state.bars_to_next as i64, 1);
    lcd.draw_text(x, 41, "B", 1);

    // Peers and sync quality.
    x = lcd.draw_text(2, 51, "P", 1);
    x += lcd.draw_int(x, 51, state.peers as i64, 1);
    x = lcd.draw_text(x + 6, 51, "SYNC ", 1);
    let clamped = state.sync_error_us.clamp(-99_999, 99_999);
    x += lcd.draw_int(x, 51, clamped, 1);
    lcd.draw_text(x, 51, "US", 1);

    // Bar-position progress bar along the bottom.
    let width = (state.beat_phase_milli.min(1000) as usize * LCD_WIDTH) / 1000;
    lcd.fill_rect(0, 61, width, 3, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_some_pixels() {
        let state = DisplayState {
            node_id: 3,
            playing: true,
            bpm_milli: 90_000,
            role: Role::Pulse,
            codename: Codename::coin(123),
            next_codename: Codename::coin(456),
            bars_to_next: 7,
            change_in_millis: 20_250,
            peers: 4,
            sync_error_us: -42,
            beat_phase_milli: 500,
        };
        let mut lcd = Lcd::new();
        render(&state, &mut lcd);
        let lit = lcd.pixels().iter().filter(|b| **b != 0).count();
        assert!(lit > 0);
        // Progress bar at half should light the left edge but not the right.
        assert!(lcd.pixel(2, 62));
        assert!(!lcd.pixel(LCD_WIDTH - 2, 62));
    }
}
