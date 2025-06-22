mod app;
mod ui;

use crate::app::AppState;
use crate::ui::render_ui;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

fn open_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = AppState::new();

    let search_app = Arc::clone(&app);
    thread::spawn(move || {
        loop {
            let mut app_guard = search_app.lock().unwrap();
            if !app_guard.loading && app_guard.should_update() {
                app_guard.update_matches();
                app_guard.mark_updated();
            }
            drop(app_guard);
            thread::sleep(Duration::from_millis(50));
        }
    });

    let result = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: Arc<Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| {
            let mut app_guard = app.lock().unwrap();
            render_ui(f, &mut app_guard);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut app_guard = app.lock().unwrap();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        return Ok(());
                    }
                    KeyCode::Char(c) => {
                        app_guard.input.push(c);
                        app_guard.mark_input_changed();
                    }
                    KeyCode::Backspace => {
                        app_guard.input.pop();
                        app_guard.mark_input_changed();
                    }
                    KeyCode::Up => app_guard.move_up(),
                    KeyCode::Down => app_guard.move_down(),
                    KeyCode::Enter => {
                        if let Some(path) = app_guard.get_selected_path() {
                            let path_to_open = path.to_string();
                            drop(app_guard);
                            let _ = open_file(&path_to_open);
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
