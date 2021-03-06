use super::state::{ApplicationState, MessageType, TermchatMessageType};
use super::util::split_each;
use crate::util::Result;

use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::{Frame, Terminal};

use std::io::Stdout;

pub fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &ApplicationState,
) -> Result<()> {
    Ok(terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(6)].as_ref())
            .split(frame.size());

        draw_messages_panel(frame, state, chunks[0]);
        draw_input_panel(frame, state, chunks[1]);
    })?)
}

fn draw_messages_panel(
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    state: &ApplicationState,
    chunk: Rect,
) {
    const MESSAGE_COLORS: [Color; 4] = [Color::Blue, Color::Yellow, Color::Cyan, Color::Magenta];

    let mut messages = state
        .messages()
        .iter()
        .rev()
        .map(|message| {
            let color = if let Some(id) = state.users_id().get(&message.user) {
                MESSAGE_COLORS[id % MESSAGE_COLORS.len()]
            } else {
                Color::Green //because is a message of the own user
            };
            let date = message.date.format("%H:%M:%S ").to_string();
            match &message.message_type {
                MessageType::Connection => Spans::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(" is online", Style::default().fg(color)),
                ]),
                MessageType::Disconnection => Spans::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(" is offline", Style::default().fg(color)),
                ]),
                MessageType::Content(content) => {
                    let mut ui_message = vec![
                        Span::styled(date, Style::default().fg(Color::DarkGray)),
                        Span::styled(&message.user, Style::default().fg(color)),
                        Span::styled(": ", Style::default().fg(color)),
                    ];
                    ui_message.extend(parse_content(content));
                    Spans::from(ui_message)
                }
                MessageType::Termchat(content, msg_type) => {
                    let (user_color, content_color) = match msg_type {
                        TermchatMessageType::Notification => (Color::Yellow, Color::LightYellow),
                        TermchatMessageType::Error => (Color::Red, Color::LightRed),
                    };
                    Spans::from(vec![
                        Span::styled(date, Style::default().fg(Color::DarkGray)),
                        Span::styled(&message.user, Style::default().fg(user_color)),
                        Span::styled(content, Style::default().fg(content_color)),
                    ])
                }
            }
        })
        .collect::<Vec<_>>();

    // check if there is a file being sent and if so draw the progress bar
    if let Some(progress) = state.progress() {
        add_progress_bar(&mut messages, chunk.width, progress);
    }

    let messages_panel = Paragraph::new(messages)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            "LAN Room",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .scroll((state.scroll_messages_view() as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_panel, chunk);
}

fn add_progress_bar(messages: &mut Vec<Spans>, panel_width: u16, progress: (usize, usize)) {
    let (current, max) = progress;
    let color = Color::LightGreen;

    let width = panel_width - 20;
    let ui_step = width as f32 / max as f32;
    let ui_current = (current as f32 * ui_step) as usize;
    let ui_remaining = ((max.saturating_sub(current)) as f32 * ui_step) as usize;

    let current: String = std::iter::repeat("#").take(ui_current).collect();
    let remaining: String = std::iter::repeat("-").take(ui_remaining).collect();
    let msg = format!("[{}{}]", current, remaining);
    let ui_message = vec![
        Span::styled("Sending: ", Style::default().fg(color)),
        Span::styled(msg, Style::default().fg(color)),
    ];
    messages.insert(0, Spans::from(ui_message));
}

fn parse_content(content: &str) -> Vec<Span> {
    let color_command = |command| {
        content
            .splitn(2, command)
            .enumerate()
            .map(|(index, part)| {
                // ?send
                if index == 0 {
                    Span::styled(command, Style::default().fg(Color::LightYellow))
                } else {
                    Span::raw(part)
                }
            })
            .collect()
    };

    const SEND_COMMAND: &str = "?send";

    if content.starts_with(SEND_COMMAND) {
        color_command(SEND_COMMAND)
    // other commands can be handled here the same way
    } else {
        vec![Span::raw(content)]
    }
}

fn draw_input_panel(
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    state: &ApplicationState,
    chunk: Rect,
) {
    let inner_width = (chunk.width - 2) as usize;

    let input = state.input().iter().collect::<String>();
    let input = split_each(input, inner_width)
        .into_iter()
        .map(|line| Spans::from(vec![Span::raw(line)]))
        .collect::<Vec<_>>();

    let input_panel = Paragraph::new(input)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            "Your message",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    frame.render_widget(input_panel, chunk);

    let input_cursor = state.ui_input_cursor(inner_width);
    frame.set_cursor(chunk.x + 1 + input_cursor.0, chunk.y + 1 + input_cursor.1)
}
