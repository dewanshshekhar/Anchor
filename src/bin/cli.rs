//! Anchor CLI - Code intelligence for AI agents.
//!
//! Usage:
//!   anchor overview              # Codebase overview
//!   anchor search <query>        # Search symbols/files
//!   anchor context <query>       # Get full context
//!   anchor deps <symbol>         # Dependencies
//!   anchor stats                 # Graph statistics
//!   anchor build                 # Rebuild graph (with TUI)
//!   anchor build --no-tui        # Rebuild graph (CLI only)

use anchor::{build_graph, get_context, graph_search, anchor_dependencies, anchor_stats, CodeGraph};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

// TUI imports
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "anchor")]
#[command(about = "Anchor - Code intelligence for AI agents", long_about = None)]
struct Cli {
    /// Project root directory (default: current directory)
    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show codebase overview - files, key symbols, entry points
    Overview,

    /// Search for symbols or files
    Search {
        /// Query string (symbol name or file path)
        query: String,

        /// How many hops to traverse in the graph
        #[arg(short, long, default_value = "1")]
        depth: usize,
    },

    /// Get full context for a symbol (code + dependencies + dependents)
    Context {
        /// Symbol name or file path
        query: String,

        /// Intent: find, understand, modify, refactor, overview
        #[arg(short, long, default_value = "understand")]
        intent: String,
    },

    /// Show what depends on a symbol and what it depends on
    Deps {
        /// Symbol name
        symbol: String,
    },

    /// Show graph statistics
    Stats,

    /// Rebuild the code graph from scratch
    Build {
        /// Disable TUI visualization (use plain CLI output)
        #[arg(long)]
        no_tui: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let root = cli.root.canonicalize().unwrap_or(cli.root);
    let cache_path = root.join(".anchor").join("graph.bin");

    match cli.command {
        Commands::Build { no_tui } => {
            if no_tui || !atty::is(atty::Stream::Stdout) {
                // CLI mode
                build_cli_mode(&root, &cache_path)?;
            } else {
                // TUI mode (default)
                build_tui_mode(&root, &cache_path)?;
            }
            return Ok(());
        }
        _ => {}
    }

    // For other commands, load the existing graph
    let graph = if cache_path.exists() {
        CodeGraph::load(&cache_path)?
    } else {
        eprintln!("Building graph (first run)...");
        let graph = build_graph(&root);
        std::fs::create_dir_all(cache_path.parent().unwrap())?;
        graph.save(&cache_path)?;
        graph
    };

    match cli.command {
        Commands::Overview => {
            let stats = graph.stats();
            println!("Anchor - Codebase Overview");
            println!("══════════════════════════");
            println!();
            println!("Files:   {}", stats.file_count);
            println!("Symbols: {}", stats.symbol_count);
            println!("Edges:   {}", stats.total_edges);
            println!();

            // Show top-level structure
            let result = graph_search(&graph, "src/", 0);
            if !result.matched_files.is_empty() {
                println!("Structure:");
                for file in result.matched_files.iter().take(15) {
                    println!("  {}", file.display());
                }
                if result.matched_files.len() > 15 {
                    println!("  ... and {} more", result.matched_files.len() - 15);
                }
            }
            println!();

            // Show entry points (main functions)
            let mains = graph_search(&graph, "main", 0);
            if !mains.symbols.is_empty() {
                println!("Entry points:");
                for sym in mains.symbols.iter().filter(|s| s.name == "main") {
                    println!("  {} in {}", sym.name, sym.file.display());
                }
            }
        }

        Commands::Search { query, depth } => {
            let result = graph_search(&graph, &query, depth);

            if result.symbols.is_empty() && result.matched_files.is_empty() {
                println!("No results for '{}'", query);
                return Ok(());
            }

            println!("Search: '{}' (depth={})", query, depth);
            println!();

            if !result.matched_files.is_empty() {
                println!("Files:");
                for file in &result.matched_files {
                    println!("  {}", file.display());
                }
                println!();
            }

            if !result.symbols.is_empty() {
                println!("Symbols:");
                for sym in &result.symbols {
                    println!("  {} ({}) - {}:{}", sym.name, sym.kind, sym.file.display(), sym.line);
                }
                println!();
            }

            if !result.connections.is_empty() {
                println!("Connections:");
                for conn in result.connections.iter().take(20) {
                    println!("  {} --[{}]--> {}", conn.from, conn.relationship, conn.to);
                }
                if result.connections.len() > 20 {
                    println!("  ... and {} more", result.connections.len() - 20);
                }
            }
        }

        Commands::Context { query, intent } => {
            let result = get_context(&graph, &query, &intent);
            let json = serde_json::to_string_pretty(&result).unwrap_or_default();
            println!("{}", json);
        }

        Commands::Deps { symbol } => {
            let result = anchor_dependencies(&graph, &symbol);
            let json = serde_json::to_string_pretty(&result).unwrap_or_default();
            println!("{}", json);
        }

        Commands::Stats => {
            let result = anchor_stats(&graph);
            let json = serde_json::to_string_pretty(&result).unwrap_or_default();
            println!("{}", json);
        }

        Commands::Build { .. } => {
            // Already handled above
        }
    }

    Ok(())
}

// CLI mode build (plain text output)
fn build_cli_mode(root: &PathBuf, cache_path: &PathBuf) -> Result<()> {
    eprintln!("Rebuilding graph...");
    let graph = build_graph(root);
    std::fs::create_dir_all(cache_path.parent().unwrap())?;
    graph.save(cache_path)?;

    let stats = graph.stats();
    println!("✓ Graph built");
    println!("  Files:   {}", stats.file_count);
    println!("  Symbols: {}", stats.symbol_count);
    println!("  Edges:   {}", stats.total_edges);
    Ok(())
}

#[derive(Clone)]
struct BuildProgress {
    complete: bool,
    error: Option<String>,
    stats: Option<(usize, usize, usize)>, // (files, symbols, edges)
}

// TUI mode build (visual interface)
fn build_tui_mode(root: &PathBuf, cache_path: &PathBuf) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let progress = Arc::new(Mutex::new(BuildProgress {
        complete: false,
        error: None,
        stats: None,
    }));

