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
use std::io::{self, IsTerminal, Write};
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
    let options = match parse_args() {
        Ok(Some(options)) => options,
        Ok(None) => return Ok(()),
        Err(message) => {
            eprintln!("scs: {message}");
            eprintln!();
            print_usage();
            std::process::exit(2);
        }
    };

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

fn parse_args() -> Result<Option<Options>, String> {
    let mut options = Options::default();
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
            "--12h" => options.twelve_hour = true,
            "--no-seconds" => options.show_seconds = false,
            "--no-color" => options.no_color = true,
            "--theme" => {
                let value = next_value(&mut args, "--theme")?;
                options.palette = Palette::ALL
                    .into_iter()
                    .find(|palette| palette.name().eq_ignore_ascii_case(&value))
                    .ok_or_else(|| {
                        format!(
                            "unknown theme '{value}'. Choose neon, sunset, matrix, ice, candy, fire, ocean, mono, or rainbow"
                        )
                    })?;
            }
            "--fps" => {
                let value = next_value(&mut args, "--fps")?;
                let fps = value
                    .parse::<u64>()
                    .map_err(|_| format!("invalid fps '{value}'"))?;
                options.fps = fps.clamp(10, 144);
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
    println!("      --12h               Use 12-hour time");
    println!("      --no-seconds        Hide seconds");
    println!(
        "      --theme <NAME>      neon, sunset, matrix, ice, candy, fire, ocean, mono, rainbow"
    );
    println!("      --fps <FPS>         Animation rate, clamped to 10-144");
    println!("      --no-color          Disable ANSI colors");
    println!();
    println!("KEYS:");
    println!("  q / Esc                 Quit");
    println!("  c                       Cycle color theme");
    println!("  h                       Toggle 12/24-hour time");
    println!("  s                       Toggle seconds");
    println!("  r                       Reset options");
}
