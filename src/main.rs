use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use sysinfo::{Pid, System};

struct ProcessInfo {
    pid: Pid,
    name: String,
    cpu_usage: f32,
    memory: u64,
}

struct App {
    system: System,
    processes: Vec<ProcessInfo>,
    selected_pid: Option<Pid>,
    status: String,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            system: System::new_all(),
            processes: Vec::new(),
            selected_pid: None,
            status: String::from("Ready"),
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.system.refresh_all();
        self.processes = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: *pid,
                name: process.name().to_string_lossy().into_owned(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            })
            .collect();
        self.processes.sort_by(|left, right| {
            right
                .cpu_usage
                .total_cmp(&left.cpu_usage)
                .then_with(|| left.name.cmp(&right.name))
        });

        if !self
            .processes
            .iter()
            .any(|process| Some(process.pid) == self.selected_pid)
        {
            self.selected_pid = self.processes.first().map(|process| process.pid);
        }
    }

    fn move_selection(&mut self, offset: isize) {
        if self.processes.is_empty() {
            self.selected_pid = None;
            return;
        }

        let current = self
            .processes
            .iter()
            .position(|process| Some(process.pid) == self.selected_pid)
            .unwrap_or(0);
        let next = (current as isize + offset).clamp(0, self.processes.len() as isize - 1) as usize;
        self.selected_pid = Some(self.processes[next].pid);
    }

    fn kill_selected(&mut self) {
        let Some(pid) = self.selected_pid else {
            self.status = String::from("No process selected");
            return;
        };

        let Some(process) = self.system.process(pid) else {
            self.status = format!("Process {} no longer exists", pid.as_u32());
            return;
        };

        let name = process.name().to_string_lossy().into_owned();
        if process.kill() {
            self.status = format!("Termination requested for {name} ({})", pid.as_u32());
        } else {
            self.status = format!("Could not terminate {name} ({})", pid.as_u32());
        }
    }

    fn selected_index(&self) -> Option<usize> {
        self.processes
            .iter()
            .position(|process| Some(process.pid) == self.selected_pid)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result?;
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    let mut last_refresh = Instant::now();

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('k') => app.kill_selected(),
                KeyCode::Up => app.move_selection(-1),
                KeyCode::Down => app.move_selection(1),
                _ => {}
            }
        }

        if last_refresh.elapsed() >= Duration::from_secs(1) {
            app.refresh();
            last_refresh = Instant::now();
        }
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let areas = Layout::vertical([Constraint::Min(5), Constraint::Length(2)]).split(frame.area());

    let header = Row::new(["PID", "NAME", "CPU%", "MEMORY"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let rows = app.processes.iter().map(|process| {
        Row::new(vec![
            Cell::from(process.pid.as_u32().to_string()),
            Cell::from(process.name.clone()),
            Cell::from(format!("{:.1}", process.cpu_usage)),
            Cell::from(format_bytes(process.memory)),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(16),
            Constraint::Length(8),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(" Processes ({}) ", app.processes.len()))
            .borders(Borders::ALL),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let mut table_state = TableState::default();
    table_state.select(app.selected_index());
    frame.render_stateful_widget(table, areas[0], &mut table_state);

    let footer = Paragraph::new(vec![
        Line::from("↑/↓ select    k terminate    q quit"),
        Line::from(Span::styled(
            app.status.as_str(),
            Style::default().fg(Color::Yellow),
        )),
    ]);
    frame.render_widget(footer, areas[1]);
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * MIB;

    if bytes as f64 >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB)
    } else {
        format!("{:.1} MiB", bytes as f64 / MIB)
    }
}