    let mut tui_state = TUIState::new(root.clone());
    
    // Build graph in background thread
    let progress_clone = Arc::clone(&progress);
    let root_clone = root.clone();
    let cache_path_clone = cache_path.clone();
    
    thread::spawn(move || {
        match build_graph_with_progress(&root_clone, &cache_path_clone) {
            Ok(stats) => {
                let mut prog = progress_clone.lock().unwrap();
                prog.stats = Some(stats);
                prog.complete = true;
            }
            Err(e) => {
                let mut prog = progress_clone.lock().unwrap();
                prog.error = Some(e.to_string());
                prog.complete = true;
            }
        }
    });

    let res = run_tui(&mut terminal, &mut tui_state, &progress);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn build_graph_with_progress(root: &PathBuf, cache_path: &PathBuf) -> Result<(usize, usize, usize)> {
    let graph = build_graph(root);
    std::fs::create_dir_all(cache_path.parent().unwrap())?;
    graph.save(cache_path)?;
    
    let stats = graph.stats();
    Ok((stats.file_count, stats.symbol_count, stats.total_edges))
}

struct TUIState {
    _root: PathBuf,
    messages: Vec<String>,
    animation_frame: usize,
    last_update: Instant,
    start_time: Instant,
}

impl TUIState {
    fn new(root: PathBuf) -> Self {
        Self {
            _root: root,
            messages: vec!["Rebuilding graph...".to_string()],
            animation_frame: 0,
            last_update: Instant::now(),
            start_time: Instant::now(),
        }
    }

    fn update(&mut self, progress: &Arc<Mutex<BuildProgress>>) {
        if self.last_update.elapsed() >= Duration::from_millis(100) {
            self.animation_frame = (self.animation_frame + 1) % 8;
            self.last_update = Instant::now();

            // Check if build completed
            let prog = progress.lock().unwrap();
            if prog.complete && self.messages.len() == 1 {
                drop(prog);
                self.finalize_messages(progress);
            }
        }
    }

