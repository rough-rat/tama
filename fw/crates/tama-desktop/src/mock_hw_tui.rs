use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::io;
use log::{Level, Record, Metadata, LevelFilter};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};

// Shared sensor state - matches tama_core::input::SensorType enum
#[derive(Clone, Debug)]
pub struct MockSensorState {
    pub battery_level: f32,  // Volts (2.5 - 4.2)
    pub temperature: f32,       // Celsius (-40 - 80)
    pub light_level: f32,       // 0.0 - 1.0
    pub accelerometer: f32,     // Movement intensity (0.0 - 1.0)
    pub mic_loudness: f32,      // Audio level (0.0 - 1.0)
}

impl Default for MockSensorState {
    fn default() -> Self {
        Self {
            battery_level: 3.7,
            temperature: 25.0,
            light_level: 0.5,
            accelerometer: 0.0,
            mic_loudness: 0.0,
        }
    }
}

// Log entry (now using standard log::Level)
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: Level,
    pub message: String,
}

// Extension trait for log::Level to provide UI rendering methods
trait LevelExt {
    fn color(&self) -> Color;
    fn prefix(&self) -> &str;
}

impl LevelExt for Level {
    fn color(&self) -> Color {
        match *self {
            Level::Error => Color::Red,
            Level::Warn => Color::Yellow,
            Level::Info => Color::Cyan,
            Level::Debug => Color::Gray,
            Level::Trace => Color::DarkGray,
        }
    }

    fn prefix(&self) -> &str {
        match *self {
            Level::Error => "[ERROR]",
            Level::Warn => "[WARN] ",
            Level::Info => "[INFO] ",
            Level::Debug => "[DEBUG]",
            Level::Trace => "[TRACE]",
        }
    }
}

// TUI state
struct TuiState {
    sensor_state: Arc<Mutex<MockSensorState>>,
    logs: Vec<LogEntry>,
    rx: Receiver<TuiMessage>,
    selected_sensor: usize,
    max_logs: usize,
    should_quit: bool,
}

impl TuiState {
    fn new(sensor_state: Arc<Mutex<MockSensorState>>, rx: Receiver<TuiMessage>) -> Self {
        Self {
            sensor_state,
            logs: Vec::new(),
            rx,
            selected_sensor: 0,
            max_logs: 100,
            should_quit: false,
        }
    }

    fn collect_messages(&mut self) {
        // Collect all pending messages
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                TuiMessage::Log(log) => {
                    self.logs.push(log);
                    if self.logs.len() > self.max_logs {
                        self.logs.remove(0);
                    }
                }
                TuiMessage::Shutdown => {
                    self.should_quit = true;
                }
            }
        }
    }

    fn adjust_sensor(&mut self, increase: bool) {
        let mut state = self.sensor_state.lock().unwrap();
        let delta = if increase { 1.0 } else { -1.0 };

        match self.selected_sensor {
            0 => state.battery_level = (state.battery_level + delta * 0.1).clamp(2.5, 4.2),
            1 => state.temperature = (state.temperature + delta).clamp(-40.0, 80.0),
            2 => state.light_level = (state.light_level + delta * 0.1).clamp(0.0, 1.0),
            3 => state.accelerometer = (state.accelerometer + delta * 0.1).clamp(0.0, 1.0),
            4 => state.mic_loudness = (state.mic_loudness + delta * 0.1).clamp(0.0, 1.0),
            _ => {}
        }
    }
}

// Shutdown signal for TUI
enum TuiMessage {
    Log(LogEntry),
    Shutdown,
}

// Logger implementation that sends logs to the TUI
pub struct TuiLogger {
    tx: Sender<TuiMessage>,
}

impl TuiLogger {
    fn new(tx: Sender<TuiMessage>) -> Self {
        Self { tx }
    }
}

impl log::Log for TuiLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true // Accept all log levels
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let entry = LogEntry {
                level: record.level(),
                message: format!("{}", record.args()),
            };
            // Ignore send errors (TUI might have shut down)
            let _ = self.tx.send(TuiMessage::Log(entry));
        }
    }

    fn flush(&self) {}
}

// Public handle for the TUI
pub struct MockHwTui {
    sensor_state: Arc<Mutex<MockSensorState>>,
    tx: Sender<TuiMessage>,
}

