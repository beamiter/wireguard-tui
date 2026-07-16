use crate::app::{App, ConnectionState, Message, Screen};
use crate::config::ConfigDetails;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;
const MIN_TERMINAL_WIDTH: u16 = 80;
const MIN_TERMINAL_HEIGHT: u16 = 24;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if terminal_too_small(area) {
        draw_too_small(frame, area);
        return;
    }

    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(frame, shell[0], app);
    match app.current_screen {
        Screen::Main => draw_main(frame, shell[1], app),
        Screen::Download => draw_download(frame, shell[1], app),
        Screen::Import => draw_import(frame, shell[1], app),
        Screen::Status => draw_status(frame, shell[1], app),
        Screen::Help => draw_help(frame, shell[1]),
    }
    draw_notice(frame, shell[2], app);
    draw_shortcuts(frame, shell[3], app);

    if let Some(server) = app.pending_delete.as_deref() {
        draw_delete_confirmation(frame, area, server);
    }
}

fn terminal_too_small(area: Rect) -> bool {
    !supports_layout(area.width, area.height)
}

pub fn supports_layout(width: u16, height: u16) -> bool {
    width >= MIN_TERMINAL_WIDTH && height >= MIN_TERMINAL_HEIGHT
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let message = Paragraph::new(vec![
        Line::from(Span::styled(
            "WireGuard TUI",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Terminal is too small."),
        Line::from(format!(
            "Resize to at least {MIN_TERMINAL_WIDTH} x {MIN_TERMINAL_HEIGHT}."
        )),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(message, area);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let (connection, color) = match &app.connection {
        ConnectionState::Unknown => ("State unknown".to_string(), Color::Yellow),
        ConnectionState::Disconnected => ("Disconnected".to_string(), Color::Red),
        ConnectionState::Connecting(server) => (
            format!("Connecting: {}", sanitize_for_terminal(server)),
            Color::Yellow,
        ),
        ConnectionState::Connected(server) => (
            format!("Connected: {}", sanitize_for_terminal(server)),
            Color::Green,
        ),
        ConnectionState::Disconnecting(server) => (
            format!("Disconnecting: {}", sanitize_for_terminal(server)),
            Color::Yellow,
        ),
        ConnectionState::Degraded(server) => (
            format!("Degraded: {}", sanitize_for_terminal(server)),
            Color::Yellow,
        ),
        ConnectionState::Ambiguous(servers) => (
            format!(
                "Multiple/unmanaged: {}",
                sanitize_for_terminal(&servers.join(", "))
            ),
            Color::Yellow,
        ),
    };

    let mut spans = vec![
        Span::styled(
            " WireGuard TUI ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(connection, Style::default().fg(color)),
    ];
    if let Some(label) = app.operation_label() {
        const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];
        spans.extend([
            Span::raw("  "),
            Span::styled(
                format!(
                    "{} {}",
                    SPINNER[app.spinner_frame % SPINNER.len()],
                    sanitize_for_terminal(label)
                ),
                Style::default().fg(Color::Yellow),
            ),
        ]);
    }

    let header = Paragraph::new(Line::from(spans))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);
    frame.render_widget(header, area);
}

fn draw_main(frame: &mut Frame, area: Rect, app: &App) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    draw_server_list(frame, panels[0], app);
    draw_details(frame, panels[1], app);
}

fn draw_server_list(frame: &mut Frame, area: Rect, app: &App) {
    let visible = app.visible_server_indices();
    let title = if app.search_query.is_empty() {
        format!(" Servers ({}) ", app.servers.len())
    } else {
        format!(
            " Servers ({}/{}) | /{}{} ",
            visible.len(),
            app.servers.len(),
            sanitize_for_terminal(&app.search_query),
            if app.search_active { "_" } else { "" }
        )
    };

    if visible.is_empty() {
        let text = if app.search_query.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No WireGuard configs found",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from("Press i to scan your Downloads folder."),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from("No servers match this search."),
                Line::from("Press Esc to clear the filter."),
            ]
        };
        frame.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(Block::default().title(title).borders(Borders::ALL)),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = visible
        .iter()
        .filter_map(|index| app.servers.get(*index))
        .map(|server| {
            let is_active = app.connection.contains(server) && app.connection.is_up();
            let marker = if is_active { "●" } else { "○" };
            let style = if is_active {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {marker} "), style),
                Span::styled(sanitize_for_terminal(server), style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default().with_selected(Some(app.selected_index));
    frame.render_stateful_widget(list, area, &mut state);

    if visible.len() > area.height.saturating_sub(2) as usize {
        let mut scrollbar_state = ScrollbarState::new(visible.len()).position(app.selected_index);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn draw_details(frame: &mut Frame, area: Rect, app: &App) {
    let Some(server) = app.selected_server() else {
        frame.render_widget(
            Paragraph::new("Select a server to inspect its configuration.")
                .alignment(Alignment::Center)
                .block(Block::default().title(" Details ").borders(Borders::ALL)),
            area,
        );
        return;
    };

    let live = (app.connection.server() == Some(server) && app.connection.is_up()).then_some(
        LiveDetails {
            stale: app.status_stale,
            handshake: non_empty(&app.status.latest_handshake),
            received: non_empty(&app.status.transfer_received),
            sent: non_empty(&app.status.transfer_sent),
            public_ip: app.current_ip.as_deref(),
        },
    );
    let lines = build_details_lines(
        server,
        app.selected_details.as_ref(),
        app.selected_details_error.as_deref(),
        live,
    );

    render_details(frame, area, lines);
}

#[derive(Clone, Copy)]
struct LiveDetails<'a> {
    stale: bool,
    handshake: Option<&'a str>,
    received: Option<&'a str>,
    sent: Option<&'a str>,
    public_ip: Option<&'a str>,
}

fn build_details_lines(
    server: &str,
    details: Option<&ConfigDetails>,
    details_error: Option<&str>,
    live: Option<LiveDetails<'_>>,
) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(Span::styled(
        sanitize_for_terminal(server),
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    ))];

    if let Some(details) = details {
        push_detail(&mut lines, "Address", details.interface_address.as_deref());
        push_detail(&mut lines, "DNS", details.dns.as_deref());
        push_detail(&mut lines, "Endpoint", details.endpoint.as_deref());
        push_detail(&mut lines, "Allowed IPs", details.allowed_ips.as_deref());
        let public_key = details.peer_public_key.as_deref().map(shorten_key);
        push_detail(&mut lines, "Peer key", public_key.as_deref());
        push_detail(
            &mut lines,
            "Private key",
            Some(if details.private_key_configured {
                "Configured (hidden)"
            } else {
                "Missing"
            }),
        );
        push_detail(
            &mut lines,
            "Keepalive",
            details.persistent_keepalive.as_deref(),
        );
    } else if let Some(error) = details_error {
        lines.push(Line::from(Span::styled(
            format!("Config unavailable: {}", sanitize_for_terminal(error)),
            Style::default().fg(Color::Red),
        )));
    }

    if let Some(live) = live {
        lines.push(Line::from(Span::styled(
            if live.stale {
                "Live status (stale)"
            } else {
                "Live status"
            },
            Style::default()
                .fg(if live.stale {
                    Color::Yellow
                } else {
                    Color::Green
                })
                .add_modifier(Modifier::BOLD),
        )));
        push_detail(&mut lines, "Handshake", live.handshake);
        push_detail(&mut lines, "Received", live.received);
        push_detail(&mut lines, "Sent", live.sent);
        push_detail(&mut lines, "Public IP", live.public_ip);
    }

    lines
}

fn render_details(frame: &mut Frame, area: Rect, lines: Vec<Line<'static>>) {
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().title(" Details ").borders(Borders::ALL)),
        area,
    );
}

fn draw_download(frame: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(Span::styled(
            "Download WireGuard configurations",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("1. Open: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                sanitize_for_terminal(app.downloader.get_download_url()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]),
        Line::from("   Press b to launch the system browser."),
        Line::from(""),
        Line::from(vec![
            Span::styled("2. Sign in: ", Style::default().fg(Color::Yellow)),
            Span::raw("enter provider credentials only in the browser page."),
        ]),
        Line::from("   wireguard-tui never stores or sends provider credentials."),
        Line::from(""),
        Line::from(vec![
            Span::styled("3. Download: ", Style::default().fg(Color::Yellow)),
            Span::raw("choose the servers you need and save their .conf files to:"),
        ]),
        Line::from(format!(
            "   {}",
            sanitize_for_terminal(&app.downloader.downloads_dir().display().to_string())
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("4. Import: ", Style::default().fg(Color::Yellow)),
            Span::raw("return here and press i."),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Security: imported files are validated; shell hooks and unsafe names are rejected.",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(" Download & Import ")
            .borders(Borders::ALL),
    );
    let max_scroll = paragraph
        .line_count(area.width)
        .saturating_sub(area.height as usize)
        .min(u16::MAX as usize) as u16;
    let paragraph = paragraph.scroll((app.download_scroll.min(max_scroll), 0));
    frame.render_widget(paragraph, area);
}

fn draw_import(frame: &mut Frame, area: Rect, app: &App) {
    if app.import_configs.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No safe WireGuard configs found",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "Scanned: {}",
                sanitize_for_terminal(&app.downloader.downloads_dir().display().to_string())
            )),
            Line::from("Press o for download instructions or r to scan again."),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(Block::default().title(" Import ").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .import_configs
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let checked = app.import_checked.get(index).copied().unwrap_or(false);
            let checkbox = if checked { "[x]" } else { "[ ]" };
            let style = Style::default().fg(if checked { Color::Green } else { Color::White });
            let display = app.import_display.get(index).cloned().unwrap_or_else(|| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".to_string())
            });
            ListItem::new(format!(" {checkbox} {}", sanitize_for_terminal(&display))).style(style)
        })
        .collect();
    let selected_count = app
        .import_checked
        .iter()
        .filter(|checked| **checked)
        .count();
    let title = format!(
        " Import | {} found, {selected_count} selected ",
        app.import_configs.len()
    );
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    let mut state = ListState::default().with_selected(Some(app.import_selected));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();
    push_detail(
        &mut lines,
        "Interface",
        non_empty(&app.status.interface).or_else(|| app.connection.server()),
    );
    push_detail(
        &mut lines,
        "State",
        Some(match &app.connection {
            ConnectionState::Unknown => "Unknown / unavailable",
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting(_) => "Connecting",
            ConnectionState::Connected(_) => "Connected",
            ConnectionState::Disconnecting(_) => "Disconnecting",
            ConnectionState::Degraded(_) => "Degraded / stale",
            ConnectionState::Ambiguous(_) => "Multiple or unmanaged interfaces",
        }),
    );
    push_detail(&mut lines, "Public IP", app.current_ip.as_deref());
    lines.push(Line::from(""));
    push_detail(&mut lines, "Endpoint", non_empty(&app.status.endpoint));
    push_detail(
        &mut lines,
        "Allowed IPs",
        non_empty(&app.status.allowed_ips),
    );
    push_detail(
        &mut lines,
        "Listening port",
        non_empty(&app.status.listening_port),
    );
    push_detail(
        &mut lines,
        "Latest handshake",
        non_empty(&app.status.latest_handshake),
    );
    lines.push(Line::from(""));
    push_detail(
        &mut lines,
        "Received",
        non_empty(&app.status.transfer_received),
    );
    push_detail(&mut lines, "Sent", non_empty(&app.status.transfer_sent));
    let public_key = non_empty(&app.status.public_key).map(shorten_key);
    push_detail(&mut lines, "Public key", public_key.as_deref());

    if let Some(error) = &app.status_error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Diagnostic: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                sanitize_for_terminal(error),
                Style::default().fg(Color::Red),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(if app.status_stale {
                " Status (stale) "
            } else {
                " Status "
            })
            .borders(Borders::ALL),
    );
    let max_scroll = paragraph
        .line_count(area.width)
        .saturating_sub(area.height as usize)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(
        paragraph.scroll((app.status_scroll.min(max_scroll), 0)),
        area,
    );
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let help = vec![
        Line::from(Span::styled(
            "Keyboard reference",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_line("↑/↓, j/k", "Move selection or scroll"),
        help_line("Home/End, PgUp/PgDn", "Jump through the server list"),
        help_line("Enter", "Connect or disconnect the selected server"),
        help_line("/", "Filter servers by name"),
        help_line("r", "Refresh configuration and status"),
        help_line("o / i", "Download instructions / import configs"),
        help_line("s", "Connection details and public IP"),
        help_line("d", "Delete after explicit confirmation"),
        help_line("Esc", "Go back, clear search, or dismiss an error"),
        help_line("q / Ctrl+C", "Quit"),
    ];
    frame.render_widget(
        Paragraph::new(help)
            .wrap(Wrap { trim: false })
            .block(Block::default().title(" Help ").borders(Borders::ALL)),
        area,
    );
}

fn help_line<'a>(key: &'a str, description: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{key:<22}"), Style::default().fg(Color::Yellow)),
        Span::raw(description),
    ])
}