    fn finalize_messages(&mut self, progress: &Arc<Mutex<BuildProgress>>) {
        let prog = progress.lock().unwrap();
        
        if let Some(error) = &prog.error {
            self.messages.push(format!("✗ Build failed: {}", error));
        } else if let Some((files, symbols, edges)) = prog.stats {
            self.messages.push("✓ Graph built".to_string());
            self.messages.push(format!("  Files:   {}", files));
            self.messages.push(format!("  Symbols: {}", symbols));
            self.messages.push(format!("  Edges:   {}", edges));
        }
    }
}

fn run_tui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut TUIState,
    progress: &Arc<Mutex<BuildProgress>>,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw_tui(f, state, progress))?;

        // Check for completion
        let prog = progress.lock().unwrap();
        let is_complete = prog.complete;
        let has_error = prog.error.is_some();
        drop(prog);

        if is_complete {
            // After completion, wait for user input or timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
            
            // Auto-exit after showing results for a bit
            if state.start_time.elapsed() > Duration::from_secs(10) && !has_error {
                return Ok(());
            }
        } else {
            // During build, just check for quit
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        return Ok(());
                    }
                }
            }
        }

        state.update(progress);
    }
}

fn draw_tui(f: &mut Frame, state: &TUIState, progress: &Arc<Mutex<BuildProgress>>) {
    let size = f.size();
    
    // Adaptive layout based on terminal size
    let (show_logo, show_info) = if size.height < 18 {
        (false, false)  // Minimal mode
    } else if size.height < 26 {
        (true, false)   // Medium mode
    } else {
        (true, true)    // Full mode
    };

    let mut constraints = vec![];
    if show_logo {
        constraints.push(Constraint::Length(3));  // Reduced from 5 to 3
    }
    constraints.push(Constraint::Min(6));
    if show_info {
        constraints.push(Constraint::Length(10));
    }
    constraints.push(Constraint::Length(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    let mut idx = 0;
    
    if show_logo {
        draw_header(f, chunks[idx], size.width);
        idx += 1;
    }
    
    draw_output(f, chunks[idx], state, progress, size.width);
    idx += 1;
    
    if show_info {
        draw_info(f, chunks[idx], progress);
        idx += 1;
    }
    
    draw_footer(f, chunks[idx], progress);
}

fn draw_header(f: &mut Frame, area: Rect, width: u16) {
    let logo = if width < 70 {
        // Compact version for narrow terminals (2 lines)
        vec![
            Line::from(vec![
                Span::styled(" ⚓ ", Style::default()
                    .fg(Color::Rgb(52, 211, 153))
                    .add_modifier(Modifier::BOLD)),
                Span::styled("ANCHOR", Style::default()
                    .fg(Color::Rgb(52, 211, 153))
                    .add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("LSP for AI • Zero Tokens", Style::default().fg(Color::Rgb(100, 116, 139))),
            ]),
        ]
    } else {
        // Wider terminal version (2 lines)
        vec![
            Line::from(vec![
                Span::styled(" ⚓ ", Style::default()
                    .fg(Color::Rgb(52, 211, 153))
                    .add_modifier(Modifier::BOLD)),
                Span::styled("ANCHOR", Style::default()
                    .fg(Color::Rgb(52, 211, 153))
                    .add_modifier(Modifier::BOLD)),
                Span::styled("  •  ", Style::default().fg(Color::Rgb(71, 85, 105))),
                Span::styled("LSP for AI Agents", Style::default().fg(Color::Rgb(100, 116, 139))),
                Span::styled("  •  ", Style::default().fg(Color::Rgb(71, 85, 105))),
                Span::styled("Zero Tokens", Style::default().fg(Color::Rgb(16, 185, 129)).add_modifier(Modifier::BOLD)),
            ]),
        ]
    };

    let header = Paragraph::new(logo).style(Style::default().bg(Color::Black));
    f.render_widget(header, area);
}

fn draw_output(f: &mut Frame, area: Rect, state: &TUIState, progress: &Arc<Mutex<BuildProgress>>, _width: u16) {
    let prog = progress.lock().unwrap();
    let is_complete = prog.complete;
    let has_error = prog.error.is_some();
    drop(prog);

    let title = if is_complete {
        if has_error {
            "  anchor build  ✗  "
        } else {
            "  anchor build  ✓  "
        }
    } else {
        "  anchor build  "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(52, 211, 153)))
        .style(Style::default().bg(Color::Rgb(10, 10, 15)))
        .title(Span::styled(title, Style::default()
            .fg(Color::Rgb(52, 211, 153))
            .add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Show all messages
    for msg in &state.messages {
        let color = if msg.starts_with('✓') {
            Color::Rgb(52, 211, 153)
        } else if msg.starts_with('✗') {
            Color::Rgb(239, 68, 68)
        } else if msg.starts_with("  ") {
            Color::Rgb(148, 163, 184)
        } else {
            Color::Rgb(203, 213, 225)
        };
        lines.push(Line::from(Span::styled(msg, Style::default().fg(color).add_modifier(Modifier::BOLD))));
    }

    // Show spinner if not complete
    if !is_complete {
        let spinner = match state.animation_frame % 8 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            3 => "⠸",
            4 => "⠼",
            5 => "⠴",
            6 => "⠦",
            _ => "⠧",
        };
        
        let elapsed = state.start_time.elapsed().as_secs();
        
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(spinner, Style::default().fg(Color::Rgb(234, 179, 8)).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("Parsing files and building graph", Style::default().fg(Color::Rgb(148, 163, 184))),
            Span::raw("   "),
            Span::styled(format!("{}s", elapsed), Style::default().fg(Color::Rgb(100, 116, 139))),
        ]));
    }

    let output = Paragraph::new(lines)
        .style(Style::default().bg(Color::Rgb(10, 10, 15)))
        .wrap(Wrap { trim: false });

    f.render_widget(output, inner);
}

fn draw_info(f: &mut Frame, area: Rect, progress: &Arc<Mutex<BuildProgress>>) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(52, 211, 153)))
        .style(Style::default().bg(Color::Rgb(10, 10, 15)))
        .title(Span::styled("  System Info  ", Style::default()
            .fg(Color::Rgb(52, 211, 153))
            .add_modifier(Modifier::BOLD)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let prog = progress.lock().unwrap();
    let (files, symbols, edges) = prog.stats.unwrap_or((0, 0, 0));
    drop(prog);

    let lines = vec![
        Line::from(vec![
            Span::styled("Engine      ", Style::default().fg(Color::Rgb(100, 116, 139))),
            Span::styled("Rust + Tree-sitter", Style::default().fg(Color::Rgb(226, 232, 240)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Storage     ", Style::default().fg(Color::Rgb(100, 116, 139))),
            Span::styled("In-Memory Graph (RAM)", Style::default().fg(Color::Rgb(192, 132, 252))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Token Cost  ", Style::default().fg(Color::Rgb(100, 116, 139))),
            Span::styled("0 tokens ", Style::default().fg(Color::Rgb(52, 211, 153)).add_modifier(Modifier::BOLD)),
            Span::styled("(structure is free!)", Style::default().fg(Color::Rgb(100, 116, 139))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Graph Stats ", Style::default().fg(Color::Rgb(100, 116, 139))),
            Span::styled(
                format!("{} files • {} symbols • {} edges", files, symbols, edges),
                Style::default().fg(Color::Rgb(226, 232, 240)),
            ),
        ]),
    ];

    let info = Paragraph::new(lines).style(Style::default().bg(Color::Rgb(10, 10, 15)));
    f.render_widget(info, inner);
}

fn draw_footer(f: &mut Frame, area: Rect, progress: &Arc<Mutex<BuildProgress>>) {
    let prog = progress.lock().unwrap();
    let is_complete = prog.complete;
    drop(prog);

    let text = if is_complete {
        vec![
            Span::styled("[", Style::default().fg(Color::Rgb(71, 85, 105))),
            Span::styled("Enter/Space", Style::default().fg(Color::Rgb(52, 211, 153)).add_modifier(Modifier::BOLD)),
            Span::styled("] Continue  ", Style::default().fg(Color::Rgb(148, 163, 184))),
            Span::styled("[", Style::default().fg(Color::Rgb(71, 85, 105))),
            Span::styled("q", Style::default().fg(Color::Rgb(52, 211, 153)).add_modifier(Modifier::BOLD)),
            Span::styled("] Quit", Style::default().fg(Color::Rgb(148, 163, 184))),
        ]
    } else {
        vec![
            Span::styled("⚡ ", Style::default().fg(Color::Rgb(234, 179, 8))),
            Span::styled("Building code graph... ", Style::default().fg(Color::Rgb(100, 116, 139))),
            Span::styled("[", Style::default().fg(Color::Rgb(71, 85, 105))),
            Span::styled("q", Style::default().fg(Color::Rgb(52, 211, 153))),
            Span::styled("] Quit", Style::default().fg(Color::Rgb(100, 116, 139))),
        ]
    };

    let footer = Paragraph::new(Line::from(text))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    f.render_widget(footer, area);
}
