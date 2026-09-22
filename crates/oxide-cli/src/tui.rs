use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use oxide_core::memory::MemoryRecord;
use oxide_core::resolve_db_path;
use oxide_core::symbol::SymbolRecord;
use oxide_db::store::ProjectStore;
use oxide_db::surreal::SurrealProjectStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Toc,
    Preview,
    Memory,
}

pub struct TuiApp {
    pub project_name: String,
    pub active_pane: ActivePane,
    pub symbols: Vec<SymbolRecord>,
    pub memories: Vec<MemoryRecord>,
    pub symbol_list_state: ListState,
    pub search_query: String,
    pub search_mode: bool,
    pub status_msg: String,
    pub should_quit: bool,
}

impl TuiApp {
    pub async fn new(project_root: &Path) -> anyhow::Result<Self> {
        let mut symbols = Vec::new();
        let mut memories = Vec::new();

        if let Ok(db_path) = resolve_db_path(project_root)
            && let Ok(store) = SurrealProjectStore::open(&db_path).await
        {
            if let Ok(hits) = store.stair_search("", 500).await {
                for h in hits {
                    let fid = oxide_core::id::FileId::from_relative_path(&h.file_path);
                    let sym_id = oxide_core::id::SymbolId::new(&fid, &h.leaf_symbol);
                    symbols.push(SymbolRecord {
                        id: sym_id,
                        file_id: fid,
                        kind: oxide_core::symbol::SymbolKind::Function,
                        name: h.leaf_symbol,
                        qualified_name: None,
                        start_line: h.start_line,
                        end_line: h.end_line,
                        signature: h.signature,
                        doc: None,
                        fingerprint: String::new(),
                        is_macro_node: h.macro_parent.is_some(),
                        parent_id: None,
                        breadcrumbs: h.breadcrumbs,
                        summary: if h.code_body.is_empty() {
                            None
                        } else {
                            Some(h.code_body)
                        },
                    });
                }
            }
            if let Ok(all_mems) = store.list_all_memories().await {
                memories = all_mems;
            }
        }

        let mut symbol_list_state = ListState::default();
        if !symbols.is_empty() {
            symbol_list_state.select(Some(0));
        }

        let project_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
            .to_string();

        Ok(Self {
            project_name,
            active_pane: ActivePane::Toc,
            symbols,
            memories,
            symbol_list_state,
            search_query: String::new(),
            search_mode: false,
            status_msg: "Press [Tab] to switch pane | [/] Search | [j/k] Navigate | [q] Quit"
                .to_string(),
            should_quit: false,
        })
    }

    pub fn filtered_symbols(&self) -> Vec<&SymbolRecord> {
        if self.search_query.is_empty() {
            self.symbols.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.symbols
                .iter()
                .filter(|s| {
                    s.name.to_lowercase().contains(&q)
                        || s.breadcrumbs.iter().any(|b| b.to_lowercase().contains(&q))
                })
                .collect()
        }
    }

    pub fn selected_symbol(&self) -> Option<&SymbolRecord> {
        let filtered = self.filtered_symbols();
        if filtered.is_empty() {
            return None;
        }
        let idx = self.symbol_list_state.selected().unwrap_or(0);
        filtered.get(idx).copied()
    }

    pub fn next_symbol(&mut self) {
        let count = self.filtered_symbols().len();
        if count == 0 {
            return;
        }
        let current = self.symbol_list_state.selected().unwrap_or(0);
        let next = if current + 1 >= count { 0 } else { current + 1 };
        self.symbol_list_state.select(Some(next));
    }

    pub fn prev_symbol(&mut self) {
        let count = self.filtered_symbols().len();
        if count == 0 {
            return;
        }
        let current = self.symbol_list_state.selected().unwrap_or(0);
        let prev = if current == 0 { count - 1 } else { current - 1 };
        self.symbol_list_state.select(Some(prev));
    }

    pub fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Toc => ActivePane::Preview,
            ActivePane::Preview => ActivePane::Memory,
            ActivePane::Memory => ActivePane::Toc,
        };
    }
}

