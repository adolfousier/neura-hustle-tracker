use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Borders, List, ListItem, Paragraph};
use ratatui::style::{Color, Style};
use ratatui::Frame;
use chrono::Local;
use std::collections::BTreeMap;
use crate::ui::app::{App, AppState, InputAction, ViewMode};

pub fn draw(app: &App, f: &mut Frame) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(size);

    // Status bar with Shift+C indicator
    let status = match &app.state {
        AppState::Dashboard { .. } => {
            if let Some(session) = &app.current_session {
                let duration = Local::now().signed_duration_since(session.start_time).num_seconds();
                let display_name = app.manual_app_name.as_ref().unwrap_or(&session.app_name);
                format!("Tracking: {} for {}s | [Shift+C] Commands | [h] History", display_name, duration)
            } else {
                format!("Not tracking - Current app: {} | [Shift+C] Commands | [h] History", app.current_app)
            }
        }
        AppState::ViewingLogs => "Viewing Logs - Press any key to return".to_string(),
        AppState::SelectingApp { .. } => "Rename Mode - Use arrow keys to select an app".to_string(),
        AppState::SelectingCategory { .. } => "Category Mode - Use arrow keys to select an app".to_string(),
        AppState::CategoryMenu { .. } => "Category Mode - Use arrow keys to select a category".to_string(),
        AppState::Input { action, .. } => {
            match action {
                InputAction::RenameApp { .. } => "Rename Mode - Enter new name for the app".to_string(),
                InputAction::CreateCategory { .. } => "Category Mode - Enter custom category name (e.g., 🎮 Gaming)".to_string(),
            }
        }
        AppState::CommandsPopup => "Commands Menu - Press key to execute or Esc to close".to_string(),
        AppState::HistoryPopup { .. } => "Session History - Use ↑/↓/PgUp/PgDn to scroll, Esc to close".to_string(),
        AppState::BreakdownDashboard { .. } => "📊 Activity Breakdown Dashboard - [↑/↓/PgUp/PgDn] Scroll | [Esc] Close".to_string(),
    };

    let status_widget = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status_widget, chunks[0]);

    // Main content area
    match &app.state {
        AppState::ViewingLogs => {
            let log_items: Vec<ListItem> = app
                .logs
                .iter()
                .rev()
                .take(20)
                .map(|log| ListItem::new(Line::from(log.clone())))
                .collect();
            let log_list = List::new(log_items)
                .block(Block::default().borders(Borders::ALL).title("Logs"));
            f.render_widget(log_list, chunks[1]);
        }

        AppState::SelectingApp { selected_index } => {
            // Full-screen app selection view
            let max_items = (chunks[1].height.saturating_sub(2) as usize).min(20).max(5);
            let usage_items: Vec<ListItem> = app
                .usage
                .iter()
                .enumerate()
                .take(max_items)
                .map(|(i, (app, duration))| {
                    let hours = duration / 3600;
                    let minutes = (duration % 3600) / 60;
                    let prefix = if i == *selected_index { "→ " } else { "  " };

                    let time_display = if hours > 0 {
                        format!("{}h {}m", hours, minutes)
                    } else {
                        format!("{}m", minutes)
                    };

                    let clean_app = App::clean_app_name(app);
                    let display = format!("{}{:<30} {}", prefix, clean_app, time_display);

                    let style = if i == *selected_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };

                    ListItem::new(Line::from(display)).style(style)
                })
                .collect();

            let usage_list = List::new(usage_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("📝 Select App to Rename (↑/↓ to navigate, Enter to select, Esc to cancel)"));
            f.render_widget(usage_list, chunks[1]);
        }

        AppState::SelectingCategory { selected_index } => {
            // Full-screen app selection view for category assignment
            let max_items = (chunks[1].height.saturating_sub(2) as usize).min(20).max(5);
            let usage_items: Vec<ListItem> = app
                .usage
                .iter()
                .enumerate()
                .take(max_items)
                .map(|(i, (app_name, duration))| {
                    let hours = duration / 3600;
                    let minutes = (duration % 3600) / 60;
                    let prefix = if i == *selected_index { "→ " } else { "  " };

                    let time_display = if hours > 0 {
                        format!("{}h {}m", hours, minutes)
                    } else {
                        format!("{}m", minutes)
                    };

                    let clean_app = App::clean_app_name(app_name);
                    let (category, color) = app.get_app_category(app_name);
                    let display = format!("{}{:<30} {} [{}]", prefix, clean_app, time_display, category);

                    let style = if i == *selected_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(color)
                    };

                    ListItem::new(Line::from(display)).style(style)
                })
                .collect();

            let usage_list = List::new(usage_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("🏷️  Select App to Change Category (↑/↓ to navigate, Enter to select, Esc to cancel)"));
            f.render_widget(usage_list, chunks[1]);
        }

        AppState::CategoryMenu { app_name, selected_index } => {
            // Show category selection menu
            let categories = App::get_category_options();
            let category_items: Vec<ListItem> = categories
                .iter()
                .enumerate()
                .map(|(i, category)| {
                    let prefix = if i == *selected_index { "→ " } else { "  " };
                    let display = format!("{}{}", prefix, category);

                    let style = if i == *selected_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        // Apply color based on category
                        let color = App::category_from_string(category).1;
                        Style::default().fg(color)
                    };

                    ListItem::new(Line::from(display)).style(style)
                })
                .collect();

            let clean_app = App::clean_app_name(app_name);
            let title = format!("🏷️  Select Category for '{}' (↑/↓ to navigate, Enter to select, Esc to cancel)", clean_app);
            let category_list = List::new(category_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(title));
            f.render_widget(category_list, chunks[1]);
        }

        AppState::Input { prompt, buffer, action } => {
            // Full-screen input view with centered input box
            let input_area = App::centered_rect(70, 30, chunks[1]);

            // Clear background
            f.render_widget(ratatui::widgets::Clear, input_area);

            // Determine title based on action
            let title = match action {
                InputAction::RenameApp { .. } => "✏️  Rename App",
                InputAction::CreateCategory { .. } => "🏷️  Create Custom Category",
            };

            // Create input text with cursor
            let input_text = vec![
                Line::from(""),
                Line::from(prompt.clone()).style(Style::default().fg(Color::Cyan)),
                Line::from(""),
                Line::from(vec![
                    ratatui::text::Span::styled("  ", Style::default()),
                    ratatui::text::Span::styled(buffer.clone(), Style::default().fg(Color::White)),
                    ratatui::text::Span::styled("█", Style::default().fg(Color::Yellow)),
                ]),
                Line::from(""),
                Line::from("  Press Enter to confirm, Esc to cancel").style(Style::default().fg(Color::Gray)),
            ];

            let input_widget = Paragraph::new(input_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(Style::default().bg(Color::Black)));

            f.render_widget(input_widget, input_area);
        }

        AppState::Dashboard { view_mode } => {
            app.draw_dashboard(f, chunks[1], view_mode);
        }

        AppState::CommandsPopup => {
            // Show dashboard in background
            app.draw_dashboard(f, chunks[1], &app.current_view_mode);

            // Draw popup overlay
            let popup_area = App::centered_rect(60, 50, size);
            f.render_widget(ratatui::widgets::Clear, popup_area);

            let commands_text = vec![
                Line::from(""),
                Line::from("  [Tab]  Switch View (Daily/Weekly/Monthly)"),
                Line::from("  [h]    View session history (scrollable popup)"),
                Line::from("  [b]    View activity breakdowns (scrollable popup)"),
                Line::from("  [r]    Rename app/tab"),
                Line::from("  [c]    Change app category"),
                Line::from("  [l]    View logs"),
                Line::from("  [q]    Quit application (auto-saves)"),
                Line::from(""),
                Line::from("  Press Esc to close this menu"),
            ];

            let popup = Paragraph::new(commands_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("📋 Commands Menu")
                    .style(Style::default().bg(Color::Black)));
            f.render_widget(popup, popup_area);
        }

        AppState::HistoryPopup { view_mode, scroll_position } => {
            // Show dashboard in background
            app.draw_dashboard(f, chunks[1], view_mode);

            // Draw popup overlay
            let popup_area = App::centered_rect(80, 70, size);
            f.render_widget(ratatui::widgets::Clear, popup_area);

            // Calculate how many items can fit in the popup
            let max_visible_items = (popup_area.height.saturating_sub(4) as usize).max(10);

            // Create history list items
            let mut history_items: Vec<ListItem> = Vec::new();

            // Get the visible slice of history based on scroll position
            let start_idx = *scroll_position;
            let end_idx = (start_idx + max_visible_items).min(app.current_history.len());

            for (idx, session) in app.current_history[start_idx..end_idx].iter().enumerate() {
                let minutes = session.duration / 60;
                let time = session.start_time.format("%Y-%m-%d %H:%M");

                // Create display name with window name if available
                let clean_app = App::clean_app_name(&session.app_name);
                let display_name = if let Some(window_name) = &session.window_name {
                    format!("{} ({})", clean_app, window_name)
                } else {
                    clean_app
                };

                let display = format!("{}  {} - {}m", time, display_name, minutes);
                let style = if idx == 0 && start_idx == 0 {
                    Style::default().fg(Color::Yellow)  // Highlight first (most recent)
                } else {
                    Style::default()
                };

                history_items.push(ListItem::new(Line::from(display)).style(style));
            }

            // Add indicator if there are more items to scroll
            let scroll_indicator = if app.current_history.len() > max_visible_items {
                format!(" (Showing {}-{} of {} sessions)", start_idx + 1, end_idx, app.current_history.len())
            } else {
                format!(" ({} sessions)", app.current_history.len())
            };

            let history_list = List::new(history_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("📜 Session History{}", scroll_indicator))
                    .style(Style::default().bg(Color::Black)));
            f.render_widget(history_list, popup_area);
        }

        AppState::BreakdownDashboard { view_mode, scroll_position } => {
            // Show dashboard in background
            app.draw_dashboard(f, chunks[1], view_mode);

            // Draw popup overlay (90% width, 85% height)
            let popup_area = App::centered_rect(90, 85, size);
            f.render_widget(ratatui::widgets::Clear, popup_area);

            // Main popup container
            let popup_block = Block::default()
                .borders(Borders::ALL)
                .title("📊 Activity Breakdown Dashboard")
                .style(Style::default().bg(Color::Black));
            f.render_widget(popup_block, popup_area);

            // Inner area for grid layout
            let inner_area = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

            // Create grid layout: 2 columns x 3 rows
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                ].as_ref())
                .split(inner_area);

            // Row 1: Categories | Browser Services
            let row1_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(rows[0]);

            // Row 2: Projects | Files
            let row2_cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(rows[1]);

            // Row 3: Terminal Sessions (full width)
            let row3_area = rows[2];

            // Render each breakdown section
            draw_breakdown_section(f, row1_cols[0], "📦 Categories", &app.category_breakdown, Color::Magenta, true);
            draw_breakdown_section(f, row1_cols[1], "🌐 Browser Services", &app.browser_breakdown, Color::Blue, false);
            draw_breakdown_section(f, row2_cols[0], "📁 Projects", &app.project_breakdown, Color::Yellow, false);

            // Files breakdown with language info
            app.draw_file_breakdown_section(f, row2_cols[1], *scroll_position);

            draw_breakdown_section(f, row3_area, "💻 Terminal Sessions", &app.terminal_breakdown, Color::Green, false);
        }
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw_dashboard(app: &App, f: &mut Frame, area: Rect, view_mode: &ViewMode) {
    // Adaptive layout based on terminal size
    let use_vertical_layout = area.width < 120 || area.height < 30;

    let (data, title) = match view_mode {
        ViewMode::Daily => (&app.daily_usage, "📊 Daily Usage"),
        ViewMode::Weekly => (&app.weekly_usage, "📊 Weekly Usage (7 days)"),
        ViewMode::Monthly => (&app.monthly_usage, "📊 Monthly Usage (30 days)"),
    };

    // Create bar chart data - limit based on space
    let max_bars = if area.width < 80 { 5 } else if area.width < 120 { 8 } else { 10 };
    let bar_data: Vec<(&str, u64)> = data
        .iter()
        .take(max_bars)
        .map(|(app, duration)| {
            let minutes = (duration / 60) as u64;
            (app.as_str(), minutes)
        })
        .collect();

    if use_vertical_layout {
        // VERTICAL LAYOUT for small terminals
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),  // Bar chart
                Constraint::Min(8),   // Timeline
                Constraint::Min(8),   // AFK
                Constraint::Min(8),   // Stats
                Constraint::Min(10),  // History
                Constraint::Min(8),   // Categories
            ].as_ref())
            .split(area);

        app.draw_bar_chart(f, chunks[0], title, &bar_data);
        app.draw_timeline(f, chunks[1]);
        app.draw_afk(f, chunks[2]);
        draw_stats(f, chunks[3], data);
        app.draw_history(f, chunks[4]);
        app.draw_pie_chart(f, chunks[5], data);
    } else {
        // HORIZONTAL LAYOUT for larger terminals (50/50 split)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ].as_ref())
            .split(area);

        // LEFT SIDE: Bar Chart + Timeline/AFK + Stats
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),   // Bar chart
                Constraint::Percentage(30),   // Timeline + AFK
                Constraint::Percentage(30),   // Detailed stats
            ].as_ref())
            .split(main_chunks[0]);

        // RIGHT SIDE: Session History + Pie Chart
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ].as_ref())
            .split(main_chunks[1]);

        app.draw_bar_chart(f, left_chunks[0], title, &bar_data);
        let timeline_afk_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(left_chunks[1]);
        app.draw_timeline(f, timeline_afk_chunks[0]);
        app.draw_afk(f, timeline_afk_chunks[1]);
        draw_stats(f, left_chunks[2], data);
        app.draw_history(f, right_chunks[0]);
        app.draw_pie_chart(f, right_chunks[1], data);
    }
}

