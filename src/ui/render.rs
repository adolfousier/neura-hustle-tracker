use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, List, ListItem, Paragraph},
    style::{Color, Style, Modifier},
    Frame,
};
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
                format!("Tracking: {} for {}s | [Shift+C] Commands | [s] History", display_name, duration)
            } else {
                format!("Not tracking - Current app: {} | [Shift+C] Commands | [s] History", app.current_app)
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
        AppState::BreakdownDashboard { .. } => "📊 Activity Breakdown Dashboard - [Tab] Switch Panels | [Enter] Select | [↑/↓/PgUp/PgDn] Navigate | [Esc] Close".to_string(),
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

        AppState::SelectingApp { selected_index, selected_unique_id: _ } => {
            // Full-screen app selection view
            let max_items = (chunks[1].height.saturating_sub(2) as usize).min(app.daily_usage.len()).max(5);
            let mut last_parent_color = Color::White;
            let usage_items: Vec<ListItem> = app
                .daily_usage
                .iter()
                .enumerate()
                .take(max_items)
                .map(|(i, item)| {
                    let duration = item.duration;
                    let hours = duration / 3600;
                    let minutes = (duration % 3600) / 60;
                    let seconds = duration % 60;
                    let prefix = if i == *selected_index { "→ " } else { "  " };

                    let time_display = if hours > 0 {
                        format!("{}h {}m {}s", hours, minutes, seconds)
                    } else if minutes > 0 {
                        format!("{}m {}s", minutes, seconds)
                    } else {
                        format!("{}s", seconds)
                    };

                    let clean_app = App::clean_app_name(&item.display_name);
                    let (_, color) = if item.is_sub_entry {
                        if let Some(parent) = &item.parent_app_name {
                            app.get_app_category(parent)
                        } else {
                            app.get_app_category(&item.display_name)
                        }
                    } else {
                        app.get_app_category(&item.display_name)
                    };

                    if !item.is_sub_entry {
                        last_parent_color = color;
                    }

                    let display = format!("{}{:<30} {}", prefix, clean_app, time_display);

                    let style = if i == *selected_index {
                        Style::default().fg(Color::Yellow)
                    } else if item.is_sub_entry {
                        Style::default().fg(last_parent_color)
                    } else {
                        Style::default().fg(color)
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

        AppState::SelectingCategory { selected_index, selected_unique_id: _, scroll_offset } => {
            // Full-screen app selection view for category assignment
            let viewport_height = chunks[1].height.saturating_sub(2) as usize;
            let max_items = viewport_height.min(app.daily_usage.len()).max(5);
            let mut last_parent_color = Color::White;
            let usage_items: Vec<ListItem> = app
                .daily_usage
                .iter()
                .skip(*scroll_offset)
                .take(max_items)
                .enumerate()
                .map(|(i, item)| {
                    let actual_index = i + *scroll_offset;
                    let duration = item.duration;
                    let hours = duration / 3600;
                    let minutes = (duration % 3600) / 60;
                    let seconds = duration % 60;
                    let prefix = if actual_index == *selected_index { "→ " } else { "  " };

                    let time_display = if hours > 0 {
                        format!("{}h {}m {}s", hours, minutes, seconds)
                    } else if minutes > 0 {
                        format!("{}m {}s", minutes, seconds)
                    } else {
                        format!("{}s", seconds)
                    };

                    let clean_app = App::clean_app_name(&item.display_name);
                    let (category, color) = if item.is_sub_entry {
                        // For sub-entries, use their own category if set, otherwise use parent's
                        if let Some(cat) = &item.category {
                            (cat.clone(), App::category_from_string(cat).1)
                        } else if let Some(parent) = &item.parent_app_name {
                            app.get_app_category(parent)
                        } else {
                            app.get_app_category(&item.display_name)
                        }
                    } else {
                        app.get_app_category(&item.display_name)
                    };

                    if !item.is_sub_entry {
                        last_parent_color = color;
                    }

                    let display = format!("{}{:<30} {} [{}]", prefix, clean_app, time_display, category);

                    let style = if actual_index == *selected_index {
                        Style::default().fg(Color::Yellow)
                    } else if item.is_sub_entry {
                        Style::default().fg(last_parent_color)
                    } else {
                        Style::default().fg(color)
                    };

                    ListItem::new(Line::from(display)).style(style)
                })
                .collect();

            let usage_list = List::new(usage_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("🏷️  Select App to Change Category (↑/↓/PgUp/PgDn to navigate, Enter to select, Esc to cancel)"));
            f.render_widget(usage_list, chunks[1]);
        }

        AppState::CategoryMenu { unique_id, selected_index, scroll_offset } => {
            // Show category selection menu
            let categories = app.get_category_options();
            let viewport_height = chunks[1].height.saturating_sub(2) as usize;
            let max_items = viewport_height.min(categories.len()).max(5);
            let category_items: Vec<ListItem> = categories
                .iter()
                .skip(*scroll_offset)
                .take(max_items)
                .enumerate()
                .map(|(i, category)| {
                    let actual_index = i + *scroll_offset;
                    let prefix = if actual_index == *selected_index { "→ " } else { "  " };
                    let display = format!("{}{}", prefix, category);

                    let style = if actual_index == *selected_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        // Apply color based on category
                        let color = App::category_from_string(category).1;
                        Style::default().fg(color)
                    };

                    ListItem::new(Line::from(display)).style(style)
                })
                .collect();

            // Extract app name from unique_id (format: "app_name:actual_name")
            let app_name = if unique_id.starts_with("app_name:") {
                unique_id.strip_prefix("app_name:").unwrap_or(unique_id)
            } else {
                unique_id
            };
            let clean_app = App::clean_app_name(app_name);
            let title = format!("🏷️  Select Category for '{}' (↑/↓/PgUp/PgDn to navigate, Enter to select, Esc to cancel)", clean_app);
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
                Line::from("  [d]    Switch to Daily view"),
                Line::from("  [w]    Switch to Weekly view"),
                Line::from("  [m]    Switch to Monthly view"),
                Line::from("  [s]    View session history (scrollable popup)"),
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

                let seconds = session.duration % 60;
                let display = if minutes > 0 {
                    format!("{}  {} - {}m {}s", time, display_name, minutes, seconds)
                } else {
                    format!("{}  {} - {}s", time, display_name, seconds)
                };
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

        AppState::BreakdownDashboard { view_mode, selected_panel, panel_scrolls } => {
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

            // Inner area for layout
            let inner_area = popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

            // Determine layout based on available width
            let use_vertical_layout = inner_area.width < 100;

            if use_vertical_layout {
                // Vertical stack layout for small screens
                let sections = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                    ].as_ref())
                    .split(inner_area);

                // Render each section with highlighting
                let category_style = if *selected_panel == 0 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let browser_style = if *selected_panel == 1 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let project_style = if *selected_panel == 2 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let file_style = if *selected_panel == 3 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let terminal_style = if *selected_panel == 4 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                draw_breakdown_section_with_style(f, sections[0], "📦 Categories", &app.category_breakdown, Color::Magenta, true, category_style, panel_scrolls[0]);
                draw_breakdown_section_with_style(f, sections[1], "🌐 Browser Services", &app.browser_breakdown, Color::Blue, false, browser_style, panel_scrolls[1]);
                draw_breakdown_section_with_style(f, sections[2], "📁 Projects", &app.project_breakdown, Color::Yellow, false, project_style, panel_scrolls[2]);
                app.draw_file_breakdown_section_with_style(f, sections[3], panel_scrolls[3], file_style);
                draw_breakdown_section_with_style(f, sections[4], "💻 Terminal Sessions", &app.terminal_breakdown, Color::Green, false, terminal_style, panel_scrolls[4]);
            } else {
                // Grid layout for larger screens
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

                // Render each section with highlighting
                let category_style = if *selected_panel == 0 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let browser_style = if *selected_panel == 1 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let project_style = if *selected_panel == 2 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let file_style = if *selected_panel == 3 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let terminal_style = if *selected_panel == 4 {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                draw_breakdown_section_with_style(f, row1_cols[0], "📦 Categories", &app.category_breakdown, Color::Magenta, true, category_style, panel_scrolls[0]);
                draw_breakdown_section_with_style(f, row1_cols[1], "🌐 Browser Services", &app.browser_breakdown, Color::Blue, false, browser_style, panel_scrolls[1]);
                draw_breakdown_section_with_style(f, row2_cols[0], "📁 Projects", &app.project_breakdown, Color::Yellow, false, project_style, panel_scrolls[2]);
                app.draw_file_breakdown_section_with_style(f, row2_cols[1], panel_scrolls[3], file_style);
                draw_breakdown_section_with_style(f, row3_area, "💻 Terminal Sessions", &app.terminal_breakdown, Color::Green, false, terminal_style, panel_scrolls[4]);
            }
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
        ViewMode::Daily => (app.daily_usage.clone(), "📊 Daily Usage"),
        ViewMode::Weekly => (app.weekly_usage.clone(), "📊 Weekly Usage (7 days)"),
        ViewMode::Monthly => (app.monthly_usage.clone(), "📊 Monthly Usage (30 days)"),
    };

    // Create a mutable clone to sort for the bar chart, filtering out sub-entries
    let mut sorted_bar_data: Vec<_> = data.iter().filter(|item| !item.is_sub_entry).cloned().collect();
    sorted_bar_data.sort_by(|a, b| b.duration.cmp(&a.duration));

    // Create bar chart data - limit based on space
    let max_bars = if area.width < 80 { 5 } else if area.width < 120 { 8 } else { 10 };
    let bar_data: &[crate::ui::hierarchical::HierarchicalDisplayItem] = &sorted_bar_data[..sorted_bar_data.len().min(max_bars)];

    if use_vertical_layout {
        // VERTICAL LAYOUT for small terminals
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),  // Bar chart
                Constraint::Min(8),   // Timeline (now sessions timeline)
                Constraint::Min(8),   // AFK
                Constraint::Min(8),   // Stats
                Constraint::Min(8),   // Categories
            ].as_ref())
            .split(area);

        app.draw_bar_chart(f, chunks[0], title, bar_data);
        draw_sessions_timeline(app, f, chunks[1], view_mode);
        app.draw_afk(f, chunks[2]);
        draw_stats(f, chunks[3], &data, app);
        app.draw_pie_chart(f, chunks[4], &data);
    } else {
        // HORIZONTAL LAYOUT for larger terminals
        // Main layout: top section (left/right) + bottom section (timeline)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(85),  // Top section (dashboard)
                Constraint::Percentage(15),  // Bottom section (sessions timeline)
            ].as_ref())
            .split(area);

        // TOP SECTION: 50/50 left/right split
        let dashboard_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ].as_ref())
            .split(main_chunks[0]);

        // LEFT SIDE: Bar Chart + Timeline/AFK
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),   // Bar chart
                Constraint::Percentage(50),   // Timeline + AFK
            ].as_ref())
            .split(dashboard_chunks[0]);

        // RIGHT SIDE: Detailed Stats + Categories
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),   // Detailed stats
                Constraint::Percentage(50),   // Categories (pie chart)
            ].as_ref())
            .split(dashboard_chunks[1]);

        app.draw_bar_chart(f, left_chunks[0], title, bar_data);
        let timeline_afk_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(left_chunks[1]);
        app.draw_timeline(f, timeline_afk_chunks[0], view_mode);
        app.draw_afk(f, timeline_afk_chunks[1]);
        draw_stats(f, right_chunks[0], &data, app);
        app.draw_pie_chart(f, right_chunks[1], &data);

        // BOTTOM: Sessions Timeline (full width)
        draw_sessions_timeline(app, f, main_chunks[1], view_mode);
    }
}

