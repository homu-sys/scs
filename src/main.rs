mod font;
mod render;
mod time;

use crate::render::{DiffRenderer, Frame, Palette};
use crate::time::LocalTime;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
struct Options {
    static_mode: bool,
    twelve_hour: bool,
    show_seconds: bool,
    palette: Palette,
    fps: u64,
    no_color: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            static_mode: false,
            twelve_hour: false,
            show_seconds: true,
            palette: Palette::Neon,
            fps: 30,
            no_color: false,
        }
    }
}

#[derive(Default)]
struct CliOptions {
    static_mode: bool,
    twelve_hour: Option<bool>,
    show_seconds: Option<bool>,
    palette: Option<Palette>,
    fps: Option<u64>,
    no_color: Option<bool>,
    config: Option<PathBuf>,
}

impl CliOptions {
    fn apply(&self, options: &mut Options) {
        if self.static_mode {
            options.static_mode = true;
        }
        if let Some(value) = self.twelve_hour {
            options.twelve_hour = value;
        }
        if let Some(value) = self.show_seconds {
            options.show_seconds = value;
        }
        if let Some(value) = self.palette {
            options.palette = value;
        }
        if let Some(value) = self.fps {
            options.fps = value.clamp(10, 144);
        }
        if let Some(value) = self.no_color {
            options.no_color = value;
        }
    }
}

#[derive(Debug, Default)]
struct Config {
    twelve_hour: Option<bool>,
    show_seconds: Option<bool>,
    palette: Option<Palette>,
    fps: Option<u64>,
    no_color: Option<bool>,
}

impl Config {
    fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read config '{}': {error}", path.display()))?;
        Self::parse(&text, &path.display().to_string())
    }

    fn parse(text: &str, source: &str) -> Result<Self, String> {
        let mut config = Self::default();

        for (index, raw_line) in text.lines().enumerate() {
            let line = strip_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }

            let location = format!("{source}:{}", index + 1);
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("{location}: expected key = value"))?;
            let key = key.trim();
            let value = value.trim();

            match key {
                "theme" => {
                    config.palette = Some(
                        parse_theme(value)
                            .ok_or_else(|| format!("{location}: unknown theme '{value}'"))?,
                    );
                }
                "twelve_hour" => {
                    config.twelve_hour = Some(
                        parse_bool(value)
                            .ok_or_else(|| format!("{location}: expected true or false"))?,
                    );
                }
                "show_seconds" => {
                    config.show_seconds = Some(
                        parse_bool(value)
                            .ok_or_else(|| format!("{location}: expected true or false"))?,
                    );
                }
                "color" => {
                    let color = parse_bool(value)
                        .ok_or_else(|| format!("{location}: expected true or false"))?;
                    config.no_color = Some(!color);
                }
                "fps" => {
                    let fps = value
                        .parse::<u64>()
                        .map_err(|_| format!("{location}: invalid fps '{value}'"))?;
                    config.fps = Some(fps.clamp(10, 144));
                }
                _ => return Err(format!("{location}: unknown key '{key}'")),
            }
        }

        Ok(config)
    }

    fn apply(self, options: &mut Options) {
        if let Some(value) = self.twelve_hour {
            options.twelve_hour = value;
        }
        if let Some(value) = self.show_seconds {
            options.show_seconds = value;
        }
        if let Some(value) = self.palette {
            options.palette = value;
        }
        if let Some(value) = self.fps {
            options.fps = value.clamp(10, 144);
        }
        if let Some(value) = self.no_color {
            options.no_color = value;
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            Hide,
            Clear(ClearType::All)
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        let _ = stdout.flush();
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("scs: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let cli = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => return Ok(()),
        Err(message) => {
            eprintln!("scs: {message}");
            eprintln!();
            print_usage();
            std::process::exit(2);
        }
    };

    let mut options = Options::default();
    let config_path = cli.config.clone().or_else(default_config_path);

    if let Some(path) = config_path {
        if cli.config.is_some() || path.exists() {
            match Config::load(&path) {
                Ok(config) => config.apply(&mut options),
                Err(message) => {
                    eprintln!("scs: {message}");
                    std::process::exit(2);
                }
            }
        }
    }

    cli.apply(&mut options);

    let is_terminal = io::stdout().is_terminal();
    let color = is_terminal && !options.no_color && env::var_os("NO_COLOR").is_none();
    let now = LocalTime::now();

    if options.static_mode || !is_terminal {
        return render::print_static(
            &now,
            options.twelve_hour,
            options.show_seconds,
            options.palette,
            color,
        );
    }

    animate(options)
}

fn animate(options: Options) -> io::Result<()> {
    let fps = options.fps.clamp(10, 144);
    let frame_time = Duration::from_secs_f64(1.0 / fps as f64);
    let _guard = TerminalGuard::enter()?;

    let mut stdout = io::stdout();
    let mut renderer = DiffRenderer::new();
    let mut palette = options.palette;
    let mut twelve_hour = options.twelve_hour;
    let mut show_seconds = options.show_seconds;
    let mut last_size = (0u16, 0u16);
    let start = Instant::now();

    loop {
        let frame_start = Instant::now();
        let now = LocalTime::now();
        let elapsed = start.elapsed().as_secs_f32();
        let (width, height) = terminal::size()?;

        if (width, height) != last_size {
            last_size = (width, height);
            renderer.reset();
            execute!(stdout, Clear(ClearType::All))?;
        }

        if width > 0 && height > 0 {
            let mut frame = Frame::new(width as usize, height as usize);
            render::render(
                &mut frame,
                &now,
                elapsed,
                twelve_hour,
                show_seconds,
                palette,
            );
            let diff = renderer.diff(&frame);
            if !diff.is_empty() {
                stdout.write_all(diff.as_bytes())?;
                stdout.flush()?;
            }
        }

        let timeout = frame_time.saturating_sub(frame_start.elapsed());
        if event::poll(timeout)? {
            loop {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('c') if ctrl => return Ok(()),
                            KeyCode::Char('c') => palette = palette.next(),
                            KeyCode::Char('h') => twelve_hour = !twelve_hour,
                            KeyCode::Char('s') => show_seconds = !show_seconds,
                            KeyCode::Char('r') => {
                                palette = options.palette;
                                twelve_hour = options.twelve_hour;
                                show_seconds = options.show_seconds;
                            }
                            _ => {}
                        }
                    }
                    Event::Resize(_, _) => {
                        renderer.reset();
                        execute!(stdout, Clear(ClearType::All))?;
                    }
                    _ => {}
                }

                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }
}

