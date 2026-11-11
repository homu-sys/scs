use crate::font::{FontSize, glyph_width};
use crate::time::LocalTime;
use std::fmt::Write as _;
use std::io::{self, Write};

const DEFAULT_FG: Rgb = Rgb::new(232, 232, 232);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn scale(self, factor: f32) -> Self {
        let scale = |value: u8| (value as f32 * factor).clamp(0.0, 255.0).round() as u8;
        Self::new(scale(self.r), scale(self.g), scale(self.b))
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Self::new(
            mix(self.r, other.r),
            mix(self.g, other.g),
            mix(self.b, other.b),
        )
    }

    pub fn fg(self) -> String {
        format!("\x1b[38;2;{};{};{}m", self.r, self.g, self.b)
    }

    pub fn bg(self) -> String {
        format!("\x1b[48;2;{};{};{}m", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    Neon,
    Sunset,
    Matrix,
    Ice,
    Candy,
    Fire,
    Ocean,
    Mono,
    Rainbow,
}

impl Palette {
    pub const ALL: [Palette; 9] = [
        Palette::Neon,
        Palette::Sunset,
        Palette::Matrix,
        Palette::Ice,
        Palette::Candy,
        Palette::Fire,
        Palette::Ocean,
        Palette::Mono,
        Palette::Rainbow,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::Neon => Self::Sunset,
            Self::Sunset => Self::Matrix,
            Self::Matrix => Self::Ice,
            Self::Ice => Self::Candy,
            Self::Candy => Self::Fire,
            Self::Fire => Self::Ocean,
            Self::Ocean => Self::Mono,
            Self::Mono => Self::Rainbow,
            Self::Rainbow => Self::Neon,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Neon => "NEON",
            Self::Sunset => "SUNSET",
            Self::Matrix => "MATRIX",
            Self::Ice => "ICE",
            Self::Candy => "CANDY",
            Self::Fire => "FIRE",
            Self::Ocean => "OCEAN",
            Self::Mono => "MONO",
            Self::Rainbow => "RAINBOW",
        }
    }

    pub fn color(self, t: f32, time: f32) -> Rgb {
        let t = (t + time * 0.045).rem_euclid(1.0);
        match self {
            Self::Neon => hsv((0.48 + t * 0.38).rem_euclid(1.0), 0.82, 1.0),
            Self::Sunset => hsv((0.96 + t * 0.16).rem_euclid(1.0), 0.88, 1.0),
            Self::Matrix => hsv(0.33 + t * 0.03, 0.75, 0.55 + t * 0.45),
            Self::Ice => hsv(0.50 + t * 0.12, 0.35, 1.0),
            Self::Candy => hsv((0.88 + t * 0.24).rem_euclid(1.0), 0.58, 1.0),
            Self::Fire => hsv((0.01 + t * 0.13).rem_euclid(1.0), 0.94, 1.0),
            Self::Ocean => hsv(0.55 + t * 0.16, 0.86, 0.74 + t * 0.26),
            Self::Mono => hsv(0.58, 0.06, 0.55 + t * 0.45),
            Self::Rainbow => hsv(t, 0.88, 1.0),
        }
    }

    pub fn accent(self, time: f32) -> Rgb {
        self.color(0.18, time)
    }

    pub fn dim(self, time: f32) -> Rgb {
        self.color(0.72, time).scale(0.48)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Option<Rgb>,
}

impl Cell {
    fn blank() -> Self {
        Self {
            ch: ' ',
            fg: DEFAULT_FG,
            bg: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
}

impl Frame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::blank(); width.saturating_mul(height)],
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::blank());
    }

    pub fn put(&mut self, x: i32, y: i32, ch: char, fg: Rgb, bg: Option<Rgb>) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return;
        }
        self.cells[y * self.width + x] = Cell { ch, fg, bg };
    }

    pub fn put_text(&mut self, x: i32, y: i32, text: &str, fg: Rgb) {
        for (offset, ch) in text.chars().enumerate() {
            self.put(x + offset as i32, y, ch, fg, None);
        }
    }

    pub fn put_text_clipped(&mut self, x: i32, y: i32, text: &str, fg: Rgb, max_width: i32) {
        if max_width <= 0 {
            return;
        }
        for (offset, ch) in text.chars().take(max_width as usize).enumerate() {
            self.put(x + offset as i32, y, ch, fg, None);
        }
    }

    fn put_bg_if_blank(&mut self, x: i32, y: i32, bg: Rgb) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return;
        }
        let cell = &mut self.cells[y * self.width + x];
        if cell.ch == ' ' && cell.bg.is_none() {
            cell.bg = Some(bg);
        }
    }
}