pub fn draw_bar_chart(app: &App, f: &mut Frame, area: Rect, title: &str, bar_data: &[crate::ui::hierarchical::HierarchicalDisplayItem]) {
    if bar_data.is_empty() {
        let empty_msg = Paragraph::new("No data available yet. Start tracking!")
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(empty_msg, area);
    } else {
        // Adaptive bar width based on terminal width
        let bar_width = 10;
        let bar_gap = if area.width < 60 { 0 } else { 1 };

        // Find max value in minutes
        let max_minutes = bar_data.iter().map(|item| (item.duration / 60) as u64).max().unwrap_or(0);

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
            .map(|item| {
                let value_minutes = (item.duration / 60) as u64;
                // Determine color: if sub-entry, use parent's color; otherwise use own category
                let (_, color) = if item.is_sub_entry {
                    // This is a sub-entry - use parent app's category color
                    if let Some(parent) = &item.parent_app_name {
                        app.get_app_category(parent)
                    } else {
                        // Fallback to white if parent is not found
                        ("".to_string(), Color::White)
                    }
                } else {
                    // This is a parent app - use its own category
                    if let Some(cat) = &item.category {
                        App::category_from_string(cat)
                    } else {
                        app.get_app_category(&item.display_name)
                    }
                };

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

                let clean_app = App::clean_app_name(&item.display_name).trim().to_string();
                let label = if clean_app.len() > bar_width as usize {
                    format!("{:<width$.width$}", &clean_app[..bar_width as usize], width = bar_width as usize)
                } else {
                    format!("{:<width$}", clean_app, width = bar_width as usize)
                };
                Bar::default()
                    .value(value_minutes)
                    .label(Line::from(label))
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

pub fn draw_stats(f: &mut Frame, area: Rect, data: &[crate::ui::hierarchical::HierarchicalDisplayItem], app: &App) {
    // Adaptive number of items based on available height - more items for hierarchical view
    let max_items = (area.height.saturating_sub(3) as usize).min(30).max(5);

    let mut stats_items: Vec<ListItem> = Vec::new();

    // Add top margin
    stats_items.push(ListItem::new(Line::from("")));

    // Group data hierarchically by category
    // We'll detect if an item is a sub-entry
    let mut shown_items = 0;
    let mut last_parent_color = Color::White;
    for item in data.iter() {
        if shown_items >= max_items {
            break;
        }

        let hours = item.duration / 3600;
        let minutes = (item.duration % 3600) / 60;
        let seconds = item.duration % 60;

        // Check if this is a child item (hierarchical sub-entry)
        let is_child = item.is_sub_entry;

        // Clean and truncate app name if terminal is narrow
        let clean_app = App::clean_app_name(&item.display_name);
        let app_display = if area.width < 40 {
            if clean_app.len() > 20 {
                format!("{}...", &clean_app[..17])
            } else {
                clean_app
            }
        } else {
            clean_app
        };

        let time_str = if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        };

        // Format display based on whether it's a parent or child entry
        let display = if is_child {
            // Child entries need indentation
            format!("  {}  {}", app_display, time_str)
        } else {
            // Parent entries
            format!("  {} - {}", app_display, time_str)
        };

        // Color based on category
        let item_style = if is_child {
            // For sub-entries, use their own category if set, otherwise use parent's color
            let color = if let Some(cat) = &item.category {
                App::category_from_string(cat).1
            } else {
                last_parent_color
            };
            Style::default().fg(color)
        } else {
            // For parent entries, use their app category
            let (_, color) = app.get_app_category(&item.display_name);
            last_parent_color = color;
            Style::default().fg(color)
        };

        stats_items.push(ListItem::new(Line::from(display)).style(item_style));
        shown_items += 1;
    }

    let total_duration: i64 = data.iter()
        .filter(|item| !item.is_sub_entry) // Only count parent entries
        .map(|item| item.duration)
        .sum();
    let total_hours = total_duration / 3600;
    let total_minutes = (total_duration % 3600) / 60;
    let total_seconds = total_duration % 60;
    let stats_title = if total_hours > 0 {
        format!("📈 Detailed Stats (Total: {}h {}m {}s)", total_hours, total_minutes, total_seconds)
    } else if total_minutes > 0 {
        format!("📈 Detailed Stats (Total: {}m {}s)", total_minutes, total_seconds)
    } else {
        format!("📈 Detailed Stats (Total: {}s)", total_seconds)
    };

    let stats_list = List::new(stats_items)
        .block(Block::default().borders(Borders::ALL).title(stats_title));
    f.render_widget(stats_list, area);
}

pub fn draw_pie_chart(app: &App, f: &mut Frame, area: Rect, data: &[crate::ui::hierarchical::HierarchicalDisplayItem]) {
    // Calculate category totals - using BTreeMap for stable sorted order
    // Filter out sub-entries
    let mut categories: BTreeMap<String, (i64, Color)> = BTreeMap::new();
    let total: i64 = data.iter()
        .filter(|item| !item.is_sub_entry)
        .map(|item| item.duration)
        .sum();

    for item in data.iter().filter(|item| !item.is_sub_entry) {
        let (category, color) = if let Some(cat) = &item.category {
            App::category_from_string(cat)
        } else {
            app.get_app_category(&item.display_name)
        };
        let entry = categories.entry(category).or_insert((0, color));
        entry.0 += item.duration;
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
            let seconds = duration % 60;
            let time_str = if hours > 0 {
                format!("{}h {}m {}s", hours, minutes, seconds)
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
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

pub fn draw_timeline(app: &App, f: &mut Frame, area: Rect, view_mode: &ViewMode) {
    use chrono::Datelike;

    // Progress bars showing % of selected period for each app
    let mut progress_lines = vec![];

    // Get the correct data source based on view mode
    let (usage_data, title, total_seconds) = match view_mode {
        ViewMode::Daily => {
            if app.flat_daily_usage.is_empty() {
                progress_lines.push(Line::from("No activity data yet today"));
                let progress = Paragraph::new(progress_lines)
                    .block(Block::default().borders(Borders::ALL).title("📊 Today's Activity Progress"));
                f.render_widget(progress, area);
                return;
            }
            let now = Local::now();
            let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap();
            let secs = now.signed_duration_since(start_of_day).num_seconds() as f64;
            (app.flat_daily_usage.clone(), "📊 Today's Activity Progress".to_string(), secs)
        }
        ViewMode::Weekly => {
            // Calculate total seconds in the week
            let now = Local::now();
            let today = now.date_naive();
            let days_since_sunday = today.weekday().num_days_from_sunday();
            let week_start = today - chrono::Duration::days(days_since_sunday as i64);
            let week_start_time = week_start.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap();
            let secs = now.signed_duration_since(week_start_time).num_seconds() as f64;

            // Aggregate weekly usage from current_history
            let mut weekly_usage: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
            for session in &app.current_history {
                let session_date = session.start_time.date_naive();
                if session_date >= week_start {
                    *weekly_usage.entry(session.app_name.clone()).or_insert(0) += session.duration;
                }
            }
            let weekly_vec: Vec<(String, i64)> = weekly_usage.into_iter().collect();
            (weekly_vec, "📊 Weekly Activity Progress".to_string(), secs)
        }
        ViewMode::Monthly => {
            // Calculate total seconds in the month
            let now = Local::now();
            let today = now.date_naive();
            let month_start = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
            let month_start_time = month_start.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap();
            let secs = now.signed_duration_since(month_start_time).num_seconds() as f64;

            // Aggregate monthly usage from current_history
            let mut monthly_usage: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
            for session in &app.current_history {
                let session_date = session.start_time.date_naive();
                if session_date >= month_start {
                    *monthly_usage.entry(session.app_name.clone()).or_insert(0) += session.duration;
                }
            }
            let monthly_vec: Vec<(String, i64)> = monthly_usage.into_iter().collect();
            (monthly_vec, "📊 Monthly Activity Progress".to_string(), secs)
        }
    };

    if usage_data.is_empty() {
        progress_lines.push(Line::from("No activity data"));
        let progress = Paragraph::new(progress_lines)
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(progress, area);
        return;
    }

    // Sort apps by usage time (descending)
    let mut sorted_apps: Vec<_> = usage_data.iter().collect();
    sorted_apps.sort_by(|a, b| b.1.cmp(&a.1));

    // Limit to top apps that fit in the area
    let max_items = (area.height.saturating_sub(4) as usize).min(10).max(3);
    let top_apps = &sorted_apps[..sorted_apps.len().min(max_items)];

    // Add top margin (consistent with other cards)
    progress_lines.push(Line::from(""));

    for (app_name, duration) in top_apps {
        let clean_app_name = App::clean_app_name(app_name);
        let (_, color) = app.get_app_category(app_name);

        // Calculate percentage of period
        let percentage = if total_seconds > 0.0 {
            ((*duration as f64 / total_seconds) * 100.0).min(100.0)
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
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(progress, area);
}

pub fn draw_sessions_timeline(app: &App, f: &mut Frame, area: Rect, view_mode: &ViewMode) {
    use chrono::{Duration, NaiveDate, Datelike, Timelike};

    if area.height < 4 {
        let msg = Paragraph::new("Terminal too small for timeline");
        f.render_widget(msg, area);
        return;
    }

    let now = Local::now();

    // Determine timeline based on view mode and collect data
    let (sessions_data, num_periods, title, is_daily) = match view_mode {
        ViewMode::Daily => {
            // For daily: show 24 hours, group sessions by hour
            let today = now.date_naive();
            let mut hourly: Vec<(i64, Vec<&crate::models::session::Session>)> = (0..24)
                .map(|h| (h, vec![]))
                .collect();
            for session in &app.current_history {
                if session.start_time.date_naive() == today {
                    let hour = session.start_time.hour() as i64;
                    if hour < 24 {
                        hourly[hour as usize].1.push(session);
                    }
                }
            }
            (hourly, 24, "📅 Today Sessions (24-Hour Activity Timeline)", true)
        }
        ViewMode::Weekly => {
            // For weekly: show 7 days
            let date = now.date_naive();
            let days_since_sunday = date.weekday().num_days_from_sunday() as i64;
            let week_start = date - Duration::days(days_since_sunday);
            let mut daily: Vec<(i64, Vec<&crate::models::session::Session>)> = (0..7)
                .map(|d| (d, vec![]))
                .collect();
            for session in &app.current_history {
                let session_date = session.start_time.date_naive();
                if session_date >= week_start && session_date < week_start + Duration::days(7) {
                    let day_offset = (session_date - week_start).num_days();
                    if day_offset >= 0 && day_offset < 7 {
                        daily[day_offset as usize].1.push(session);
                    }
                }
            }
            (daily, 7, "📅 Weekly Sessions (7-Day Activity Timeline)", false)
        }
        ViewMode::Monthly => {
            // For monthly: show days in current month
            let date = now.date_naive();
            let month_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
            let days_in_month = if date.month() == 12 {
                (NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap() - month_start).num_days()
            } else {
                (NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap() - month_start).num_days()
            } as i64;
            let mut daily: Vec<(i64, Vec<&crate::models::session::Session>)> = (0..days_in_month)
                .map(|d| (d, vec![]))
                .collect();
            for session in &app.current_history {
                let session_date = session.start_time.date_naive();
                if session_date >= month_start && session_date < month_start + Duration::days(days_in_month) {
                    let day_offset = (session_date - month_start).num_days();
                    if day_offset >= 0 && day_offset < days_in_month {
                        daily[day_offset as usize].1.push(session);
                    }
                }
            }
            (daily, days_in_month, "📅 Monthly Sessions (Day-By-Day Activity Timeline)", false)
        }
    };

    let mut lines = vec![];
    lines.push(ratatui::text::Line::from(""));

    // Use nearly full width for timeline visualization
    let available_width = (area.width as usize).saturating_sub(2);
    let num_periods_usize = num_periods as usize;

    // Calculate block width to fill available space evenly
    let block_width = (available_width / num_periods_usize).max(1);

    let mut bar_line = vec![];

    for (_, session_pair) in sessions_data.iter().enumerate() {
        let sessions = &session_pair.1;
        // Get dominant category color for this period
        let color = if sessions.is_empty() {
            // Empty period - use dim gray
            Color::DarkGray
        } else {
            // Calculate dominant category by duration
            let category_map = sessions.iter()
                .fold(std::collections::HashMap::new(), |mut map, s| {
                    let cat = s.category.as_deref().unwrap_or("📦 Other");
                    *map.entry(cat.to_string()).or_insert(0i64) += s.duration;
                    map
                });

            let dominant = category_map.into_iter()
                .max_by_key(|(_, dur)| *dur)
                .map(|(cat, _)| cat)
                .unwrap_or_else(|| "📦 Other".to_string());

            // Use category_from_string directly since dominant already has the emoji+name format
            App::category_from_string(&dominant).1
        };

        // Draw colored blocks - use full block character for better visibility
        let block_char = if sessions.is_empty() { "░" } else { "█" };
        for _ in 0..block_width {
            bar_line.push(ratatui::text::Span::styled(block_char, Style::default().fg(color)));
        }
    }

    lines.push(ratatui::text::Line::from(bar_line));

    // Build label line - adaptive label frequency based on available space
    let label_interval = if is_daily {
        if available_width > 200 {
            1  // Show every hour
        } else if available_width > 100 {
            2  // Show every 2 hours
        } else {
            4  // Show every 4 hours
        }
    } else {
        1  // Always show every day/period for weekly/monthly
    };

    let mut label_spans = vec![];
    for idx in 0..num_periods_usize {
        if idx as i64 % label_interval == 0 {
            let label = match view_mode {
                ViewMode::Daily => format!("{:02}h", idx),
                ViewMode::Weekly => {
                    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                    days[idx % 7].to_string()
                }
                ViewMode::Monthly => format!("{}", idx + 1),
            };

            // Calculate starting position for this label
            let label_start = idx * block_width;
            let next_label_start = ((idx as i64 + label_interval) as usize).min(num_periods_usize) * block_width;
            let label_space = next_label_start.saturating_sub(label_start).max(label.len());

            label_spans.push(ratatui::text::Span::raw(format!("{:<width$}", label, width = label_space)));
        } else if idx as i64 % label_interval != (label_interval - 1) {
            // Add spacing between labels
            let space_width = block_width;
            label_spans.push(ratatui::text::Span::raw(" ".repeat(space_width)));
        }
    }

    if !label_spans.is_empty() {
        lines.push(ratatui::text::Line::from(label_spans));
    }

    // Summary with category breakdown
    let total_duration: i64 = sessions_data.iter()
        .flat_map(|(_, s)| s.iter())
        .map(|s| s.duration)
        .sum();
    let hours = total_duration / 3600;
    let minutes = (total_duration % 3600) / 60;

    // Build category summary for legend
    let mut category_durations: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for (_, sessions) in &sessions_data {
        for session in sessions {
            let cat = session.category.as_deref().unwrap_or("Other").to_string();
            *category_durations.entry(cat).or_insert(0) += session.duration;
        }
    }

    let mut category_summary = vec![];
    let mut sorted_categories: Vec<&String> = category_durations.keys().collect();
    sorted_categories.sort_by_key(|cat| std::cmp::Reverse(category_durations[*cat]));

    for cat in sorted_categories.iter().take(5) {
        let cat_color = App::category_from_string(cat).1;
        category_summary.push(ratatui::text::Span::styled(
            format!("● {} ", cat),
            Style::default().fg(cat_color),
        ));
    }

    let mut summary_line = vec![
        ratatui::text::Span::raw(format!("Total: {}h {}m | ", hours, minutes)),
    ];
    summary_line.extend(category_summary);
    lines.push(ratatui::text::Line::from(summary_line));

    let timeline = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(timeline, area);
}

pub fn draw_afk(app: &App, f: &mut Frame, area: Rect) {
    let afk_threshold_secs = 300; // 5 minutes
    let is_afk = app.is_afk(afk_threshold_secs);
    let last_input = *app.last_input.lock().unwrap();
    let idle_duration = Local::now().signed_duration_since(last_input).num_seconds();
    let idle_minutes = idle_duration / 60;
    let idle_seconds = idle_duration % 60;

    // Determine AFK and IDLE status
    let is_idle = idle_duration >= 600; // 10 minutes = IDLE
    let status = if is_idle { "IDLE" } else if is_afk { "AFK" } else { "Active" };
    let color = if is_idle { Color::Yellow } else if is_afk { Color::Red } else { Color::Green };

    // Calculate average keyboard activity percentage
    // Avg Activity % = (Total Session Time - Total Accumulated Idle) / Total Session Time × 100%
    // Accumulates idle from ALL sessions today + current ongoing idle

    // Total tracking time today (excluding IDLE gaps only)
    let total_tracking_today: i64 = app.current_history.iter()
        .filter(|s| !s.is_idle.unwrap_or(false))  // Exclude IDLE (10+ min gaps)
        .map(|s| s.duration)
        .sum();

    // Sum accumulated idle from all past sessions (excluding IDLE gaps)
    let past_sessions_accumulated_idle: i64 = app.current_history.iter()
        .filter(|s| !s.is_idle.unwrap_or(false))
        .map(|s| s.idle_accumulation_secs.unwrap_or(0))
        .sum();

    // Current session accumulated idle (seconds)
    let current_session_accumulated_idle = if let Some(ref session) = app.current_session {
        session.idle_accumulation_secs.unwrap_or(0)
    } else {
        0
    };

    // Current session time (excluding IDLE)
    let current_session_time = if let Some(ref session) = app.current_session {
        if !session.is_idle.unwrap_or(false) {
            Local::now().signed_duration_since(session.start_time).num_seconds()
        } else {
            0
        }
    } else {
        0
    };

    // Total time in seconds
    let total_time_in_secs = total_tracking_today + current_session_time;

    // If currently idle, add the ongoing idle duration to current session accumulated idle
    let ongoing_idle_secs = if idle_duration > 0 && idle_duration < 600 && current_session_time > 0 {
        idle_duration  // Use the actual idle_duration from earlier in the function
    } else {
        0
    };

    // Total idle = past sessions idle + current session accumulated idle + ongoing idle
    let total_idle_secs = past_sessions_accumulated_idle + current_session_accumulated_idle + ongoing_idle_secs;
    let idle_to_subtract = total_idle_secs.min(total_time_in_secs);

    // Activity = (total time - idle) / total time
    let active_time_secs = total_time_in_secs.saturating_sub(idle_to_subtract);

    let avg_activity_percentage = if total_time_in_secs > 0 {
        ((active_time_secs as f64 / total_time_in_secs as f64) * 100.0).min(100.0)
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
        Line::from("IDLE if idle > 10 minutes"),
    ];

    let afk_paragraph = Paragraph::new(afk_lines)
        .block(Block::default().borders(Borders::ALL).title("🚫 AFK Status"));
    f.render_widget(afk_paragraph, area);
}

pub fn draw_breakdown_section_with_style(
    f: &mut Frame,
    area: Rect,
    title: &str,
    data: &[(String, i64)],
    color: Color,
    is_category: bool,
    style: Style,
    scroll_position: usize,
) {
    let max_items = (area.height.saturating_sub(3) as usize).max(3);
    let mut items: Vec<ListItem> = Vec::new();

    if data.is_empty() {
        items.push(ListItem::new(Line::from("  No data available")));
    } else {
        // Apply scroll position
        let start_idx = scroll_position.min(data.len().saturating_sub(max_items));
        let end_idx = (start_idx + max_items).min(data.len());
        
        for (name, duration) in data[start_idx..end_idx].iter() {
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let seconds = duration % 60;
            let time_str = if hours > 0 {
                format!("{}h {}m {}s", hours, minutes, seconds)
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
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
        .block(Block::default().borders(Borders::ALL).title(title).style(style));
    f.render_widget(list, area);
}

pub fn draw_file_breakdown_section_with_style(
    app: &App,
    f: &mut Frame,
    area: Rect,
    scroll_position: usize,
    style: Style,
) {
    let max_items = (area.height.saturating_sub(3) as usize).max(3);
    let mut items: Vec<ListItem> = Vec::new();

    if app.file_breakdown.is_empty() {
        items.push(ListItem::new(Line::from("  No file data available")));
    } else {
        // Apply scroll position
        let start_idx = scroll_position.min(app.file_breakdown.len().saturating_sub(max_items));
        let end_idx = (start_idx + max_items).min(app.file_breakdown.len());
        
        for (filename, language, duration) in app.file_breakdown[start_idx..end_idx].iter() {
            let hours = duration / 3600;
            let minutes = (duration % 3600) / 60;
            let seconds = duration % 60;
            let time_str = if hours > 0 {
                format!("{}h {}m {}s", hours, minutes, seconds)
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
            };

            let display = format!("  {} ({})  {}", filename, language, time_str);
            items.push(ListItem::new(Line::from(display)).style(Style::default().fg(Color::Cyan)));
        }
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("📝 Files Edited").style(style));
    f.render_widget(list, area);
}
