use crate::app::{App, Screen, Message};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Gauge},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::Main => draw_main(f, app),
        Screen::Download => draw_download(f, app),
        Screen::Import => draw_import(f, app),
        Screen::Status => draw_status(f, app),
        Screen::Settings => draw_settings(f, app),
    }
}

fn draw_main(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_server_list(f, chunks[1], app);
    draw_footer(f, chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = "🔒 WireGuard VPN Manager";
    let status = if let Some(ref server) = app.active_server {
        format!("✓ Connected: {}", server)
    } else {
        "✗ Not Connected".to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                status,
                Style::default().fg(if app.active_server.is_some() {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(paragraph, area);
}

fn draw_server_list(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .servers
        .iter()
        .enumerate()
        .map(|(idx, server)| {
            let is_selected = idx == app.selected_index;
            let is_active = Some(server) == app.active_server.as_ref();

            let mut content = String::new();
            if is_active {
                content.push_str("● ");
            } else {
                content.push_str("○ ");
            }
            content.push_str(server);

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(if is_active { Color::Green } else { Color::White })
            } else {
                Style::default().fg(if is_active { Color::Green } else { Color::White })
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title("Available Servers")
            .borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    let message_text = match &app.message {
        Message::None => "Ready".to_string(),
        Message::Info(msg) => msg.clone(),
        Message::Error(msg) => msg.clone(),
        Message::Success(msg) => msg.clone(),
    };

    let message_color = match &app.message {
        Message::None => Color::White,
        Message::Info(_) => Color::Blue,
        Message::Error(_) => Color::Red,
        Message::Success(_) => Color::Green,
    };

    let status_line = Paragraph::new(message_text)
        .style(Style::default().fg(message_color))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(status_line, chunks[0]);

    let help_text = if app.loading {
        "Loading...".to_string()
    } else {
        "↑↓: Navigate | Enter: Connect/Disconnect | o: Open Browser | i: Import | d: Delete | s: Status | q: Quit".to_string()
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, chunks[1]);
}

fn draw_download(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(5),
                Constraint::Min(10),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new("Downloading WireGuard Configurations...")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);

    f.render_widget(title, chunks[0]);

    if app.loading {
        let gauge = Gauge::default()
            .block(Block::default().title("Progress").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(50);

        f.render_widget(gauge, chunks[1]);
    } else {
        let message = match &app.message {
            Message::Success(msg) => {
                vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("✓ ", Style::default().fg(Color::Green)),
                        Span::raw(msg),
                    ]),
                    Line::from(""),
                    Line::from("Press any key to continue..."),
                ]
            }
            Message::Error(msg) => {
                vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("✗ ", Style::default().fg(Color::Red)),
                        Span::raw(msg),
                    ]),
                    Line::from(""),
                    Line::from("Press any key to continue..."),
                ]
            }
            _ => vec![Line::from("Initializing download...")],
        };

        let paragraph = Paragraph::new(message)
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(paragraph, chunks[1]);
    }
}