pub async fn run_tui(project_root: &Path) -> ExitCode {
    if let Err(e) = run_tui_inner(project_root).await {
        eprintln!("TUI error: {}", e);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run_tui_inner(project_root: &Path) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(project_root).await?;

    let res = run_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

async fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if app.search_mode {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        app.search_mode = false;
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.should_quit = true;
                    }
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Tab => {
                        app.toggle_pane();
                    }
                    KeyCode::Char('/') => {
                        app.search_mode = true;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.next_symbol();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.prev_symbol();
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main 3-pane layout
            Constraint::Length(3), // Search / Command bar
            Constraint::Length(1), // Footer status
        ])
        .split(f.area());

    // 1. Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let title_line = Line::from(vec![
        Span::styled(
            " Oxide-Embed ⚡ ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "Workspace: {} | AST Nodes: {}",
            app.project_name,
            app.symbols.len()
        )),
    ]);
    let header = Paragraph::new(title_line).block(header_block);
    f.render_widget(header, chunks[0]);

    // 2. Middle (3 Panes: Code-ToC, Preview, Graph & Memory)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Code-ToC Tree
            Constraint::Percentage(45), // Symbol & Body Preview
            Constraint::Percentage(25), // Memory & Graph Info
        ])
        .split(chunks[1]);

    // Pane 1: Code-ToC
    let filtered_symbols = app.filtered_symbols();
    let items: Vec<ListItem> = filtered_symbols
        .iter()
        .map(|s| {
            let prefix = if s.is_macro_node { "📦 " } else { "  ● " };
            let display_text = format!("{}{:<18} [{}]", prefix, s.name, s.kind.as_str());
            ListItem::new(display_text)
        })
        .collect();

    let toc_style = if app.active_pane == ActivePane::Toc {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    let toc_list = List::new(items)
        .block(
            Block::default()
                .title(" 1. Code-ToC Hierarchy ")
                .borders(Borders::ALL)
                .border_style(toc_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(toc_list, main_chunks[0], &mut app.symbol_list_state);

    // Pane 2: Preview
    let preview_style = if app.active_pane == ActivePane::Preview {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    let preview_content = if let Some(sym) = app.selected_symbol() {
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("Symbol: ", Style::default().fg(Color::Yellow)),
            Span::styled(&sym.name, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" ({})", sym.kind.as_str())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Lines:  ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}-{}", sym.start_line, sym.end_line)),
        ]));
        if !sym.breadcrumbs.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Breadcrumb: ", Style::default().fg(Color::Cyan)),
                Span::raw(sym.breadcrumbs.join(" > ")),
            ]));
        }
        lines.push(Line::from(""));
        if let Some(sig) = &sym.signature {
            lines.push(Line::from(vec![
                Span::styled("Signature:\n", Style::default().fg(Color::Magenta)),
                Span::raw(sig),
            ]));
            lines.push(Line::from(""));
        }
        if let Some(doc) = &sym.doc {
            lines.push(Line::from(vec![
                Span::styled("Docstring:\n", Style::default().fg(Color::Blue)),
                Span::raw(doc),
            ]));
        }
        lines
    } else {
        vec![Line::from("No symbol selected")]
    };

    let preview = Paragraph::new(preview_content)
        .block(
            Block::default()
                .title(" 2. Symbol AST & Code Context ")
                .borders(Borders::ALL)
                .border_style(preview_style),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(preview, main_chunks[1]);

    // Pane 3: Graph & Memory
    let mem_style = if app.active_pane == ActivePane::Memory {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Gray)
    };

    let mut side_content = vec![
        Line::from(Span::styled(
            "GraphRAG & Memory Layers",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("• Total Memories: {}", app.memories.len())),
        Line::from("• Ebbinghaus Decay: Active"),
        Line::from(""),
    ];

    if app.memories.is_empty() {
        side_content.push(Line::from(Span::styled(
            "No memories recorded yet.",
            Style::default().fg(Color::DarkGray),
        )));
        side_content.push(Line::from("Run: oxide-embed remember"));
    } else {
        for m in app.memories.iter().take(6) {
            side_content.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", m.kind.as_str()),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(&m.title, Style::default().fg(Color::White)),
            ]));
        }
    }

    let side_pane = Paragraph::new(side_content).block(
        Block::default()
            .title(" 3. Graph & Memory ")
            .borders(Borders::ALL)
            .border_style(mem_style),
    );
    f.render_widget(side_pane, main_chunks[2]);

    // 3. Search / Query Bar
    let search_title = if app.search_mode {
        " Query (Typing - [Enter]/[Esc] to finish) "
    } else {
        " Search (Press [/] to filter) "
    };
    let search_bar = Paragraph::new(app.search_query.as_str()).block(
        Block::default()
            .title(search_title)
            .borders(Borders::ALL)
            .border_style(if app.search_mode {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            }),
    );
    f.render_widget(search_bar, chunks[2]);

    // 4. Footer
    let footer =
        Paragraph::new(app.status_msg.as_str()).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}
