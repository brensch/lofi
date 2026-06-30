use eframe::egui;
use lofi_app::display::{render, DisplayState, Lcd, LCD_HEIGHT, LCD_WIDTH};

/// Paint a device's LCD by rasterizing its [`DisplayState`] through the exact
/// framebuffer code the firmware uses, then drawing the lit pixels.
pub fn draw_lcd(ui: &mut egui::Ui, display: &DisplayState, scale: f32) -> egui::Response {
    let mut lcd = Lcd::new();
    render(display, &mut lcd);

    let size = egui::vec2(LCD_WIDTH as f32 * scale, LCD_HEIGHT as f32 * scale);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(6, 14, 10));

    let on = egui::Color32::from_rgb(120, 240, 170);
    for y in 0..LCD_HEIGHT {
        for x in 0..LCD_WIDTH {
            if lcd.pixel(x, y) {
                let min = egui::pos2(
                    rect.left() + x as f32 * scale,
                    rect.top() + y as f32 * scale,
                );
                let px = egui::Rect::from_min_size(min, egui::vec2(scale, scale));
                painter.rect_filled(px, 0.0, on);
            }
        }
    }
    response
}
