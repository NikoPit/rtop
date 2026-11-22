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
use ratatui::text::Span;
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState};
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
    search: String,
    searching: bool,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            system: System::new_all(),
            processes: Vec::new(),
            selected_pid: None,
            status: String::new(),
            search: String::new(),
            searching: false,
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

        self.keep_selection_visible();
    }

    fn move_selection(&mut self, offset: isize) {
        let visible = self.visible_process_indices();
        if visible.is_empty() {
            self.selected_pid = None;
            return;
        }

        let current = visible
            .iter()
            .position(|&index| Some(self.processes[index].pid) == self.selected_pid)
            .unwrap_or(0);
        let next = (current as isize + offset).clamp(0, visible.len() as isize - 1) as usize;
        self.selected_pid = Some(self.processes[visible[next]].pid);
    }

    fn visible_process_indices(&self) -> Vec<usize> {
        let query = self.search.to_lowercase();
        self.processes
            .iter()
            .enumerate()
            .filter_map(|(index, process)| {
                let matches_name = process.name.to_lowercase().contains(&query);
                let matches_pid = process.pid.as_u32().to_string().contains(&query);
                (matches_name || matches_pid).then_some(index)
            })
            .collect()
    }

    fn keep_selection_visible(&mut self) {
        let visible = self.visible_process_indices();
        if !visible
            .iter()
            .any(|&index| Some(self.processes[index].pid) == self.selected_pid)
        {
            self.selected_pid = visible.first().map(|&index| self.processes[index].pid);
        }
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
        self.visible_process_indices()
            .iter()
            .position(|&index| Some(self.processes[index].pid) == self.selected_pid)
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
            if app.searching {
                match key.code {
                    KeyCode::Esc => {
                        app.search.clear();
                        app.searching = false;
                        app.keep_selection_visible();
                    }
                    KeyCode::Enter => app.searching = false,
                    KeyCode::Backspace => {
                        app.search.pop();
                        app.keep_selection_visible();
                    }
                    KeyCode::Char(character) => {
                        app.search.push(character);
                        app.keep_selection_visible();
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('k') => app.kill_selected(),
                    KeyCode::Char('/') => {
                        app.search.clear();
                        app.searching = true;
                        app.status.clear();
                        app.keep_selection_visible();
                    }
                    KeyCode::Up => app.move_selection(-1),
                    KeyCode::Down => app.move_selection(1),
                    _ => {}
                }
            }
        }

        if last_refresh.elapsed() >= Duration::from_secs(1) {
            app.refresh();
            last_refresh = Instant::now();
        }
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(frame.area());

    let header = Row::new(["PID", "NAME", "CPU%", "MEMORY"]).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    let visible = app.visible_process_indices();
    let rows = visible.iter().map(|&index| {
        let process = &app.processes[index];
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
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let mut table_state = TableState::default();
    table_state.select(app.selected_index());
    frame.render_stateful_widget(table, areas[0], &mut table_state);

    let footer_text = if app.searching || !app.search.is_empty() {
        format!("/{}", app.search)
    } else {
        app.status.clone()
    };
    let footer = Paragraph::new(Span::styled(
        footer_text,
        Style::default().fg(Color::Yellow),
    ));
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

#[cfg(test)]
mod tests {
    use super::{App, ProcessInfo};
    use sysinfo::{Pid, System};

    fn app_with_processes() -> App {
        App {
            system: System::new(),
            processes: vec![
                ProcessInfo {
                    pid: Pid::from_u32(42),
                    name: String::from("worker"),
                    cpu_usage: 0.0,
                    memory: 0,
                },
                ProcessInfo {
                    pid: Pid::from_u32(108),
                    name: String::from("database"),
                    cpu_usage: 0.0,
                    memory: 0,
                },
            ],
            selected_pid: Some(Pid::from_u32(42)),
            status: String::new(),
            search: String::new(),
            searching: false,
        }
    }

    #[test]
    fn search_matches_names_without_case_sensitivity() {
        let mut app = app_with_processes();
        app.search = String::from("DATA");

        assert_eq!(app.visible_process_indices(), vec![1]);
    }

    #[test]
    fn search_matches_pid_and_keeps_selection_in_results() {
        let mut app = app_with_processes();
        app.search = String::from("108");
        app.keep_selection_visible();

        assert_eq!(app.visible_process_indices(), vec![1]);
        assert_eq!(app.selected_pid, Some(Pid::from_u32(108)));
    }
}