pub fn draw_bar_chart(app: &App, f: &mut Frame, area: Rect, title: &str, bar_data: &[(&str, u64)]) {
    if bar_data.is_empty() {
        let empty_msg = Paragraph::new("No data available yet. Start tracking!")
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(empty_msg, area);
    } else {
        // Adaptive bar width based on terminal width
        let bar_width = if area.width < 60 { 3 } else if area.width < 100 { 5 } else { 6 };
        let bar_gap = if area.width < 60 { 0 } else { 1 };

        // Find max value in minutes
        let max_minutes = bar_data.iter().map(|(_, v)| *v).max().unwrap_or(0);

        // Calculate scale: minimum 8h (480 min), or max_value + 2h (120 min)
        // This ensures bars never reach the top
        let scale_minutes = if max_minutes <= 480 {
            480  // 8h default for regular workday
        } else {
            // Round up to next hour and add 2h buffer
            ((max_minutes / 60) + 3) * 60
        };

        let scale_hours = scale_minutes / 60;

        // Create bars with category-based colors and hour labels
        let bars: Vec<Bar> = bar_data
            .iter()
            .map(|(app_name, value_minutes)| {
                let (_, color) = app.get_app_category(app_name);
                let hours = value_minutes / 60;
                let mins = value_minutes % 60;

                // Format label: show hours only, or hours + minutes
                let value_label = if mins == 0 {
                    format!("{}h", hours)
                } else if hours == 0 {
                    format!("{}m", mins)
                } else {
                    format!("{}h{}m", hours, mins)
                };

                let clean_app = App::clean_app_name(app_name);
                Bar::default()
                    .value(*value_minutes)
                    .label(Line::from(clean_app))
                    .text_value(value_label)
                    .style(Style::default().fg(color))
                    .value_style(Style::default().fg(Color::White))
            })
            .collect();

        let chart_title = format!("{} (scale: 0-{}h)", title, scale_hours);

        let barchart = BarChart::default()
            .block(Block::default().borders(Borders::ALL).title(chart_title))
            .bar_width(bar_width)
            .bar_gap(bar_gap)
            .max(scale_minutes)  // Set max scale directly instead of padding bar
            .data(BarGroup::default().bars(&bars));
        f.render_widget(barchart, area);
    }
}

