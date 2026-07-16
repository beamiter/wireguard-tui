mod app;
mod commands;
mod config;
mod download;
mod ui;
mod vpn;

use anyhow::{bail, Result};
use app::{App, Screen};
use commands::CommandExecutor;
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, IsTerminal, Stdout};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use std::time::Duration;

type Tui = Terminal<CrosstermBackend<Stdout>>;

#[tokio::main]
async fn main() -> Result<()> {
    install_panic_hook();

    if CommandExecutor::running_as_root() {
        bail!(
            "refusing to run the TUI as root; run it as your normal user and authorize only WireGuard operations with sudo"
        );
    }

    // Authenticate before raw mode so sudo can safely display its prompt. All commands issued
    // from inside the TUI use non-interactive sudo and therefore cannot corrupt the terminal.
    let privilege_warning = authorize_privileges();
    let mut app = App::new().await?;
    if let Some(warning) = privilege_warning {
        app.set_startup_warning(warning);
    }

    run(app).await
}

fn authorize_privileges() -> Option<String> {
    if !io::stdin().is_terminal()
        || std::env::var_os("WIREGUARD_TUI_NO_SUDO_PROMPT").is_some()
        || !CommandExecutor::check_wireguard_installed().unwrap_or(false)
    {
        return None;
    }

    eprintln!("wireguard-tui: authorizing WireGuard operations (sudo may prompt)...");
    CommandExecutor::authorize_privileges()
        .err()
        .map(|error| format!("Privilege authorization failed: {error}"))
}

async fn run(mut app: App) -> Result<()> {
    let signals = TerminationSignals::new()?;
    let mut session = TerminalSession::enter()?;

    let loop_result = async {
        loop {
            if signals.received() {
                break;
            }
            session.terminal.draw(|frame| ui::draw(frame, &app))?;

            if event::poll(Duration::from_millis(80))? {
                match event::read()? {
                    Event::Key(key) if handle_key(&mut app, key) => break,
                    Event::Key(_)
                    | Event::Mouse(_)
                    | Event::Paste(_)
                    | Event::FocusGained
                    | Event::FocusLost
                    | Event::Resize(_, _) => {}
                }
            }

            app.tick().await;
        }

        Ok(())
    }
    .await;

    // Restore the user's terminal before waiting for a bounded in-flight
    // transaction to complete. This also covers draw and input errors.
    drop(session);
    app.shutdown().await;
    loop_result
}

struct TerminationSignals {
    received: Arc<AtomicU8>,
    watchers: [tokio::task::JoinHandle<()>; 3],
}

impl TerminationSignals {
    fn new() -> Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};
        let received = Arc::new(AtomicU8::new(0));
        let watch = |mut signal: tokio::signal::unix::Signal| {
            let received = Arc::clone(&received);
            tokio::spawn(async move {
                while signal.recv().await.is_some() {
                    if received.fetch_add(1, Ordering::AcqRel) > 0 {
                        std::process::exit(130);
                    }
                }
            })
        };
        let watchers = [
            watch(signal(SignalKind::interrupt())?),
            watch(signal(SignalKind::terminate())?),
            watch(signal(SignalKind::hangup())?),
        ];
        Ok(Self { received, watchers })
    }

    fn received(&self) -> bool {
        self.received.load(Ordering::Acquire) > 0
    }
}

impl Drop for TerminationSignals {
    fn drop(&mut self) {
        for watcher in &self.watchers {
            watcher.abort();
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if key.kind == KeyEventKind::Release {
        return false;
    }

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return app.request_quit();
    }

    let layout_visible =
        crossterm::terminal::size().is_ok_and(|(width, height)| ui::supports_layout(width, height));
    if !layout_visible {
        if key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('q')
            && key.modifiers == KeyModifiers::NONE
        {
            return app.request_quit();
        }
        if app.pending_delete.is_some()
            && key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Esc | KeyCode::Char('n' | 'N'))
        {
            app.cancel_delete();
        }
        return false;
    }

    if key.modifiers.intersects(
        KeyModifiers::CONTROL
            | KeyModifiers::ALT
            | KeyModifiers::SUPER
            | KeyModifiers::HYPER
            | KeyModifiers::META,
    ) {
        return false;
    }