impl MockHwTui {
    pub fn new() -> Result<Self, anyhow::Error> {
        let sensor_state = Arc::new(Mutex::new(MockSensorState::default()));
        let (tx, rx) = channel();

        let sensor_state_clone = Arc::clone(&sensor_state);

        // Spawn TUI thread
        thread::spawn(move || {
            if let Err(e) = run_tui(sensor_state_clone, rx) {
                eprintln!("TUI error: {}", e);
            }
        });

        // Try to initialize the logger, but don't fail if one is already set
        // (log_capture may have set one already)
        let logger = TuiLogger::new(tx.clone());
        if log::set_boxed_logger(Box::new(logger)).is_ok() {
            log::set_max_level(LevelFilter::Trace);
        }
        // If logger was already set, that's fine - logs will go to log_capture instead

        Ok(Self {
            sensor_state,
            tx,
        })
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(TuiMessage::Shutdown);
        // Give the TUI thread time to clean up
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    pub fn get_sensor_state(&self) -> MockSensorState {
        self.sensor_state.lock().unwrap().clone()
    }
}

impl Drop for MockHwTui {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_tui(
    sensor_state: Arc<Mutex<MockSensorState>>,
    rx: Receiver<TuiMessage>,
) -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tui_state = TuiState::new(sensor_state, rx);

    // Initial log
    tui_state.logs.push(LogEntry {
        level: Level::Info,
        message: "Mock Hardware TUI started".to_string(),
    });

    loop {
        tui_state.collect_messages();
        
        // Check if we should quit
        if tui_state.should_quit {
            break;
        }

        terminal.draw(|f| ui(f, &tui_state))?;

        // Poll for events with timeout
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        log::info!("Ctrl+C pressed, shutting down TUI");
                        break;
                    }
                    KeyCode::Up => {
                        if tui_state.selected_sensor > 0 {
                            tui_state.selected_sensor -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if tui_state.selected_sensor < 4 {  // 5 sensors (0-4)
                            tui_state.selected_sensor += 1;
                        }
                    }
                    KeyCode::Left | KeyCode::Char('-') => {
                        tui_state.adjust_sensor(false);
                    }
                    KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                        tui_state.adjust_sensor(true);
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Sensors
            Constraint::Min(8),     // Logs
            Constraint::Length(3),  // Help
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("Mock Hardware Control Panel")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Sensors
    render_sensors(f, chunks[1], state);

    // Logs
    render_logs(f, chunks[2], state);

    // Help
    let help = Paragraph::new("↑/↓: Select sensor | ←/→ or +/-: Adjust value | Q/ESC/Ctrl+C: Quit")
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[3]);
}

fn render_sensors(f: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Sensor Values (adjust with ←/→ or +/-)");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let sensor_state = state.sensor_state.lock().unwrap();

    // Matches tama_core::input::SensorType enum order
    let sensors = vec![
        ("Battery Voltage", sensor_state.battery_level, "%", 0.0, 100.0),
        ("Temperature", sensor_state.temperature, "°C", -40.0, 80.0),
        ("Light Level", sensor_state.light_level, "", 0.0, 1.0),
        ("Accelerometer", sensor_state.accelerometer, "", 0.0, 1.0),
        ("Mic Loudness", sensor_state.mic_loudness, "", 0.0, 1.0),
    ];

    let sensor_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(2); 5])
        .split(inner);

    for (i, (name, value, unit, min, max)) in sensors.iter().enumerate() {
        let is_selected = i == state.selected_sensor;
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let ratio = ((value - min) / (max - min)).clamp(0.0, 1.0);
        let label = format!("{}: {:.2}{}", name, value, unit);

        let gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(style)
            .ratio(ratio as f64)
            .label(label);

        f.render_widget(gauge, sensor_layout[i]);
    }
}

fn render_logs(f: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Logs (scrolls automatically)");

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Show last N logs that fit in the area
    let max_logs = inner.height as usize;
    let start_idx = if state.logs.len() > max_logs {
        state.logs.len() - max_logs
    } else {
        0
    };

    let log_items: Vec<ListItem> = state.logs[start_idx..]
        .iter()
        .map(|log| {
            let content = Line::from(vec![
                Span::styled(log.level.prefix(), Style::default().fg(log.level.color())),
                Span::raw(" "),
                Span::raw(&log.message),
            ]);
            ListItem::new(content)
        })
        .collect();

    let logs_list = List::new(log_items)
        .block(Block::default());

    f.render_widget(logs_list, inner);
}
