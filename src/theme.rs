use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub name_lower: String,
    pub path: PathBuf,
    pub palette: [Color; 16],
    pub background: Color,
    pub foreground: Color,
    pub is_dark: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeFilter {
    All,
    Dark,
    Light,
}

impl ThemeFilter {
    fn next(self) -> Self {
        match self {
            ThemeFilter::All => ThemeFilter::Dark,
            ThemeFilter::Dark => ThemeFilter::Light,
            ThemeFilter::Light => ThemeFilter::All,
        }
    }

    fn label(self) -> &'static str {
        match self {
            ThemeFilter::All => "all",
            ThemeFilter::Dark => "dark",
            ThemeFilter::Light => "light",
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn acquire() -> Result<Self, color_eyre::Report> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = execute!(stdout, Show);
    }
}

fn parse_color(hex: &str) -> Option<Color> {
    let token = hex.split_whitespace().next()?;
    let token = token.strip_prefix('#').unwrap_or(token);
    let hex: String = token
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn named_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (255, 0, 0),
        Color::Green => (0, 255, 0),
        Color::Yellow => (255, 255, 0),
        Color::Blue => (0, 0, 255),
        Color::Magenta => (255, 0, 255),
        Color::Cyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        _ => (128, 128, 128),
    }
}

fn color_luminance(color: Color) -> f32 {
    let (r, g, b) = named_rgb(color);
    0.2126 * (r as f32 / 255.0) + 0.7152 * (g as f32 / 255.0) + 0.0722 * (b as f32 / 255.0)
}

fn parse_palette_entry(entry: &str) -> Option<(usize, Color)> {
    let (idx, value) = entry.split_once('=')?;
    let idx = idx.trim().parse::<usize>().ok()?;
    if idx >= 16 {
        return None;
    }
    let color = parse_color(value.trim())?;
    Some((idx, color))
}

fn parse_theme_file(path: &Path) -> Option<Theme> {
    let content = fs::read_to_string(path).ok()?;
    let mut palette: [Color; 16] = [Color::Black; 16];
    let mut background = Color::Black;
    let mut foreground = Color::White;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "palette" => {
                    for token in value
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                    {
                        if let Some((idx, color)) = parse_palette_entry(token) {
                            palette[idx] = color;
                        }
                    }
                }
                "background" => {
                    if let Some(color) = parse_color(value) {
                        background = color;
                    }
                }
                "foreground" => {
                    if let Some(color) = parse_color(value) {
                        foreground = color;
                    }
                }
                _ => {}
            }
        }
    }

    let is_dark = color_luminance(background) < 0.5;

    let name = path.file_stem()?.to_string_lossy().to_string();
    Some(Theme {
        name_lower: name.to_lowercase(),
        name,
        path: path.to_path_buf(),
        palette,
        background,
        foreground,
        is_dark,
    })
}

fn scan_theme_dir(dir: &Path, themes: &mut Vec<Theme>) {
    if !dir.exists() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        eprintln!(
            "warning: cannot read theme directory {}: permission denied",
            dir.display()
        );
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match parse_theme_file(&path) {
            Some(theme) => themes.push(theme),
            None => eprintln!("warning: failed to parse theme file: {}", path.display()),
        }
    }
}

fn discover_themes() -> Vec<Theme> {
    let mut themes = Vec::new();

    for dir_path in [
        "/Applications/Ghostty.app/Contents/Resources/ghostty/themes",
        "/opt/homebrew/share/ghostty/themes",
        "/usr/share/ghostty/themes",
    ] {
        scan_theme_dir(Path::new(dir_path), &mut themes);
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_dir = Path::new(&home).join(".config/ghostty/themes");
        scan_theme_dir(&user_dir, &mut themes);
    } else {
        eprintln!("warning: home is not set, skipping user theme directory");
    }

    themes.sort_by(|a, b| a.name_lower.cmp(&b.name_lower));
    themes
}

fn apply_theme(name: &str) -> Result<(), color_eyre::Report> {
    let home = std::env::var("HOME")?;
    let auto_dir = PathBuf::from(&home).join(".config/ghostty/auto");
    fs::create_dir_all(&auto_dir)?;

    let theme_file = auto_dir.join("theme.ghostty");
    fs::write(&theme_file, format!("theme = {}\n", name))?;

    Ok(())
}

fn read_active_theme_name() -> Result<Option<String>, color_eyre::Report> {
    let home = std::env::var("HOME")?;
    let theme_file = PathBuf::from(&home).join(".config/ghostty/auto/theme.ghostty");

    let content = match fs::read_to_string(&theme_file) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == "theme"
        {
            let name = value.trim().to_string();
            if !name.is_empty() {
                return Ok(Some(name));
            }
        }
    }

    Ok(None)
}

