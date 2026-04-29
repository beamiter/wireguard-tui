mod app;
mod ui;
mod vpn;
mod download;
mod config;
mod commands;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let app = app::App::new().await?;
    run(app).await
}

async fn run(mut app: app::App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        terminal.draw(|f| {
            ui::draw(f, &app);
        })?;

        if let Ok(true) = event::poll(std::time::Duration::from_millis(100)) {
            if let Event::Key(key) = event::read()? {
                // 全局快捷键
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Char('q'), KeyModifiers::NONE) => break,
                    _ => {}
                }

                // 根据当前屏幕处理不同的键
                match app.current_screen {
                    app::Screen::Import => {
                        match key.code {
                            KeyCode::Esc => {
                                app.current_screen = app::Screen::Main;
                            }
                            KeyCode::Up => {
                                if app.import_selected > 0 {
                                    app.import_selected -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if app.import_selected < app.import_configs.len().saturating_sub(1) {
                                    app.import_selected += 1;
                                }
                            }
                            KeyCode::Enter => {
                                app.handle_import_selected().await?;
                            }
                            KeyCode::Char('a') => {
                                app.handle_import_all().await?;
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        match key.code {
                            KeyCode::Esc => {
                                app.current_screen = app::Screen::Main;
                            }
                            KeyCode::Up => app.handle_up(),
                            KeyCode::Down => app.handle_down(),
                            KeyCode::Enter => app.handle_enter().await?,
                            KeyCode::Char('o') => app.handle_open_browser().await?,
                            KeyCode::Char('i') => app.handle_import().await?,
                            KeyCode::Char('d') => app.handle_delete().await?,
                            KeyCode::Char('s') => app.handle_status().await?,
                            _ => {}
                        }
                    }
                }
            }
        }

        app.tick().await?;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