pub fn draw_stats(f: &mut Frame, area: Rect, data: &[(String, i64)]) {
    // Adaptive number of items based on available height
    let max_items = (area.height.saturating_sub(3) as usize).min(15).max(3);

    let mut stats_items: Vec<ListItem> = Vec::new();

    // Add top margin
    stats_items.push(ListItem::new(Line::from("")));

    stats_items.extend(data
        .iter()
        .take(max_items)
        .map(|(app, duration)| {
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;

            // Clean and truncate app name if terminal is narrow
            let clean_app = App::clean_app_name(app);
            let app_display = if area.width < 40 {
                if clean_app.len() > 15 {
                    format!("{}...", &clean_app[..12])
                } else {
                    clean_app
                }
            } else {
                clean_app
            };

            let display = if hours > 0 {
                format!("  {} - {}h {}m", app_display, hours, minutes)
            } else {
                format!("  {} - {}m", app_display, minutes)
            };
            ListItem::new(Line::from(display))
        })
        .collect::<Vec<ListItem>>());

    let total_duration: i64 = data.iter().map(|(_, d)| d).sum();
    let total_hours = total_duration / 3600;
    let total_minutes = (total_duration % 3600) / 60;
    let stats_title = if total_hours > 0 {
        format!("📈 Detailed Stats (Total: {}h {}m)", total_hours, total_minutes)
    } else {
        format!("📈 Detailed Stats (Total: {}m)", total_minutes)
    };

    let stats_list = List::new(stats_items)
        .block(Block::default().borders(Borders::ALL).title(stats_title));
    f.render_widget(stats_list, area);
}

