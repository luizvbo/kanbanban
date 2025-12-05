use crate::app::{App, OverviewTab};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{BarChart, Block, Borders, Row, Table, Tabs},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    let titles = vec!["Charts (1)", "Audit Logs (2)"];
    let index = match app.overview_state.tab {
        OverviewTab::Charts => 0,
        OverviewTab::Logs => 1,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(index)
        .highlight_style(Style::default().fg(Color::Cyan));
    f.render_widget(tabs, chunks[0]);

    match app.overview_state.tab {
        OverviewTab::Charts => render_charts(f, app, chunks[1]),
        OverviewTab::Logs => render_logs(f, app, chunks[1]),
    }
}

fn render_charts(f: &mut Frame, app: &App, area: Rect) {
    if app.chart_data.is_empty() {
        return;
    }

    // Convert String keys to &str for BarChart
    let data: Vec<(&str, u64)> = app
        .chart_data
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();

    let barchart = BarChart::default()
        .block(
            Block::default()
                .title("Tasks by Category")
                .borders(Borders::ALL),
        )
        .data(&data)
        .bar_width(15)
        .bar_gap(2)
        .bar_style(Style::default().fg(Color::Yellow))
        .value_style(Style::default().fg(Color::Black).bg(Color::Yellow));

    f.render_widget(barchart, area);
}

fn render_logs(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec!["Time", "Type", "Object", "Description"]).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    let rows: Vec<Row> = app
        .overview_state
        .logs
        .iter()
        .map(|log| {
            Row::new(vec![
                log.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                log.event_type.clone(),
                log.object_type.clone(),
                log.description.clone(),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(16),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Min(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Audit Logs "))
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_stateful_widget(table, area, &mut app.overview_state.log_state.clone());
}
