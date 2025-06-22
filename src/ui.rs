use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::app::AppState;

pub fn render_ui(f: &mut Frame, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),                   // Search box
            if app.loading { Constraint::Length(1) } else { Constraint::Length(0) }, // Loading indicator
            Constraint::Min(1),                      // Results
            Constraint::Length(1),                   // Status bar
        ])
        .split(f.size());

    let (search_chunk, loading_chunk, main_body_chunk, _status_chunk) = 
        (chunks[0], chunks[1], chunks[2], chunks[3]);

    // Render search input
    let input_style = if app.input.is_empty() {
        Style::default().fg(Color::Gray)
    } else {
        Style::default().fg(Color::White)
    };
    let input_text = if app.input.is_empty() {
        "Type to search files..."
    } else {
        &app.input
    };
    let input = Paragraph::new(input_text)
        .style(input_style)
        .block(
            Block::default()
                .title("🔍 Fuzzy File Finder")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        );
    f.render_widget(input, search_chunk);

    // Render loading
    if app.loading {
        let loading = Paragraph::new("Loading files...")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(loading, loading_chunk);
    }

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(main_body_chunk);

    let (results_chunk, preview_chunk) = (main_chunks[0], main_chunks[1]);

    // Build results list
    let items: Vec<ListItem> = app
        .matches
        .iter()
        .map(|file| {
            let filename = std::path::Path::new(&file.path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let directory = std::path::Path::new(&file.path)
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            let content = Line::from(vec![
                Span::styled(format!("{} ", filename), Style::default().fg(Color::Green)),
                Span::styled(format!("({})", directory), Style::default().fg(Color::Gray)),
            ]);
            ListItem::new(content)
        })
        .collect();

    let results_title = format!(
        "Results ({}/{})", 
        app.matches.len(), 
        app.all_files.len()
    );
    let list = List::new(items)
        .block(Block::default().title(results_title).borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    // Update selection in list_state
    app.list_state.select(Some(app.selected));
    f.render_stateful_widget(list, results_chunk, &mut app.list_state);

    // Render file preview
    let preview = Paragraph::new(app.preview_content.clone())
        .style(Style::default().fg(Color::White))
        .block(Block::default().title("Preview").borders(Borders::ALL));
    f.render_widget(preview, preview_chunk);

    // Render status bar
    let status = if app.loading {
        "Loading... | ESC: Quit | ↑↓: Navigate".to_string()
    } else {
        format!(
            "Files: {} | Matches: {} | ESC: Quit | Enter: Open | ↑↓: Navigate",
            app.all_files.len(),
            app.matches.len()
        )
    };
    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center);
    f.render_widget(status_bar, chunks[chunks.len() - 1]);
}