pub fn draw_history(app: &App, f: &mut Frame, area: Rect) {
    // Adaptive number of items based on available height
    let max_items = (area.height.saturating_sub(3) as usize).min(30).max(5);

    let mut history_items: Vec<ListItem> = Vec::new();

    // Add top margin
    history_items.push(ListItem::new(Line::from("")));

    // Add current session first with real-time duration
    if let Some(current_session) = &app.current_session {
        let current_duration = Local::now().signed_duration_since(current_session.start_time).num_seconds();
        let minutes = current_duration / 60;
        let time = current_session.start_time.format("%H:%M");

        // Create display name with window name if available
        let clean_app = App::clean_app_name(&current_session.app_name);
        let display_name = if let Some(window_name) = &current_session.window_name {
            if area.width < 40 {
                // Truncate both app and window names for narrow terminals
                let app_short = if clean_app.len() > 8 {
                    format!("{}...", &clean_app[..5])
                } else {
                    clean_app.clone()
                };
                let window_short = if window_name.len() > 8 {
                    format!("{}...", &window_name[..5])
                } else {
                    window_name.clone()
                };
                format!("{} ({})", app_short, window_short)
            } else {
                format!("{} ({})", clean_app, window_name)
            }
        } else {
            // Fallback to just app name if no window name
            if area.width < 40 {
                if clean_app.len() > 12 {
                    format!("{}...", &clean_app[..9])
                } else {
                    clean_app
                }
            } else {
                clean_app
            }
        };

        let display = format!("{} - {}: {}m [LIVE]", time, display_name, minutes);
        history_items.push(ListItem::new(Line::from(display)).style(Style::default().fg(Color::Green)));
    }

    // Add historical sessions
    let remaining_slots = max_items.saturating_sub(history_items.len());
    history_items.extend(
        app.current_history
            .iter()
            .take(remaining_slots)
            .map(|session| {
                let minutes = session.duration / 60;
                let time = session.start_time.format("%H:%M");

                // Create display name with window name if available
                let clean_app = App::clean_app_name(&session.app_name);
                let display_name = if let Some(window_name) = &session.window_name {
                    if area.width < 40 {
                        // Truncate both app and window names for narrow terminals
                        let app_short = if clean_app.len() > 8 {
                            format!("{}...", &clean_app[..5])
                        } else {
                            clean_app.clone()
                        };
                        let window_short = if window_name.len() > 8 {
                            format!("{}...", &window_name[..5])
                        } else {
                            window_name.clone()
                        };
                        format!("{} ({})", app_short, window_short)
                    } else {
                        format!("{} ({})", clean_app, window_name)
                    }
                } else {
                    // Fallback to just app name if no window name
                    if area.width < 40 {
                        if clean_app.len() > 12 {
                            format!("{}...", &clean_app[..9])
                        } else {
                            clean_app
                        }
                    } else {
                        clean_app
                    }
                };

                let display = format!("{} - {}: {}m", time, display_name, minutes);
                ListItem::new(Line::from(display))
            })
            .collect::<Vec<ListItem>>()
    );

    let history_list = List::new(history_items)
        .block(Block::default().borders(Borders::ALL).title("📜 Session History"));
    f.render_widget(history_list, area);
}