fn draw_notice(frame: &mut Frame, area: Rect, app: &App) {
    let (color, prefix) = match app.message {
        Message::None => (MUTED, ""),
        Message::Info(_) => (Color::Blue, "Info: "),
        Message::Error(_) => (Color::Red, "Error: "),
        Message::Success(_) => (Color::Green, "Success: "),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                prefix,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                sanitize_for_terminal(app.message.text()),
                Style::default().fg(color),
            ),
        ]))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

fn draw_shortcuts(frame: &mut Frame, area: Rect, app: &App) {
    let text = if app.pending_delete.is_some() {
        "y/Enter confirm delete | n/Esc cancel"
    } else if app.is_busy() {
        "Background operation active | navigation and Esc remain available"
    } else {
        match app.current_screen {
            Screen::Main => {
                "↑↓/jk navigate | Enter connect | / search | r refresh | o download | i import | d delete | s status | ? help | q quit"
            }
            Screen::Download => "↑↓ scroll | b open browser | i import | Esc back | q quit",
            Screen::Import if app.import_configs.is_empty() => {
                "r rescan | o download info | Esc back | q quit"
            }
            Screen::Import => {
                "↑↓/jk navigate | Space toggle | a/n all/none | Enter import | r rescan | o download | Esc back | q quit"
            }
            Screen::Status => "↑↓/jk scroll | r refresh | Esc back | q quit",
            Screen::Help => "Esc or ? back | q quit",
        }
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_delete_confirmation(frame: &mut Frame, area: Rect, server: &str) {
    let popup = centered_rect(84, 11, area);
    frame.render_widget(Clear, popup);
    let dialog = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("Delete configuration '{}'?", sanitize_for_terminal(server)),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("An active interface will be disconnected first."),
        Line::from("Press y or Enter to delete; n or Esc to cancel."),
    ])
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .title(" Confirm delete ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(dialog, popup);
}

fn centered_rect(width_percent: u16, height: u16, area: Rect) -> Rect {
    let vertical_margin = area.height.saturating_sub(height) / 2;
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(vertical_margin),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    let horizontal_margin = (100_u16.saturating_sub(width_percent)) / 2;
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(horizontal_margin),
            Constraint::Percentage(width_percent),
            Constraint::Percentage(horizontal_margin),
        ])
        .split(vertical[1])[1]
}