fn parse_args() -> Result<Option<CliOptions>, String> {
    let mut options = CliOptions::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("scs {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--static" => options.static_mode = true,
            "--12h" => options.twelve_hour = Some(true),
            "--24h" => options.twelve_hour = Some(false),
            "--seconds" => options.show_seconds = Some(true),
            "--no-seconds" => options.show_seconds = Some(false),
            "--color" => options.no_color = Some(false),
            "--no-color" => options.no_color = Some(true),
            "--config" => {
                let value = next_value(&mut args, "--config")?;
                options.config = Some(PathBuf::from(value));
            }
            "--theme" => {
                let value = next_value(&mut args, "--theme")?;
                options.palette = Some(theme_value(&value)?);
            }
            "--fps" => {
                let value = next_value(&mut args, "--fps")?;
                let fps = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid fps '{value}'"))?;
                options.fps = Some(fps.clamp(10, 144));
            }
            _ => return Err(format!("unknown option '{arg}'")),
        }
    }

    Ok(Some(options))
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn theme_value(value: &str) -> Result<Palette, String> {
    Palette::from_name(value).ok_or_else(|| {
        format!(
            "unknown theme '{value}'. Choose neon, sunset, matrix, ice, candy, fire, ocean, mono, or rainbow"
        )
    })
}

fn parse_theme(value: &str) -> Option<Palette> {
    Palette::from_name(value.trim_matches('"').trim_matches('\''))
}

fn parse_bool(value: &str) -> Option<bool> {
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

fn default_config_path() -> Option<PathBuf> {
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        if !config_home.is_empty() {
            return Some(PathBuf::from(config_home).join("scs").join("config.toml"));
        }
    }

    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("scs")
            .join("config.toml")
    })
}

fn print_help() {
    println!("scs {} - a very cool TUI clock", env!("CARGO_PKG_VERSION"));
    println!();
    print_usage();
}

fn print_usage() {
    println!("USAGE:");
    println!("  scs [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help              Show this help");
    println!("  -V, --version           Show the version");
    println!("      --static            Print one frame and exit");
    println!("      --config <PATH>     Config file (default: ~/.config/scs/config.toml)");
    println!("      --12h               Use 12-hour time");
    println!("      --24h               Use 24-hour time");
    println!("      --seconds           Show seconds");
    println!("      --no-seconds        Hide seconds");
    println!("      --color             Enable ANSI colors");
    println!("      --no-color          Disable ANSI colors");
    println!(
        "      --theme <NAME>      neon, sunset, matrix, ice, candy, fire, ocean, mono, rainbow"
    );
    println!("      --fps <FPS>         Animation rate, clamped to 10-144");
    println!();
    println!("KEYS:");
    println!("  q / Esc                 Quit");
    println!("  c                       Cycle color theme");
    println!("  h                       Toggle 12/24-hour time");
    println!("  s                       Toggle seconds");
    println!("  r                       Reset options");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_values() {
        let config = Config::parse(
            "theme = \"matrix\"\ntwelve_hour = true\nshow_seconds = false\nfps = 60\ncolor = false\n",
            "test",
        )
        .unwrap();

        assert_eq!(config.palette, Some(Palette::Matrix));
        assert_eq!(config.twelve_hour, Some(true));
        assert_eq!(config.show_seconds, Some(false));
        assert_eq!(config.fps, Some(60));
        assert_eq!(config.no_color, Some(true));
    }

    #[test]
    fn cli_overrides_config() {
        let mut options = Options::default();
        let config = Config::parse("theme = \"matrix\"\nfps = 60\n", "test").unwrap();
        config.apply(&mut options);

        let cli = CliOptions {
            palette: Some(Palette::Sunset),
            fps: Some(30),
            ..CliOptions::default()
        };
        cli.apply(&mut options);

        assert_eq!(options.palette, Palette::Sunset);
        assert_eq!(options.fps, 30);
    }

    #[test]
    fn strips_inline_comments() {
        let config = Config::parse("theme = \"ice\" # cool\n", "test").unwrap();
        assert_eq!(config.palette, Some(Palette::Ice));
    }

    #[test]
    fn rejects_unknown_config_key() {
        let error = Config::parse("nope = true\n", "test").unwrap_err();
        assert!(error.contains("unknown key"));
    }
}