pub struct DiffRenderer {
    previous: Vec<Cell>,
    width: usize,
    height: usize,
}

impl Default for DiffRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffRenderer {
    pub fn new() -> Self {
        Self {
            previous: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn reset(&mut self) {
        self.previous.clear();
        self.width = 0;
        self.height = 0;
    }

    pub fn diff(&mut self, frame: &Frame) -> String {
        if self.width != frame.width || self.height != frame.height {
            self.width = frame.width;
            self.height = frame.height;
            self.previous = vec![Cell::blank(); frame.cells.len()];
        }

        let mut output = String::with_capacity(frame.cells.len() * 3);
        for y in 0..frame.height {
            let mut x = 0usize;
            while x < frame.width {
                let index = y * frame.width + x;
                if self.previous[index] == frame.cells[index] {
                    x += 1;
                    continue;
                }

                let run_start = x;
                while x < frame.width
                    && self.previous[y * frame.width + x] != frame.cells[y * frame.width + x]
                {
                    x += 1;
                }

                let _ = write!(output, "\x1b[{};{}H", y + 1, run_start + 1);
                let mut last_fg = None;
                let mut last_bg = None;
                for cell in &frame.cells[y * frame.width + run_start..y * frame.width + x] {
                    if last_fg != Some(cell.fg) {
                        output.push_str(&cell.fg.fg());
                        last_fg = Some(cell.fg);
                    }
                    if last_bg != Some(cell.bg) {
                        match cell.bg {
                            Some(bg) => output.push_str(&bg.bg()),
                            None => output.push_str("\x1b[49m"),
                        }
                        last_bg = Some(cell.bg);
                    }
                    output.push(cell.ch);
                }
                output.push_str("\x1b[0m");
            }
        }

        self.previous.copy_from_slice(&frame.cells);
        output
    }
}

pub fn render(
    frame: &mut Frame,
    now: &LocalTime,
    elapsed: f32,
    twelve_hour: bool,
    show_seconds: bool,
    palette: Palette,
) {
    frame.clear();
    let width = frame.width as i32;
    let height = frame.height as i32;
    let (clock_text, meridiem) = now.clock_text(twelve_hour, show_seconds);
    let (font, scale, extras) = choose_font(&clock_text, width, height);
    let clock_width = font.width(&clock_text) * scale;
    let clock_height = font.height() * scale;

    if clock_width > width - 2 || clock_height > height {
        center_text(frame, height / 2, &clock_text, palette.accent(elapsed));
        if height > 2 {
            center_text(
                frame,
                height / 2 + 1,
                &now.date_text(),
                palette.dim(elapsed),
            );
        }
        return;
    }

    let clock_x = (width - clock_width) / 2;
    let clock_y = if extras {
        ((height - clock_height) / 2 - 2).max(1)
    } else {
        (height - clock_height) / 2
    };

    draw_clock(
        frame,
        &clock_text,
        (clock_x, clock_y),
        font,
        scale,
        palette,
        elapsed,
    );

    if !extras {
        return;
    }

    let palette_label = palette.name();
    if width >= palette_label.len() as i32 + 12 {
        frame.put_text(
            width - palette_label.len() as i32 - 2,
            1,
            palette_label,
            palette.dim(elapsed),
        );
    }

    let date_y = clock_y + clock_height + 1;
    if date_y < height - 1 {
        let date = match meridiem {
            Some(meridiem) => format!("{}   {meridiem}", now.date_text()),
            None => now.date_text(),
        };
        center_text(
            frame,
            date_y,
            &date,
            palette.color(0.5, elapsed).scale(0.82),
        );
    }

    let bar_y = date_y + 2;
    if bar_y < height - 1 {
        let bar_width = clock_width.clamp(10, 60);
        draw_bar(
            frame,
            (width - bar_width) / 2,
            bar_y,
            bar_width,
            now.minute_progress(),
            palette,
            elapsed,
        );
    }

    let hint_y = height - 2;
    if hint_y >= 0 {
        let hint = if width >= 62 {
            "q quit   c theme   h 12/24   s seconds"
        } else {
            "q quit   c theme   h 12/24"
        };
        center_text(frame, hint_y, hint, palette.dim(elapsed));
    }
}

pub fn print_static(
    now: &LocalTime,
    twelve_hour: bool,
    show_seconds: bool,
    palette: Palette,
    color: bool,
) -> io::Result<()> {
    let (terminal_width, terminal_height) = crossterm::terminal::size().unwrap_or((100, 30));
    let width = (terminal_width as usize).clamp(60, 120);
    let height = (terminal_height as usize).clamp(20, 40);
    let mut frame = Frame::new(width, height);
    render(
        &mut frame,
        now,
        now.subsec,
        twelve_hour,
        show_seconds,
        palette,
    );

    let mut output = String::new();
    for y in 0..height {
        let mut last_visible = 0usize;
        for x in 0..width {
            let cell = frame.cells[y * width + x];
            if cell.ch != ' ' || cell.bg.is_some() {
                last_visible = x + 1;
            }
        }

        for cell in &frame.cells[y * width..y * width + last_visible] {
            if color {
                if let Some(bg) = cell.bg {
                    output.push_str(&bg.bg());
                } else {
                    output.push_str("\x1b[49m");
                }
                output.push_str(&cell.fg.fg());
            }
            output.push(cell.ch);
            if color {
                output.push_str("\x1b[0m");
            }
        }
        output.push('\n');
    }

    io::stdout().write_all(output.as_bytes())
}

fn choose_font(text: &str, width: i32, height: i32) -> (FontSize, i32, bool) {
    let candidates = [
        (FontSize::Large, 2),
        (FontSize::Large, 1),
        (FontSize::Small, 2),
        (FontSize::Small, 1),
    ];

    for (font, scale) in candidates {
        let clock_width = font.width(text) * scale;
        let clock_height = font.height() * scale;
        if clock_width <= width - 4 && clock_height <= height - 7 {
            return (font, scale, true);
        }
    }

    for (font, scale) in candidates {
        let clock_width = font.width(text) * scale;
        let clock_height = font.height() * scale;
        if clock_width <= width - 2 && clock_height <= height - 2 {
            return (font, scale, false);
        }
    }

    (FontSize::Small, 1, false)
}

#[derive(Clone, Copy)]
struct LitPixel {
    x: i32,
    y: i32,
    color: Rgb,
}

fn draw_clock(
    frame: &mut Frame,
    text: &str,
    origin: (i32, i32),
    font: FontSize,
    scale: i32,
    palette: Palette,
    elapsed: f32,
) {
    let (x, y) = origin;
    let total_width = font.width(text).max(1) as f32;
    let mut cursor_x = x;
    let mut pixels = Vec::new();

    for ch in text.chars() {
        let Some(glyph) = font.glyph(ch) else {
            continue;
        };
        let width = glyph_width(glyph);
        let colon = ch == ':';
        for (row, line) in glyph.iter().enumerate() {
            for (col, cell) in line.chars().enumerate() {
                if cell == ' ' {
                    continue;
                }

                let pixel_x = cursor_x + col as i32 * scale;
                let t = ((pixel_x - x) as f32 / total_width + row as f32 * 0.012 + elapsed * 0.04)
                    .rem_euclid(1.0);
                let mut color = palette.color(t, elapsed);
                if colon {
                    let pulse = 0.45 + 0.55 * (0.5 + 0.5 * (elapsed * 5.2).cos());
                    color = color.scale(pulse);
                }
                pixels.push(LitPixel {
                    x: pixel_x,
                    y: y + row as i32 * scale,
                    color,
                });
            }
        }
        cursor_x += width * scale + font.gap() * scale;
    }

    for pixel in &pixels {
        let shadow = pixel.color.scale(0.10);
        for dy in 0..scale {
            for dx in 0..scale {
                frame.put(
                    pixel.x + dx + scale,
                    pixel.y + dy + scale,
                    ' ',
                    DEFAULT_FG,
                    Some(shadow),
                );
            }
        }
    }

    for pixel in &pixels {
        let glow = pixel.color.scale(0.20);
        for dy in -1..=scale {
            for dx in -1..=scale {
                if dx >= 0 && dx < scale && dy >= 0 && dy < scale {
                    continue;
                }
                frame.put_bg_if_blank(pixel.x + dx, pixel.y + dy, glow);
            }
        }
    }

    for pixel in &pixels {
        for dy in 0..scale {
            for dx in 0..scale {
                frame.put(pixel.x + dx, pixel.y + dy, '█', pixel.color, None);
            }
        }
    }
}

fn draw_bar(
    frame: &mut Frame,
    x: i32,
    y: i32,
    width: i32,
    progress: f32,
    palette: Palette,
    elapsed: f32,
) {
    let filled = (progress.clamp(0.0, 1.0) * width as f32).round() as i32;
    for index in 0..width {
        let t = index as f32 / width.max(1) as f32;
        let (ch, color) = if index < filled {
            let mut color = palette.color(t, elapsed);
            if index + 1 == filled {
                color = color.lerp(Rgb::new(255, 255, 255), 0.55);
            }
            ('━', color)
        } else {
            ('─', Rgb::new(54, 58, 72))
        };
        frame.put(x + index, y, ch, color, None);
    }
}

fn center_text(frame: &mut Frame, y: i32, text: &str, color: Rgb) {
    let width = frame.width as i32;
    let text_width = text.chars().count() as i32;
    let x = (width - text_width) / 2;
    frame.put_text_clipped(x, y, text, color, width);
}

fn hsv(hue: f32, saturation: f32, value: f32) -> Rgb {
    let hue = hue.rem_euclid(1.0);
    let saturation = saturation.clamp(0.0, 1.0);
    let value = value.clamp(0.0, 1.0);
    let sector = (hue * 6.0).floor();
    let fraction = hue * 6.0 - sector;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - fraction * saturation);
    let t = value * (1.0 - (1.0 - fraction) * saturation);

    let (r, g, b) = match sector as i32 % 6 {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };

    Rgb::new(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_time() -> LocalTime {
        LocalTime {
            hour: 13,
            minute: 5,
            second: 9,
            subsec: 0.5,
            year: 2026,
            month: 11,
            day: 11,
            weekday: 3,
        }
    }

    #[test]
    fn diff_only_emits_changes() {
        let mut frame = Frame::new(4, 2);
        frame.put(1, 1, 'x', Rgb::new(255, 0, 0), None);
        let mut renderer = DiffRenderer::new();
        assert!(renderer.diff(&frame).contains('x'));
        assert!(renderer.diff(&frame).is_empty());
    }

    #[test]
    fn renders_clock_and_extras() {
        let mut frame = Frame::new(100, 30);
        render(&mut frame, &sample_time(), 1.25, false, true, Palette::Neon);
        assert!(frame.cells.iter().any(|cell| cell.ch == '█'));
        assert!(frame.cells.iter().any(|cell| cell.ch == '━'));
        assert!(frame.cells.iter().any(|cell| cell.ch == 'W'));
        assert!(!frame.cells.iter().any(|cell| cell.ch == 'S'));
    }

    #[test]
    fn palettes_stay_in_range() {
        for palette in Palette::ALL {
            for step in 0..20 {
                let color = palette.color(step as f32 / 20.0, 3.0);
                assert!((0..=255).contains(&color.r));
                assert!((0..=255).contains(&color.g));
                assert!((0..=255).contains(&color.b));
            }
        }
    }
}