pub fn draw_pie_chart(app: &App, f: &mut Frame, area: Rect, data: &[(String, i64)]) {
    // Calculate category totals - using BTreeMap for stable sorted order
    let mut categories: BTreeMap<&str, (i64, Color)> = BTreeMap::new();
    let total: i64 = data.iter().map(|(_, d)| d).sum();

    for (app_name, duration) in data {
        let (category, color) = app.get_app_category(app_name);
        let entry = categories.entry(category).or_insert((0, color));
        entry.0 += duration;
    }

    // Create pie chart representation as text
    let mut pie_lines = vec![];
    pie_lines.push(Line::from(""));

    // Sort by duration descending for consistent display
    let mut sorted_cats: Vec<_> = categories.iter().collect();
    sorted_cats.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    for (category, (duration, color)) in sorted_cats {
        if total > 0 {
            let percentage = (*duration as f64 / total as f64 * 100.0) as u64;
            let bar_length = (percentage / 5).max(1) as usize; // Scale down for display
            let bar = "█".repeat(bar_length);
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let time_str = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };

            pie_lines.push(Line::from(vec![
                ratatui::text::Span::styled(format!("{} ", category), Style::default().fg(*color)),
                ratatui::text::Span::styled(bar, Style::default().fg(*color)),
                ratatui::text::Span::raw(format!(" {}% ({})", percentage, time_str)),
            ]));
        }
    }

    let pie_chart = Paragraph::new(pie_lines)
        .block(Block::default().borders(Borders::ALL).title("🥧 Categories"));
    f.render_widget(pie_chart, area);
}