fn push_detail(lines: &mut Vec<Line<'static>>, label: &str, value: Option<&str>) {
    let value = value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-")
        .to_owned();
    let value = sanitize_for_terminal(&value);
    lines.push(Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(Color::Yellow)),
        Span::styled(value, Style::default().fg(Color::White)),
    ]));
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}

fn shorten_key(key: &str) -> String {
    let mut characters = key.chars();
    let prefix: String = characters.by_ref().take(20).collect();
    if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn sanitize_for_terminal(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if is_unsafe_terminal_character(character) {
                '�'
            } else {
                character
            }
        })
        .collect()
}

fn is_unsafe_terminal_character(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{00ad}'
                | '\u{034f}'
                | '\u{061c}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    fn buffer_text(buffer: &Buffer) -> String {
        let mut rendered = String::new();
        for y in buffer.area.y..buffer.area.bottom() {
            for x in buffer.area.x..buffer.area.right() {
                rendered.push_str(buffer[(x, y)].symbol());
            }
            rendered.push('\n');
        }
        rendered
    }

    fn render_details_at_80_by_24(
        details: &ConfigDetails,
        server: &str,
        live: LiveDetails<'_>,
    ) -> Buffer {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| {
                let shell = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(5),
                        Constraint::Length(3),
                        Constraint::Length(2),
                    ])
                    .split(frame.area());
                let panels = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
                    .split(shell[1]);
                render_details(
                    frame,
                    panels[1],
                    build_details_lines(server, Some(details), None, Some(live)),
                );
            })
            .expect("render details");
        terminal.backend().buffer().clone()
    }

    #[test]
    fn tiny_terminals_render_without_panicking_and_threshold_is_exact() {
        assert!(terminal_too_small(Rect::new(0, 0, 79, 24)));
        assert!(terminal_too_small(Rect::new(0, 0, 80, 23)));
        assert!(!terminal_too_small(Rect::new(0, 0, 80, 24)));

        for (width, height) in [(1, 1), (10, 3), (79, 23)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            terminal
                .draw(|frame| draw_too_small(frame, frame.area()))
                .expect("render small-terminal message");
        }

        let backend = TestBackend::new(79, 23);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| draw_too_small(frame, frame.area()))
            .expect("render minimum-size guidance");
        assert!(buffer_text(terminal.backend().buffer()).contains("80 x 24"));
    }

    #[test]
    fn main_details_fit_completely_at_80_by_24() {
        let details = ConfigDetails {
            interface_address: Some("10.0.0.2/32".to_string()),
            dns: Some("1.1.1.1".to_string()),
            peer_public_key: Some("abcdefghijklmnopqrstuvwxyz0123456789=".to_string()),
            endpoint: Some("vpn.example:51820".to_string()),
            allowed_ips: Some("0.0.0.0/0, ::/0".to_string()),
            persistent_keepalive: Some("25".to_string()),
            private_key_configured: true,
        };
        let buffer = render_details_at_80_by_24(
            &details,
            "example",
            LiveDetails {
                stale: false,
                handshake: Some("12 seconds ago"),
                received: Some("1.2 MiB"),
                sent: Some("345 KiB"),
                public_ip: Some("203.0.113.7"),
            },
        );
        let rendered = buffer_text(&buffer);

        assert!(rendered.contains("Address: 10.0.0.2/32"));
        assert!(rendered.contains("Live status"));
        assert!(rendered.contains("Public IP: 203.0.113.7"));
    }

    #[test]
    fn unicode_is_preserved_and_terminal_controls_never_reach_cells() {
        let details = ConfigDetails {
            interface_address: Some("香港🚀\u{1b}[31m\nspoof".to_string()),
            dns: Some("1.1.1.1\u{202e}hidden".to_string()),
            peer_public_key: Some("密钥\tvalue".to_string()),
            endpoint: Some("東京.example:51820".to_string()),
            allowed_ips: Some("0.0.0.0/0".to_string()),
            persistent_keepalive: Some("25".to_string()),
            private_key_configured: true,
        };
        let buffer = render_details_at_80_by_24(
            &details,
            "香港🚀\u{1b}[2J",
            LiveDetails {
                stale: true,
                handshake: Some("刚刚\rnow"),
                received: Some("1 MiB"),
                sent: Some("2 MiB"),
                public_ip: Some("203.0.113.9\u{7}"),
            },
        );
        let rendered = buffer_text(&buffer);

        // Wide glyphs occupy a continuation cell in Ratatui's buffer, so
        // inspect them individually instead of requiring adjacent cell text.
        for character in ['香', '港', '🚀', '東', '京'] {
            assert!(rendered.contains(character));
        }
        assert!(rendered.contains(".example"));
        assert!(rendered.contains('�'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{202e}'));
        assert!(buffer.content.iter().all(|cell| cell
            .symbol()
            .chars()
            .all(|character| !is_unsafe_terminal_character(character))));
    }

    #[test]
    fn delete_confirmation_fits_at_the_minimum_supported_size() {
        let backend = TestBackend::new(MIN_TERMINAL_WIDTH, MIN_TERMINAL_HEIGHT);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| draw_delete_confirmation(frame, frame.area(), "server-1234567\u{1b}[31m"))
            .expect("render delete confirmation");
        let rendered = buffer_text(terminal.backend().buffer());

        assert!(rendered.contains("Delete configuration"));
        assert!(rendered.contains("An active interface will be"));
        assert!(rendered.contains("Press y or Enter to delete"));
        assert!(rendered.contains("n or Esc"));
        assert!(rendered.contains("cancel."));
        assert!(!rendered.contains('\u{1b}'));
    }
}