    if app.search_active {
        match key.code {
            KeyCode::Esc => app.clear_search(),
            KeyCode::Enter => app.finish_search(),
            KeyCode::Backspace => app.pop_search(),
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                app.push_search(character)
            }
            _ => {}
        }
        return false;
    }

    if app.pending_delete.is_some() {
        if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Enter | KeyCode::Char('y' | 'Y') => {
                    app.confirm_delete();
                }
                KeyCode::Esc | KeyCode::Char('n' | 'N') => app.cancel_delete(),
                _ => {}
            }
        }
        return false;
    }

    if key.kind == KeyEventKind::Press
        && key.code == KeyCode::Char('q')
        && key.modifiers == KeyModifiers::NONE
    {
        return app.request_quit();
    }

    match app.current_screen {
        Screen::Main => handle_main_key(app, key),
        Screen::Download => handle_download_key(app, key),
        Screen::Import => handle_import_key(app, key),
        Screen::Status => handle_status_key(app, key),
        Screen::Help => {
            if key.kind == KeyEventKind::Press
                && matches!(key.code, KeyCode::Esc | KeyCode::Char('?'))
            {
                app.current_screen = Screen::Main;
            }
        }
    }

    false
}

fn handle_main_key(app: &mut App, key: KeyEvent) {
    let navigation = matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat);
    if navigation {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.handle_up(),
            KeyCode::Down | KeyCode::Char('j') => app.handle_down(),
            KeyCode::Home => app.handle_home(),
            KeyCode::End => app.handle_end(),
            KeyCode::PageUp => app.handle_page_up(),
            KeyCode::PageDown => app.handle_page_down(),
            _ => {}
        }
    }

    if key.kind != KeyEventKind::Press {
        return;
    }
    match key.code {
        KeyCode::Enter => app.handle_enter(),
        KeyCode::Esc if !app.search_query.is_empty() => app.clear_search(),
        KeyCode::Esc => app.dismiss_message(),
        KeyCode::Char('o') => app.show_download(),
        KeyCode::Char('i') => app.handle_import(),
        KeyCode::Char('d') => app.request_delete(),
        KeyCode::Char('s') => app.show_status(),
        KeyCode::Char('r') => app.refresh_all(),
        KeyCode::Char('/') => app.begin_search(),
        KeyCode::Char('?') => app.current_screen = Screen::Help,
        _ => {}
    }
}

fn handle_download_key(app: &mut App, key: KeyEvent) {
    let navigation = matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat);
    if navigation {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.scroll_download_up(),
            KeyCode::Down | KeyCode::Char('j') => app.scroll_download_down(),
            KeyCode::Home => app.scroll_download_home(),
            KeyCode::End => app.scroll_download_end(),
            _ => {}
        }
    }
    if key.kind == KeyEventKind::Press {
        match key.code {
            KeyCode::Esc => app.current_screen = Screen::Main,
            KeyCode::Char('b') => app.open_download_page(),
            KeyCode::Char('i') => app.handle_import(),
            _ => {}
        }
    }
}

fn handle_import_key(app: &mut App, key: KeyEvent) {
    let navigation = matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat);
    if navigation {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.handle_import_up(),
            KeyCode::Down | KeyCode::Char('j') => app.handle_import_down(),
            _ => {}
        }
    }
    if key.kind == KeyEventKind::Press {
        match key.code {
            KeyCode::Esc => app.current_screen = Screen::Main,
            KeyCode::Char(' ') => app.handle_toggle_check(),
            KeyCode::Enter => app.handle_import_selected(),
            KeyCode::Char('a') => app.set_all_imports(true),
            KeyCode::Char('n') => app.set_all_imports(false),
            KeyCode::Char('r') => app.handle_import(),
            KeyCode::Char('o') => app.show_download(),
            _ => {}
        }
    }
}

fn handle_status_key(app: &mut App, key: KeyEvent) {
    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.scroll_status_up(),
            KeyCode::Down | KeyCode::Char('j') => app.scroll_status_down(),
            KeyCode::Home => app.scroll_status_home(),
            KeyCode::End => app.scroll_status_end(),
            _ => {}
        }
    }
    if key.kind == KeyEventKind::Press {
        match key.code {
            KeyCode::Esc => app.current_screen = Screen::Main,
            KeyCode::Char('r') => app.refresh_status_now(),
            _ => {}
        }
    }
}

struct TerminalSession {
    terminal: Tui,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen) {
            restore_terminal();
            return Err(error.into());
        }

        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => {
                let mut session = Self { terminal };
                if let Err(error) = session.terminal.clear() {
                    drop(session);
                    return Err(error.into());
                }
                Ok(session)
            }
            Err(error) => {
                restore_terminal();
                Err(error.into())
            }
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn install_panic_hook() {
    let ui_thread = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Background task panics are reported through JoinError. Only the
        // thread that owns the terminal may tear down raw/alternate-screen mode.
        if std::thread::current().id() == ui_thread {
            restore_terminal();
        }
        previous(panic_info);
    }));
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
}