fn clear_active_theme() -> Result<(), color_eyre::Report> {
    let home = std::env::var("HOME")?;
    let theme_file = PathBuf::from(&home).join(".config/ghostty/auto/theme.ghostty");

    match fs::remove_file(theme_file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn ghostty_pids_from_ps() -> Result<Vec<String>, color_eyre::Report> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,command="])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| color_eyre::eyre::eyre!("ps command failed: {e}"))?;

    if !output.status.success() {
        return Err(color_eyre::eyre::eyre!(
            "ps command failed with exit code: {}",
            output.status
        ));
    }

    let mut pids = Vec::new();
    let stdout = std::str::from_utf8(&output.stdout)
        .map_err(|_| color_eyre::eyre::eyre!("ps output is not valid UTF-8"))?;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((pid, command)) = trimmed.split_once(char::is_whitespace) else {
            continue;
        };

        let cmd = command.trim();
        if cmd.contains("/Ghostty.app/Contents/MacOS/ghostty")
            || cmd == "ghostty"
            || cmd.ends_with("/ghostty")
        {
            pids.push(pid.to_string());
        }
    }

    Ok(pids)
}

fn send_sigusr2_to_pids(pids: &[String]) -> Result<bool, color_eyre::Report> {
    if pids.is_empty() {
        return Ok(false);
    }

    let status = Command::new("kill")
        .arg("-SIGUSR2")
        .args(pids)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    Ok(status.success())
}

fn reload_ghostty() -> Result<bool, color_eyre::Report> {
    let pids = ghostty_pids_from_ps()?;
    if pids.is_empty() {
        return Ok(false);
    }
    send_sigusr2_to_pids(&pids)
}

fn filter_themes(themes: &[Theme], filter: ThemeFilter, search: &str) -> Vec<usize> {
    let search_lower = search.to_lowercase();
    let mut indices: Vec<usize> = (0..themes.len()).collect();
    indices.retain(|&i| {
        let matches_filter = match filter {
            ThemeFilter::All => true,
            ThemeFilter::Dark => themes[i].is_dark,
            ThemeFilter::Light => !themes[i].is_dark,
        };
        let matches_search = if search_lower.is_empty() {
            true
        } else {
            themes[i].name_lower.contains(search_lower.as_str())
        };
        matches_filter && matches_search
    });
    indices
}

