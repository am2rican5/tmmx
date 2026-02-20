mod app;
mod event;
mod model;
mod template;
mod tmux;
mod ui;

use std::io;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::KeyEventKind;
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::App;
use event::{AppEvent, EventReader};

fn main() -> Result<()> {
    color_eyre::install()?;

    // Install panic hook that restores terminal (once)
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    loop {
        // Check tmux is available (each iteration, in case server died while attached)
        if !tmux::is_tmux_running() {
            eprintln!("Error: tmux server is not running.");
            eprintln!("Start tmux first, then run tmmx.");
            std::process::exit(1);
        }

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        match result {
            Ok(Some(target)) => {
                // Attach to tmux session (blocks until user detaches)
                let status = std::process::Command::new("tmux")
                    .args(["attach-session", "-t", &target])
                    .status();
                if let Err(e) = status {
                    eprintln!("Failed to attach to tmux: {}", e);
                    return Err(e.into());
                }
                // After detach, loop restarts tmmx
            }
            Ok(None) => return Ok(()), // Normal quit
            Err(e) => return Err(e),
        }
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<String>> {
    let mut app = App::new();
    let mut events = EventReader::new(Duration::from_millis(250));

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        match events.next()? {
            AppEvent::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
            AppEvent::Tick => {
                app.tick();
            }
            AppEvent::Resize => {}
        }

        if app.should_suspend {
            return Ok(app.suspend_target.take());
        }

        if !app.running {
            return Ok(None);
        }
    }
}
