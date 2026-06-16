use crate::app::{App, Message, Screen};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.current_screen {
        Screen::Main => draw_main(f, app),
        Screen::Download => draw_download(f, app),
        Screen::Import => draw_import(f, app),
        Screen::Status => draw_status(f, app),
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
        Line::from(vec![Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            status,
            Style::default().fg(if app.active_server.is_some() {
                Color::Green
            } else {
                Color::Red
            }),
        )]),
    ];

    let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(paragraph, area);
}

fn draw_server_list(f: &mut Frame, area: Rect, app: &App) {
    if app.servers.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No WireGuard configs found",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from("Press o for download info, then i to import downloaded .conf files."),
        ])
        .block(
            Block::default()
                .title("Available Servers")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

        f.render_widget(empty, area);
        return;
    }

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

            let mut lines = vec![Line::from(content)];

            if is_selected {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "  Connection Details",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));

                if let Some(details) = &app.selected_details {
                    push_detail_line(&mut lines, "Address", details.interface_address.as_deref());
                    push_detail_line(&mut lines, "DNS", details.dns.as_deref());
                    push_detail_line(&mut lines, "Endpoint", details.endpoint.as_deref());
                    push_detail_line(&mut lines, "Allowed IPs", details.allowed_ips.as_deref());
                    push_detail_line(
                        &mut lines,
                        "Public Key",
                        details
                            .peer_public_key
                            .as_deref()
                            .map(shorten_key)
                            .as_deref(),
                    );
                    push_detail_line(
                        &mut lines,
                        "Private Key",
                        Some(if details.private_key_configured {
                            "Configured"
                        } else {
                            "Missing"
                        }),
                    );
                    push_detail_line(
                        &mut lines,
                        "Keepalive",
                        details.persistent_keepalive.as_deref(),
                    );
                } else if let Some(error) = &app.selected_details_error {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled("Config: ", Style::default().fg(Color::Yellow)),
                        Span::styled(error, Style::default().fg(Color::Red)),
                    ]));
                } else {
                    lines.push(Line::from("    No config details available"));
                }

                if is_active {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![Span::styled(
                        "  Live Status",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )]));
                    push_detail_line(
                        &mut lines,
                        "Handshake",
                        non_empty(&app.status.latest_handshake),
                    );
                    push_detail_line(
                        &mut lines,
                        "Received",
                        non_empty(&app.status.transfer_received),
                    );
                    push_detail_line(&mut lines, "Sent", non_empty(&app.status.transfer_sent));
                }
            }

            let style = if is_selected {
                Style::default().bg(Color::DarkGray).fg(if is_active {
                    Color::Green
                } else {
                    Color::White
                })
            } else {
                Style::default().fg(if is_active {
                    Color::Green
                } else {
                    Color::White
                })
            };

            ListItem::new(lines).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Available Servers")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn push_detail_line(lines: &mut Vec<Line>, label: &str, value: Option<&str>) {
    let value = value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(format!("{}: ", label), Style::default().fg(Color::Yellow)),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ]));
}

fn non_empty(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn shorten_key(key: &str) -> String {
    const VISIBLE_PREFIX: usize = 20;

    if key.len() > VISIBLE_PREFIX {
        format!("{}...", &key[..VISIBLE_PREFIX])
    } else {
        key.to_string()
    }
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
        "↑↓: Navigate | Enter: Connect/Disconnect | o: Download Info | i: Import | d: Delete | s: Status | q: Quit".to_string()
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
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    let title = Paragraph::new("Download WireGuard Configurations")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);

    f.render_widget(title, chunks[0]);

    // 显示下载信息
    let download_url = app.downloader.get_download_url();

    let info_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Step 1: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Open this URL in your browser:"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                download_url,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]),
        Line::from(""),
        Line::from("─".repeat(70)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Step 2: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Login with your credentials:"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Username: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &app.username,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Password: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                &app.password,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from("─".repeat(70)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Step 3: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Download server configs (*.conf files)"),
        ]),
        Line::from(""),
        Line::from("  • Select servers you want"),
        Line::from("  • Download to ~/Downloads/"),
        Line::from("  • Files format: str-*.conf (e.g., str-zrh302.conf)"),
        Line::from(""),
        Line::from("─".repeat(70)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Step 4: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Return to this TUI and press 'i' to import",
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(info_lines)
        .block(
            Block::default()
                .title("Download Instructions")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, chunks[1]);

    let help = Paragraph::new("Press Esc to return to main screen")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, chunks[2]);
}

fn draw_status(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(15)].as_ref())
        .split(f.area());

    let header = Paragraph::new("VPN Status")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
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
                if app.status.is_connected {
                    "Connected ✓"
                } else {
                    "Disconnected ✗"
                },
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

    let paragraph = Paragraph::new(status_lines).block(
        Block::default()
            .title("Connection Details")
            .borders(Borders::ALL),
    );

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
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, chunks[0]);

    if app.loading {
        let loading = Paragraph::new("Scanning Downloads directory...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(loading, chunks[1]);
    } else if app.import_configs.is_empty() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let scan_path = format!("{}/Downloads", home);

        let empty = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No .conf files found",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Scanned path: ", Style::default().fg(Color::Cyan)),
                Span::styled(&scan_path, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from("Steps to import configs:"),
            Line::from("  1. Press 'o' to view download info"),
            Line::from("  2. Copy URL and credentials to browser"),
            Line::from("  3. Download server configs to ~/Downloads/"),
            Line::from("  4. Press 'i' again to import them"),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Tip: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Any .conf files in Downloads will be detected"),
            ]),
        ];

        let paragraph = Paragraph::new(empty)
            .block(
                Block::default()
                    .title("No Configs Found")
                    .borders(Borders::ALL),
            )
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
                let is_checked = app.import_checked.get(idx).copied().unwrap_or(false);
                let info = ConfigDownloader::format_config_info(path);

                // 复选框符号
                let checkbox = if is_checked { "[✓]" } else { "[ ]" };

                // 选择指示器
                let indicator = if is_selected { "▶" } else { " " };

                let content = format!("{} {} {}", indicator, checkbox, info);

                let style = if is_selected {
                    Style::default().bg(Color::DarkGray).fg(if is_checked {
                        Color::Green
                    } else {
                        Color::White
                    })
                } else {
                    Style::default().fg(if is_checked {
                        Color::Green
                    } else {
                        Color::White
                    })
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let checked_count = app.import_checked.iter().filter(|&&x| x).count();
        let title = format!(
            "Found {} config(s) - {} selected",
            app.import_configs.len(),
            checked_count
        );

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        f.render_widget(list, chunks[1]);
    }

    let help_text = if app.loading {
        "Loading...".to_string()
    } else if app.import_configs.is_empty() {
        "Press Esc to go back | Press 'o' to view download info".to_string()
    } else {
        "↑↓: Navigate | Space: Check/Uncheck | a: Check All | n: Uncheck All | Enter: Import | Esc: Cancel".to_string()
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, chunks[2]);
}