fn format_hex(color: Color) -> String {
    let (r, g, b) = named_rgb(color);
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn render_preview(frame: &mut ratatui::Frame, theme: &Theme, area: Rect) {
    if area.width < 40 || area.height < 12 {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(theme.name.as_str()).bold(),
                Line::from(if theme.is_dark {
                    "dark theme"
                } else {
                    "light theme"
                }),
                Line::from(format!("bg {}", format_hex(theme.background))),
                Line::from(format!("fg {}", format_hex(theme.foreground))),
            ])
            .block(Block::default().borders(Borders::ALL).title(" preview ")),
            area,
        );
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);

    frame.render_widget(Paragraph::new(theme.name.as_str()).bold(), chunks[0]);

    let indicator = if theme.is_dark {
        "Dark Theme"
    } else {
        "Light Theme"
    };
    frame.render_widget(Paragraph::new(indicator).dim(), chunks[1]);

    frame.render_widget(Paragraph::new("System Palette").bold(), chunks[3]);

    for row in 0..2 {
        let mut spans = Vec::new();
        for col in 0..8 {
            let idx = row * 8 + col;
            let color = theme.palette[idx];

            spans.push(Span::styled(
                format!(" {idx:02} "),
                Style::default().bg(color).fg(Color::Black).bold(),
            ));
            spans.push(Span::from(" "));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[4 + row]);
    }

    let meta_lines = vec![
        Line::from(vec![
            Span::from("Background: ").bold(),
            Span::styled(" █ ", Style::default().fg(theme.background)),
            Span::from(format!("({})", format_hex(theme.background))),
        ]),
        Line::from(vec![
            Span::from("Foreground: ").bold(),
            Span::styled(" █ ", Style::default().fg(theme.foreground)),
            Span::from(format!("({})", format_hex(theme.foreground))),
        ]),
    ];
    frame.render_widget(Paragraph::new(meta_lines), chunks[7]);

    let fallback =
        |base: Color, light: Color| -> Color { if base != Color::Black { base } else { light } };
    let kw = Style::default().fg(fallback(theme.palette[1], theme.palette[9]));
    let fn_name = Style::default().fg(fallback(theme.palette[4], theme.palette[12]));
    let type_st = Style::default().fg(fallback(theme.palette[3], theme.palette[11]));
    let text = Style::default().fg(theme.foreground);
    let literal = Style::default().fg(fallback(theme.palette[2], theme.palette[10]));
    let macro_st = Style::default().fg(fallback(theme.palette[5], theme.palette[13]));
    let comment = Style::default()
        .fg(theme.palette[8])
        .add_modifier(Modifier::ITALIC);
    let num_col = Style::default().fg(theme.palette[8]);

    let rust_code = vec![
        Line::from(vec![
            Span::styled(" 1 | ", num_col),
            Span::styled("let probe = reqwest::Client::new();", comment),
        ]),
        Line::from(vec![
            Span::styled(" 2 | ", num_col),
            Span::styled("pub fn ", kw),
            Span::styled("verify_health", fn_name),
            Span::styled("(", text),
            Span::styled("port", text),
            Span::styled(": ", text),
            Span::styled("u16", type_st),
            Span::styled(") -> ", text),
            Span::styled("bool ", type_st),
            Span::styled("{", text),
        ]),
        Line::from(vec![
            Span::styled(" 3 | ", num_col),
            Span::styled("    let ", kw),
            Span::styled("addr = format!(", text),
            Span::styled("\"http://127.0.0.1:{}\"", literal),
            Span::styled(", port);", text),
        ]),
        Line::from(vec![
            Span::styled(" 4 | ", num_col),
            Span::styled("    println!(", macro_st),
            Span::styled("\"Connecting to workspace engine...\"", literal),
            Span::styled(");", text),
        ]),
        Line::from(vec![
            Span::styled(" 5 | ", num_col),
            Span::styled("    reqwest::Client::new()", text),
        ]),
        Line::from(vec![
            Span::styled(" 6 | ", num_col),
            Span::styled("        .get(&addr)", text),
        ]),
        Line::from(vec![
            Span::styled(" 7 | ", num_col),
            Span::styled("        .send()", text),
        ]),
        Line::from(vec![
            Span::styled(" 8 | ", num_col),
            Span::styled("        .is_ok()", text),
        ]),
        Line::from(vec![
            Span::styled(" 9 | ", num_col),
            Span::styled("}", text),
        ]),
    ];

    let editor_snippet = Paragraph::new(rust_code).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Live Editor Preview (Rust) ")
            .border_style(Style::default().dim())
            .bg(theme.background),
    );

    frame.render_widget(editor_snippet, chunks[9]);
}

fn restore_previous_theme(
    preview_applied: bool,
    original_theme_name: &Option<String>,
) -> Result<bool, color_eyre::Report> {
    if !preview_applied {
        return Ok(false);
    }
    if let Some(name) = original_theme_name {
        apply_theme(name)?;
    } else {
        clear_active_theme()?;
    }
    reload_ghostty()?;
    Ok(true)
}

