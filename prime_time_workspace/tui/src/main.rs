use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::stream::StreamExt;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Sparkline},
};
use serde::Deserialize;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

#[derive(Deserialize, Clone, Default)]
struct SimUpdate {
    tick: u64,
    net_entropy: i64,
    void_events: i64,
    surplus_events: i64,
    total_wealth: u64,
    total_vault_books: u64,
    books_standard: u64,
    books_heuristic: u64,
    agent_deltas: Vec<i64>, // Explicitly including this so bincode deserializes fully
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Connect to WebSocket
    let url = "ws://127.0.0.1:3005/ws";
    let ws_stream_result = connect_async(url).await;

    if ws_stream_result.is_err() {
        disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        println!("Failed to connect to the server. Is it running?");
        return Ok(());
    }

    let (ws_stream, _) = ws_stream_result.unwrap();
    let (_, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            if let Ok(tokio_tungstenite::tungstenite::Message::Binary(bin)) = msg {
                if let Ok(update) = bincode::deserialize::<SimUpdate>(&bin) {
                    let _ = tx.send(update).await;
                }
            }
        }
    });

    let mut latest_update = SimUpdate::default();
    let mut entropy_history = vec![0; 100]; // Keep history for sparkline

    loop {
        // Handle WS updates
        while let Ok(update) = rx.try_recv() {
            latest_update = update;
            // Sparklines don't do well with negative numbers, scale or absolute value
            entropy_history.push(latest_update.net_entropy.abs() as u64);
            if entropy_history.len() > 100 {
                entropy_history.remove(0);
            }
        }

        // Draw TUI
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Length(7),
                        Constraint::Min(5),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            let header = Paragraph::new(format!(
                "🔥 PRIME-TIME ABM ENGINE - TUI MONITOR | TICK: {}",
                latest_update.tick
            ))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            let stats_text = format!(
                "  Net Entropy: {}\n  Total Wealth: {}\n  Vault Books: {} (Std: {}, Heu: {})\n  Voids: {} | Surplus: {}",
                latest_update.net_entropy,
                latest_update.total_wealth,
                latest_update.total_vault_books,
                latest_update.books_standard,
                latest_update.books_heuristic,
                latest_update.void_events,
                latest_update.surplus_events
            );

            let stats = Paragraph::new(stats_text)
                .block(Block::default().title(" Global State ").borders(Borders::ALL));
            f.render_widget(stats, chunks[1]);

            let sparkline = Sparkline::default()
                .block(Block::default().title(" Absolute Entropy Sparkline ").borders(Borders::ALL))
                .data(&entropy_history)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(sparkline, chunks[2]);
        })?;

        // Handle Input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Restore Terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