fn draw_status(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(15),
            ]
            .as_ref(),
        )
        .split(f.area());

    let header = Paragraph::new("VPN Status")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, chunks[0]);

    let status_lines = vec![
        Line::from(vec![
            Span::styled("Interface: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.status.interface),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                if app.status.is_connected { "Connected ✓" } else { "Disconnected ✗" },
                Style::default().fg(if app.status.is_connected {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Endpoint: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.status.endpoint),
        ]),
        Line::from(vec![
            Span::styled("Allowed IPs: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.status.allowed_ips),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Listening Port: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.status.listening_port),
        ]),
        Line::from(vec![
            Span::styled("Latest Handshake: ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.status.latest_handshake),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Received: ", Style::default().fg(Color::Green)),
            Span::raw(&app.status.transfer_received),
        ]),
        Line::from(vec![
            Span::styled("Sent: ", Style::default().fg(Color::Green)),
            Span::raw(&app.status.transfer_sent),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Public Key: ", Style::default().fg(Color::Magenta)),
            Span::raw(if app.status.public_key.len() > 20 {
                format!("{}...", &app.status.public_key[..20])
            } else {
                app.status.public_key.clone()
            }),
        ]),
    ];

    let paragraph = Paragraph::new(status_lines)
        .block(Block::default().title("Connection Details").borders(Borders::ALL));

    f.render_widget(paragraph, chunks[1]);
}

fn draw_settings(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new("Settings")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    f.render_widget(title, chunks[0]);

    let config_path = app.config_manager.get_config_path_str();
    let credentials_configured = !app.username.is_empty()
        && app.username != "a314393"
        && !app.password.is_empty()
        && app.password != "L7W8cXG3MH";

    let settings_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Configuration Status", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                if credentials_configured { "✓ Configured" } else { "⚠ Not Configured" },
                Style::default().fg(if credentials_configured { Color::Green } else { Color::Red }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Username: ", Style::default().fg(Color::Yellow)),
            Span::raw(if app.username.is_empty() {
                "(not configured)".to_string()
            } else if app.username == "a314393" {
                "⚠ Using template value - please update!".to_string()
            } else {
                format!("{} ✓", app.username)
            }),
        ]),
        Line::from(vec![
            Span::styled("Password: ", Style::default().fg(Color::Yellow)),
            Span::raw(if app.password.is_empty() {
                "(not configured)".to_string()
            } else if app.password == "L7W8cXG3MH" {
                "⚠ Using template value - please update!".to_string()
            } else {
                "***configured*** ✓".to_string()
            }),
        ]),
        Line::from(""),
        Line::from("─".repeat(60)),
        Line::from(""),
        Line::from(vec![
            Span::styled("Configuration File Location:", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&config_path, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from("How to configure:"),
        Line::from(vec![
            Span::raw("  1. Edit the file: "),
            Span::styled(format!("nano {}", config_path), Style::default().fg(Color::Yellow)),
        ]),
        Line::from("  2. Update the username and password with your StrongVPN credentials"),
        Line::from("  3. Save and restart the application"),
        Line::from(""),
        Line::from("Get your credentials from:"),
        Line::from("  • Login to https://strongtech.org/account/"),
        Line::from("  • Click 'Account Setup Instructions'"),
        Line::from("  • Look for 'VPN Account Information'"),
        Line::from("  • Username starts with 'a'"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Note: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("Use VPN credentials (AIO Username/Password), NOT website login!"),
        ]),
    ];

    let paragraph = Paragraph::new(settings_text)
        .block(Block::default().title("Configuration").borders(Borders::ALL));

    f.render_widget(paragraph, chunks[1]);
}

fn draw_import(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(f.area());

    let header = Paragraph::new("Import WireGuard Configurations")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, chunks[0]);

    if app.loading {
        let loading = Paragraph::new("Scanning Downloads directory...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(loading, chunks[1]);
    } else if app.import_configs.is_empty() {
        let empty = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("No WireGuard configurations found in ~/Downloads/", Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from("Steps to import configs:"),
            Line::from("  1. Press 'o' to open download page in browser"),
            Line::from("  2. Login with your StrongVPN credentials"),
            Line::from("  3. Download server configs (*.conf files)"),
            Line::from("  4. Press 'i' again to import them"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tip: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("Files must be in Downloads folder and named like str-*.conf"),
            ]),
        ];

        let paragraph = Paragraph::new(empty)
            .block(Block::default().title("No Configs Found").borders(Borders::ALL))
            .alignment(Alignment::Left);

        f.render_widget(paragraph, chunks[1]);
    } else {
        use crate::download::ConfigDownloader;

        let items: Vec<ListItem> = app
            .import_configs
            .iter()
            .enumerate()
            .map(|(idx, path)| {
                let is_selected = idx == app.import_selected;
                let info = ConfigDownloader::format_config_info(path);

                let content = if is_selected {
                    format!("▶ {}", info)
                } else {
                    format!("  {}", info)
                };

                let style = if is_selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title(format!("Found {} config(s) in ~/Downloads", app.import_configs.len()))
                .borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        f.render_widget(list, chunks[1]);
    }

    let help_text = if app.loading {
        "Loading...".to_string()
    } else if app.import_configs.is_empty() {
        "Press Esc or 'o' to go back and open download page".to_string()
    } else {
        "↑↓: Select | Enter: Import selected | a: Import all | Esc: Cancel".to_string()
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, chunks[2]);
}