pub fn run(list_only: bool) -> Result<(), color_eyre::Report> {
    const LIVE_PREVIEW_DEBOUNCE: Duration = Duration::from_millis(120);

    let themes = discover_themes();
    if themes.is_empty() {
        eprintln!("warning: no themes found");
        return Ok(());
    }

    let is_tty = std::io::stdout().is_terminal();

    if list_only || !is_tty {
        eprintln!("info: listing discovered themes");
        for t in &themes {
            let filter = if t.is_dark { "dark" } else { "light" };
            println!("  {:<30} [{}] {}", t.name, filter, t.path.display());
        }
        return Ok(());
    }

    let original_theme_name = read_active_theme_name()?;
    let mut preview_applied = false;

    let result = {
        let _terminal_guard = TerminalGuard::acquire()?;
        (|| -> Result<(Option<Theme>, bool), color_eyre::Report> {
            let stdout = std::io::stdout();
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;

            let mut filter = ThemeFilter::All;
            let mut search = String::new();
            let mut search_active = false;
            let mut state = ListState::default().with_selected(Some(0));
            let mut filtered = filter_themes(&themes, filter, &search);
            let mut last_preview_at = Instant::now() - LIVE_PREVIEW_DEBOUNCE;
            let mut previewed_theme: Option<String> = None;
            let mut preview_warning: Option<String> = None;

            loop {
                let current_idx = state.selected().unwrap_or(0);
                let filtered_len = filtered.len();
                if current_idx >= filtered_len {
                    state.select(Some(filtered_len.saturating_sub(1)));
                }

                if let Some(&orig_idx) = filtered.get(current_idx) {
                    let selected_theme = &themes[orig_idx].name;
                    let needs_preview = previewed_theme.as_deref() != Some(selected_theme.as_str());
                    if needs_preview && last_preview_at.elapsed() >= LIVE_PREVIEW_DEBOUNCE {
                        last_preview_at = Instant::now();
                        match apply_theme(selected_theme) {
                            Ok(()) => {
                                preview_applied = true;
                                previewed_theme = Some(selected_theme.clone());
                                match reload_ghostty() {
                                    Ok(true) => preview_warning = None,
                                    Ok(false) => {
                                        preview_warning =
                                            Some("reload signal not acknowledged".to_string());
                                    }
                                    Err(e) => {
                                        preview_warning = Some(format!("reload error: {e}"));
                                    }
                                }
                            }
                            Err(e) => {
                                preview_warning = Some(format!("preview apply error: {e}"));
                            }
                        }
                    }
                }

                terminal.draw(|frame| {
                    let area = frame.area();
                    let chunks = Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Fill(1),
                        Constraint::Length(1),
                    ])
                    .split(area);

                    let title = Line::from_iter([
                        Span::from("Select Theme").bold(),
                        Span::from(
                            " (j/k navigate, f filter, / search, Enter confirm, Esc cancel)",
                        ),
                    ]);
                    frame.render_widget(Paragraph::new(title).centered(), chunks[0]);

                    let filter_text = format!("[Filter: {}]", filter.label());
                    frame.render_widget(Paragraph::new(filter_text).centered().dim(), chunks[1]);

                    let search_suffix = if search_active { "|" } else { "" };
                    let search_text = if search.is_empty() {
                        if search_active {
                            "Search: |".to_string()
                        } else {
                            "Search: (press / to start, Backspace to clear)".to_string()
                        }
                    } else {
                        format!("Search: {}{}", search, search_suffix)
                    };
                    frame.render_widget(Paragraph::new(search_text).centered().dim(), chunks[2]);

                    let main_layout = Layout::horizontal([
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ])
                    .split(chunks[3]);

                    let list_items: Vec<ListItem> = if filtered.is_empty() {
                        vec![ListItem::new("  (no matching themes)")]
                    } else {
                        filtered
                            .iter()
                            .enumerate()
                            .map(|(i, &orig_idx)| {
                                let text = if i == current_idx {
                                    format!("> {}", themes[orig_idx].name)
                                } else {
                                    format!("  {}", themes[orig_idx].name)
                                };
                                ListItem::new(text)
                            })
                            .collect()
                    };

                    let list = List::new(list_items)
                        .block(
                            Block::default()
                                .borders(Borders::RIGHT)
                                .border_style(Style::default().dim()),
                        )
                        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                        .highlight_spacing(HighlightSpacing::Always);

                    frame.render_stateful_widget(list, main_layout[0], &mut state);

                    if let Some(&orig_idx) = filtered.get(current_idx) {
                        render_preview(frame, &themes[orig_idx], main_layout[1]);
                    } else {
                        frame.render_widget(
                            Paragraph::new("No themes match the current filter/search").centered(),
                            main_layout[1],
                        );
                    }

                    let footer = if let Some(err) = &preview_warning {
                        Line::from(format!("Live preview warning: {err}"))
                    } else {
                        Line::from("Live preview on (debounced). Enter keep, q/Esc cancel")
                    };
                    frame.render_widget(Paragraph::new(footer).centered().dim(), chunks[4]);
                })?;

                if event::poll(Duration::from_millis(16))?
                    && let Event::Key(key) = event::read()?
                {
                    if search_active {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                search_active = false;
                            }
                            KeyCode::Backspace => {
                                search.pop();
                            }
                            KeyCode::Char(c) => {
                                search.push(c);
                            }
                            _ => {}
                        }
                        filtered = filter_themes(&themes, filter, &search);
                        continue;
                    }

                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => state.select_next(),
                        KeyCode::Char('k') | KeyCode::Up => state.select_previous(),
                        KeyCode::Char('g') | KeyCode::Home => state.select_first(),
                        KeyCode::Char('G') | KeyCode::End => state.select_last(),
                        KeyCode::Char('f') => {
                            filter = filter.next();
                        }
                        KeyCode::Char('/') => {
                            search_active = true;
                        }
                        KeyCode::Backspace => {
                            search.clear();
                        }
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(idx) = state.selected()
                                && let Some(&orig_idx) = filtered.get(idx)
                            {
                                return Ok((Some(themes[orig_idx].clone()), preview_applied));
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Esc => return Ok((None, preview_applied)),
                        _ => {}
                    }

                    filtered = filter_themes(&themes, filter, &search);
                }
            }
        })()
    };
    eprintln!();

    match result {
        Ok((Some(theme), _)) => {
            apply_theme(&theme.name)?;
            eprintln!("info: applied theme {}", theme.name);
            match reload_ghostty() {
                Ok(true) => {
                    eprintln!("info: signaled ghostty to reload configuration");
                }
                Ok(false) => {
                    eprintln!("warning: ghostty reload signal was not acknowledged");
                    eprintln!(
                        "info: try 'kill -SIGUSR2 $(pgrep -f Ghostty.app/Contents/MacOS/ghostty)' manually"
                    );
                    eprintln!("info: on macOS, reload may be ignored while ghostty is focused");
                }
                Err(e) => {
                    eprintln!("warning: could not execute ghostty reload signal: {e}");
                    eprintln!(
                        "info: try 'kill -SIGUSR2 $(pgrep -f Ghostty.app/Contents/MacOS/ghostty)' manually"
                    );
                }
            }
        }
        Ok((None, preview_applied)) => {
            match restore_previous_theme(preview_applied, &original_theme_name) {
                Ok(true) => eprintln!(
                    "info: cancelled and {}",
                    if original_theme_name.is_some() {
                        "restored previous theme"
                    } else {
                        "cleared preview theme"
                    }
                ),
                Ok(false) => eprintln!("info: cancelled"),
                Err(e) => eprintln!("warning: cancelled but failed to restore theme: {e}"),
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            if let Err(re) = restore_previous_theme(preview_applied, &original_theme_name) {
                eprintln!("warning: failed to restore previous theme: {re}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempThemeFile(PathBuf);

    impl TempThemeFile {
        fn new(contents: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "gsty-test-{}-{}.ghostty",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock before unix epoch")
                    .as_nanos()
            ));
            fs::write(&path, contents).expect("failed to write temp theme file");
            Self(path)
        }
    }

    impl Drop for TempThemeFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn parse_color_valid() {
        assert_eq!(parse_color("#ffffff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_color("000000"), Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn parse_color_with_trailing_comments() {
        assert_eq!(
            parse_color("#ff0000 # Red comment"),
            Some(Color::Rgb(255, 0, 0))
        );
    }

    #[test]
    fn parse_color_invalid() {
        assert!(parse_color("#fff").is_none());
        assert!(parse_color("invalid_hex").is_none());
    }

    #[test]
    fn color_luminance_orders_light_above_dark() {
        assert!(color_luminance(Color::White) > color_luminance(Color::Black));
    }

    #[test]
    fn parse_theme_file_handles_inline_comments() {
        let file = TempThemeFile::new(
            "background = #111111 # dark\nforeground = #eeeeee # light\npalette = 1=#ff0000 # red\n",
        );

        let theme = parse_theme_file(&file.0).expect("theme should parse");
        assert_eq!(theme.background, Color::Rgb(0x11, 0x11, 0x11));
        assert_eq!(theme.foreground, Color::Rgb(0xee, 0xee, 0xee));
        assert_eq!(theme.palette[1], Color::Rgb(0xff, 0x00, 0x00));
        assert!(theme.is_dark);
    }

    #[test]
    fn parse_theme_file_accepts_multiple_palette_entries() {
        let file =
            TempThemeFile::new("palette = 0=#000000, 1=#111111 2=#222222\nbackground = #fafafa\n");

        let theme = parse_theme_file(&file.0).expect("theme should parse");
        assert_eq!(theme.palette[0], Color::Rgb(0x00, 0x00, 0x00));
        assert_eq!(theme.palette[1], Color::Rgb(0x11, 0x11, 0x11));
        assert_eq!(theme.palette[2], Color::Rgb(0x22, 0x22, 0x22));
        assert!(!theme.is_dark);
    }

    #[test]
    fn parse_theme_file_handles_inline_comments_without_space() {
        let file = TempThemeFile::new(
            "background = #111111#dark\nforeground = #eeeeee#light\npalette = 1=#ff0000#red\n",
        );

        let theme = parse_theme_file(&file.0).expect("theme should parse");
        assert_eq!(theme.background, Color::Rgb(0x11, 0x11, 0x11));
        assert_eq!(theme.foreground, Color::Rgb(0xee, 0xee, 0xee));
        assert_eq!(theme.palette[1], Color::Rgb(0xff, 0x00, 0x00));
        assert!(theme.is_dark);
    }
}