pub fn draw_timeline(app: &App, f: &mut Frame, area: Rect) {
    // Real-time progress bars showing % of day for each app
    let mut progress_lines = vec![];

    if app.daily_usage.is_empty() {
        progress_lines.push(Line::from("No activity data yet today"));
        let progress = Paragraph::new(progress_lines)
            .block(Block::default().borders(Borders::ALL).title("📊 Today's Activity Progress"));
        f.render_widget(progress, area);
        return;
    }

    // Calculate total seconds in the day so far
    let now = Local::now();
    let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap();
    let seconds_since_midnight = now.signed_duration_since(start_of_day).num_seconds() as f64;

    // Sort apps by usage time (descending)
    let mut sorted_apps: Vec<_> = app.daily_usage.iter().collect();
    sorted_apps.sort_by(|a, b| b.1.cmp(&a.1));

    // Limit to top apps that fit in the area
    let max_items = (area.height.saturating_sub(4) as usize).min(10).max(3);
    let top_apps = &sorted_apps[..sorted_apps.len().min(max_items)];

    // Add top margin (consistent with other cards)
    progress_lines.push(Line::from(""));

    for (app_name, total_seconds) in top_apps {
        let clean_app_name = App::clean_app_name(app_name);
        let (_, color) = app.get_app_category(app_name);

        // Calculate percentage of day
        let percentage = if seconds_since_midnight > 0.0 {
            ((*total_seconds as f64 / seconds_since_midnight) * 100.0).min(100.0)
        } else {
            0.0
        };

        // Create progress bar (only filled portion visible)
        let bar_width = (area.width.saturating_sub(20) as usize).max(10); // Reserve space for labels
        let filled_width = ((percentage / 100.0) * bar_width as f64) as usize;

        let mut bar_chars = String::new();
        for i in 0..bar_width {
            if i < filled_width {
                bar_chars.push('█');
            } else {
                bar_chars.push(' ');
            }
        }

        // Create the progress line
        let progress_line = vec![
            ratatui::text::Span::styled(format!("{:<12}", clean_app_name), Style::default().fg(color)),
            ratatui::text::Span::styled(format!("{:>5.1}%", percentage), Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" "),
            ratatui::text::Span::styled(bar_chars, Style::default().fg(color)),
        ];

        progress_lines.push(Line::from(progress_line));
    }



    let progress = Paragraph::new(progress_lines)
        .block(Block::default().borders(Borders::ALL).title("📊 Today's Activity Progress"));
    f.render_widget(progress, area);
}

