// Tokio async runtime
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

// Crossterm
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

// Ratatui
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

// Standard library
use std::error::Error;

// Common Crate
use common::{ChatClient, Client, ClientId, RoomId};

// App state
struct App {
    messages: Vec<String>,
    input: String,
    status: String,
    client_id: Option<u64>,
    current_room: Option<String>,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            status: "Connecting...".to_string(),
            client_id: None,
            current_room: None,
            should_quit: false,
        }
    }

    fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    fn enter_char(&mut self, c: char) {
        self.input.push(c);
    }

    fn delete_char(&mut self) {
        self.input.pop();
    }

    fn drain_input(&mut self) -> String {
        self.input.drain(..).collect()
    }
}

pub struct RuntimeClient {
    pub client: Client,
    pub rx: mpsc::Receiver<String>,
    pub server_addr: String,
}
impl RuntimeClient {
    pub fn new(id: ClientId, server_addr: String, name: String) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let chat_client = ChatClient { tx };
        let client = Client {
            id,
            name: Some(name),
            message: chat_client,
            current_room: Some(RoomId("0".to_string())),
        };

        Self {
            client,
            rx,
            server_addr,
        }
    }
    pub async fn send_message(&self, msg: String) -> Result<(), common::Errors> {
        self.client.send(msg).await?;
        Ok(())
    }
    pub async fn run(mut self) -> Result<(), Box<dyn Error>> {
        // Enter TUI mode
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // App state shared between tasks
        let mut app = App::new();

        // Network Connection
        let stream = TcpStream::connect(&self.server_addr).await?;
        let (reader, mut writer) = stream.into_split();

        let mut reader = BufReader::new(reader);

        // Reading initial clientID from server
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line.starts_with("CLIENT_ID:") {
            let id_str = line["CLIENT_ID:".len()..].trim();
            let id: u64 = id_str.parse().expect("Invalid Client Id from Server");
            self.client.id = ClientId(id); // This updates to correct client id 
            println!("Assigned Client ID: {}", id);
        }

        // Channels for communication between tasks
        let (ui_tx, mut ui_rx) = mpsc::channel::<String>(100);

        // Channel for status updates
        let (status_tx, mut status_rx) = mpsc::channel::<String>(100);

        // Writer Tasks, takes inputs from terminal to be sent to server
        let mut rx = self.rx;
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let _ = writer.write_all(msg.as_bytes()).await;
                let _ = writer.write_all(b"\n").await;
            }
        });

        // Reader task, reading from TCPSTREAM
        //let tx = self.client.message.tx.clone();
        let reader_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                let _ = ui_tx.send(line.trim().to_string()).await;
                line.clear();
            }
            let _ = status_tx.send("Disconnected".to_string()).await;
        });

        // Main event loop
        let tick_rate = std::time::Duration::from_millis(100);
        let mut last_tick = std::time::Instant::now();

        loop {
            // Drawing UI
            terminal.draw(|f| ui(f, &app))?;

            // Handle timeout for tick rate
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| std::time::Duration::from_secs(0));

            // Checking for keyboard events
            if crossterm::event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    // Only process Key press
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('c')
                                if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                            {
                                app.should_quit = true;
                            }

                            // Enter to send message
                            KeyCode::Enter => {
                                let msg = app.drain_input();
                                if !msg.is_empty() {
                                    app.add_message(format!("You: {}", msg));

                                    if msg.eq_ignore_ascii_case("/quit") {
                                        app.should_quit = true;
                                    }

                                    // Send to server
                                    if let Err(_) = self.client.message.tx.send(msg).await {
                                        app.status = "Failed to send message".to_string();
                                    }
                                }
                            }

                            // Backspace to delete char
                            KeyCode::Backspace => {
                                app.delete_char();
                            }

                            // Regular char input
                            KeyCode::Char(c) => {
                                app.enter_char(c);
                            }

                            _ => {}
                        }
                    }
                }
            }

            // Check for new messages from server (non_blocking)
            while let Ok(msg) = ui_rx.try_recv() {
                app.add_message(format!("📨 {}", msg));
            }

            // Check for status updates (non_blocking)
            while let Ok(status) = status_rx.try_recv() {
                app.status = status;
            }

            // Check to quit
            if app.should_quit {
                break;
            }

            // Update tick timer
            if last_tick.elapsed() >= tick_rate {
                last_tick = std::time::Instant::now();
            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        // Clean up tasks
        writer_handle.abort();
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(2), reader_handle).await;

        println!("Disconnected");
        Ok(())
    }
}

// UI rendering function (drawing the interface)
fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Message Area
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| {
            let content = Line::from(Span::raw(m));
            ListItem::new(content)
        })
        .collect();

    let messages_widget = List::new(messages).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Chat Messages")
            .style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(messages_widget, chunks[0]);

    // Input area, shows what user is typing
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input (Press Enter to send, ESC to quit)"),
        );
    f.render_widget(input, chunks[1]);

    // Status Bar, Shows connection status and info
    let status_text = if let Some(id) = app.client_id {
        format!(
            " Status: {} | Client ID {} | Room: {} ",
            app.status,
            id,
            app.current_room.as_ref().unwrap_or(&"None".to_string())
        )
    } else {
        format!(" Status: {} ", app.status)
    };

    let status = Paragraph::new(status_text).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(status, chunks[2]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut name = String::new();
    println!("Please Enter A Username: ");
    io::stdin().read_line(&mut name)?;
    let name = name.trim().to_string();

    if name.is_empty() {
        println!("Username cannot be empty");
        return Ok(());
    }
    let client = RuntimeClient::new(ClientId(0), "127.0.0.1:8080".to_string(), name);

    client.run().await
}

fn display_to_terminal(line: String) {
    println!("{}", line);
}