pub fn draw_afk(app: &App, f: &mut Frame, area: Rect) {
    let afk_threshold_secs = 300; // 5 minutes
    let is_afk = app.is_afk(afk_threshold_secs);
    let status = if is_afk { "AFK" } else { "Active" };
    let color = if is_afk { Color::Red } else { Color::Green };
    let last_input = *app.last_input.lock().unwrap();
    let idle_duration = Local::now().signed_duration_since(last_input).num_seconds();
    let idle_minutes = idle_duration / 60;
    let idle_seconds = idle_duration % 60;

    // Calculate average keyboard activity percentage (total tracking time vs idle time)
    // Use daily usage to calculate total tracking time today
    let total_tracking_today: i64 = app.daily_usage.iter().map(|(_, d)| d).sum();

    // Estimate active time: total tracking time - (idle time if currently AFK)
    let active_time = if is_afk && idle_duration > afk_threshold_secs {
        total_tracking_today.saturating_sub(idle_duration)
    } else {
        total_tracking_today
    };

    let avg_activity_percentage = if total_tracking_today > 0 {
        ((active_time as f64 / total_tracking_today as f64) * 100.0).min(100.0)
    } else {
        100.0 // Default to 100% if no data yet
    };

    let afk_lines = vec![
        Line::from(""),
        Line::from(vec![
            ratatui::text::Span::styled("Status: ", Style::default()),
            ratatui::text::Span::styled(status, Style::default().fg(color)),
        ]),
        Line::from(""),
        Line::from(format!("Idle for: {}m {}s", idle_minutes, idle_seconds)),
        Line::from(""),
        Line::from(vec![
            ratatui::text::Span::styled("Avg Activity: ", Style::default()),
            ratatui::text::Span::styled(
                format!("{:.1}%", avg_activity_percentage),
                Style::default().fg(Color::Cyan)
            ),
        ]),
        Line::from(""),
        Line::from("Detects keyboard/mouse activity"),
        Line::from("AFK if idle > 5 minutes"),
    ];

    let afk_paragraph = Paragraph::new(afk_lines)
        .block(Block::default().borders(Borders::ALL).title("🚫 AFK Status"));
    f.render_widget(afk_paragraph, area);
}

pub fn draw_breakdown_section(
    f: &mut Frame,
    area: Rect,
    title: &str,
    data: &[(String, i64)],
    color: Color,
    is_category: bool,
) {
    let max_items = (area.height.saturating_sub(3) as usize).max(3);
    let mut items: Vec<ListItem> = Vec::new();

    if data.is_empty() {
        items.push(ListItem::new(Line::from("  No data available")));
    } else {
        for (name, duration) in data.iter().take(max_items) {
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let time_str = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };

            // For categories, extract color from category name
            let item_color = if is_category {
                App::category_from_string(name).1
            } else {
                color
            };

            let display = format!("  {}  {}", name, time_str);
            items.push(ListItem::new(Line::from(display)).style(Style::default().fg(item_color)));
        }
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(list, area);
}

pub fn draw_file_breakdown_section(
    app: &App,
    f: &mut Frame,
    area: Rect,
    _scroll_position: usize,
) {
    let max_items = (area.height.saturating_sub(3) as usize).max(3);
    let mut items: Vec<ListItem> = Vec::new();

    if app.file_breakdown.is_empty() {
        items.push(ListItem::new(Line::from("  No file data available")));
    } else {
        for (filename, language, duration) in app.file_breakdown.iter().take(max_items) {
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let time_str = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };

            let display = format!("  {} ({})  {}", filename, language, time_str);
            items.push(ListItem::new(Line::from(display)).style(Style::default().fg(Color::Cyan)));
        }
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("📝 Files Edited"));
    f.render_widget(list, area);
}
