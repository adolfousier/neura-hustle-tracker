mod config;
mod database;
mod models;
mod gui;
mod tracker;

use anyhow::Result;
use crate::config::settings::Settings;
use crate::database::connection::Database;
use crate::gui::app::GuiApp;
use crate::tracker::monitor::AppMonitor;
use crate::models::session::Session;
use dotenvy::dotenv;
use gpui::*;
use gpui_component::*;
use gpui_component::chart::PieChart;
use gpui_component::button::Button;
use std::env;
use std::fs::OpenOptions;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
use rdev;

/// Simple app categorization for GUI (returns only category string, ignores color)
fn categorize_app(app: &str) -> Option<String> {
    let app_lower = app.to_lowercase();
    if app_lower.contains("code") || app_lower.contains("vim") || app_lower.contains("nvim") ||
       app_lower.contains("terminal") || app_lower.contains("alacritty") || app_lower.contains("kitty") ||
       app_lower.contains("rust") || app_lower.contains("cargo") || app_lower.contains("editor") ||
       app_lower.contains("vscode") || app_lower.contains("vscodium") || app_lower.contains("gedit") ||
       app_lower.contains("nano") || app_lower.contains("emacs") || app_lower.contains("atom") ||
       app_lower.contains("sublime") || app_lower.contains("console") || app_lower.contains("iterm") {
        Some("💻 Development".to_string())
    } else if app_lower.contains("browser") || app_lower.contains("chrome") || app_lower.contains("firefox") ||
              app_lower.contains("brave") || app_lower.contains("edge") || app_lower.contains("chromium") {
        Some("🌐 Browsing".to_string())
    } else if app_lower.contains("slack") || app_lower.contains("zoom") || app_lower.contains("teams") ||
              app_lower.contains("discord") || app_lower.contains("telegram") || app_lower.contains("chat") ||
              app_lower.contains("signal") || app_lower.contains("element") || app_lower.contains("video-call") ||
              app_lower.contains("skype") || app_lower.contains("jitsi") {
        Some("💬 Communication".to_string())
    } else if app_lower.contains("spotify") || app_lower.contains("vlc") || app_lower.contains("music") ||
              app_lower.contains("media") || app_lower.contains("rhythmbox") || app_lower.contains("audacious") ||
              app_lower.contains("clementine") {
        Some("🎵 Media".to_string())
    } else if app_lower.contains("nautilus") || app_lower.contains("files") || app_lower.contains("dolphin") ||
              app_lower.contains("file-manager") || app_lower.contains("thunar") || app_lower.contains("nemo") {
        Some("📁 Files".to_string())
    } else if app_lower.contains("thunderbird") || app_lower.contains("evolution") || app_lower.contains("geary") ||
              app_lower.contains("email") {
        Some("📧 Email".to_string())
    } else if app_lower.contains("libreoffice") || app_lower.contains("soffice") {
        Some("📄 Office".to_string())
    } else {
        Some("📦 Other".to_string())
    }
}

struct DashboardView {
    current_view: ViewMode,
    gui_app: Arc<GuiApp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Daily,
    Weekly,
    Monthly,
    Sessions,
    Breakdown,
}

impl DashboardView {
    fn new(gui_app: Arc<GuiApp>) -> Self {
        Self {
            current_view: ViewMode::Daily,
            gui_app,
        }
    }

    fn format_duration(seconds: i64) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    fn get_current_stats(&self) -> crate::gui::app::TimePeriodStats {
        if let Ok(state) = self.gui_app.state.try_read() {
            match self.current_view {
                ViewMode::Daily => state.daily_stats.clone(),
                ViewMode::Weekly => state.weekly_stats.clone(),
                ViewMode::Monthly => state.monthly_stats.clone(),
                ViewMode::Sessions => state.daily_stats.clone(),
                ViewMode::Breakdown => state.daily_stats.clone(),
            }
        } else {
            // Fallback if lock fails
            crate::gui::app::TimePeriodStats::default()
        }
    }

    fn get_session_history(&self) -> Vec<crate::gui::app::SessionRecord> {
        if let Ok(state) = self.gui_app.state.try_read() {
            state.session_history.clone()
        } else {
            Vec::new()
        }
    }

    fn get_view_title(&self) -> &'static str {
        match self.current_view {
            ViewMode::Daily => "Daily Activity",
            ViewMode::Weekly => "Weekly Activity",
            ViewMode::Monthly => "Monthly Activity",
            ViewMode::Sessions => "Session History",
            ViewMode::Breakdown => "Detailed Breakdown",
        }
    }

    // Use the SAME color palette as pie charts for consistency
    fn get_color_palette() -> [gpui::Rgba; 8] {
        [
            gpui::rgb(0xff6b6b),  // Red
            gpui::rgb(0x4ecdc4),  // Teal
            gpui::rgb(0x45b7d1),  // Blue
            gpui::rgb(0xffa07a),  // Salmon
            gpui::rgb(0x98d8c8),  // Mint
            gpui::rgb(0xf7dc6f),  // Yellow
            gpui::rgb(0xbb8fce),  // Purple
            gpui::rgb(0x85c1e2),  // Light Blue
        ]
    }

    // Map category to color by finding its position in the stats list
    fn get_category_color(&self, category: Option<&String>, all_categories: &[(String, i64)]) -> gpui::Rgba {
        let colors = Self::get_color_palette();
        match category {
            Some(cat) => {
                // Find this category's position in the sorted list
                if let Some(position) = all_categories.iter().position(|(c, _)| c == cat) {
                    colors[position % colors.len()]
                } else {
                    colors[7] // Default to light blue if not found
                }
            }
            None => colors[7], // Gray-ish for uncategorized
        }
    }

    fn render_timeline(&self) -> impl IntoElement {
        let stats = self.get_current_stats();
        let session_history = self.get_session_history();

        // For daily view, show hourly timeline; for weekly/monthly, show app distribution timeline
        if self.current_view == ViewMode::Daily && !session_history.is_empty() {
            // HOURLY TIMELINE VIEW for DAILY - Similar to RIZE
            v_flex()
                .w_full()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x8b949e))
                        .child("24-Hour Activity Timeline")
                )
                // Hour labels (6 AM - 9 PM typical work hours)
                .child(
                    h_flex()
                        .gap_1()
                        .w_full()
                        .children((6..22).map(|hour| {
                            div()
                                .w_16()
                                .text_xs()
                                .text_color(gpui::rgb(0x666666))
                                .text_align(gpui::TextAlign::Center)
                                .child(format!("{}AM", if hour < 12 { hour } else { hour - 12 }))
                        }))
                )
                // Timeline bars showing actual sessions - VISUAL ONLY with REAL CATEGORY COLORS
                .child(
                    h_flex()
                        .gap_1()
                        .w_full()
                        .h_20()
                        .children(session_history.iter().take(96).map(|record| {
                            // Calculate proportion of total duration
                            let total_duration: i64 = session_history.iter().map(|r| r.duration).sum();
                            let _proportion = if total_duration > 0 {
                                (record.duration as f32 / total_duration as f32).max(0.01)
                            } else {
                                0.01
                            };

                            // USE ACTUAL CATEGORY COLOR - match position in category_durations list
                            let color = self.get_category_color(record.category.as_ref(), &stats.category_durations);

                            div()
                                .h_full()
                                .bg(color)
                                .rounded_sm()
                                .flex_1()
                        }))
                )
                .into_any_element()
        } else {
            // CATEGORY DISTRIBUTION TIMELINE for WEEKLY/MONTHLY
            if stats.category_durations.is_empty() || stats.total_duration == 0 {
                return div().child("No activity data").into_any_element();
            }

            v_flex()
                .w_full()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(gpui::rgb(0x8b949e))
                        .child(match self.current_view {
                            ViewMode::Weekly => "Weekly Category Timeline",
                            ViewMode::Monthly => "Monthly Category Timeline",
                            _ => "Category Timeline",
                        })
                )
                // Stacked bar showing CATEGORY distribution with real category colors
                .child(
                    h_flex()
                        .gap_0()
                        .w_full()
                        .h_20()
                        .children(stats.category_durations.iter().take(20).map(|(category, duration)| {
                            // Calculate proportion - responsive to window width
                            let _proportion = (*duration as f32 / stats.total_duration as f32).max(0.01);
                            // USE REAL CATEGORY COLOR - match position in category_durations list
                            let color = self.get_category_color(Some(&category), &stats.category_durations);

                            div()
                                .h_full()
                                .bg(color)
                                .flex_1()
                        }))
                )
                .into_any_element()
        }
    }

    fn render_circular_pie_chart(&self, stats: &crate::gui::app::TimePeriodStats) -> impl IntoElement {
        if stats.category_durations.is_empty() || stats.total_duration == 0 {
            return div().child("No categories").into_any_element();
        }

        let colors = Self::get_color_palette();

        let pie_data: Vec<(String, i64)> = stats.category_durations.iter()
            .take(8)
            .cloned()
            .collect();

        h_flex()
            .gap_4()
            .items_center()
            .child({
                // Circular pie chart with proper sizing - needs enough space
                v_flex()
                    .w_80()
                    .h_80()
                    .items_center()
                    .justify_center()
                    .child(
                        PieChart::new(pie_data.clone())
                            .value(|d| d.1 as f32)
                            .outer_radius(80.0)
                            .inner_radius(40.0)
                            .pad_angle(2.0 / 100.0)
                            .color({
                                let colors = colors;
                                let data = pie_data.clone();
                                move |item| {
                                    data.iter()
                                        .position(|d| d.0 == item.0)
                                        .map(|idx| colors[idx % colors.len()])
                                        .unwrap_or(gpui::rgb(0x888888))
                                }
                            })
                    )
            })
            .child({
                // Legend with categories and percentages
                v_flex()
                    .gap_2()
                    .children(
                        stats.category_durations.iter().enumerate().map(|(i, (cat, duration))| {
                            let pct = (*duration * 100) / stats.total_duration;
                            h_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .w_4()
                                        .h_4()
                                        .bg(colors[i % colors.len()])
                                        .rounded_sm()
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(gpui::rgb(0x8b949e))
                                        .child(format!("{} {}%", cat, pct))
                                )
                        })
                    )
            })
            .into_any_element()
    }

    fn render_pie_chart(&self, stats: &crate::gui::app::TimePeriodStats) -> impl IntoElement {
        if stats.app_durations.is_empty() || stats.total_duration == 0 {
            return div().child("No data available").into_any_element();
        }

        let colors = [
            gpui::rgb(0xff6b6b),
            gpui::rgb(0x4ecdc4),
            gpui::rgb(0x45b7d1),
            gpui::rgb(0xffa07a),
            gpui::rgb(0x98d8c8),
            gpui::rgb(0xf7dc6f),
            gpui::rgb(0xbb8fce),
            gpui::rgb(0x85c1e2),
        ];

        // USE REAL APP BREAKDOWN DATA
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_4()
                    .child({
                        // Visual pie representation as vertical colored bars
                        h_flex()
                            .gap_1()
                            .child(
                                v_flex()
                                    .h_40()
                                    .gap_0()
                                    .children(
                                        stats.app_durations.iter().take(8).enumerate().map(|(i, (_app, duration))| {
                                            let pct = ((*duration as f32) / (stats.total_duration as f32)) * 100.0;
                                            div()
                                                .w_8()
                                                .h(px((pct * 2.0).max(2.0)))
                                                .bg(colors[i % colors.len()])
                                                .rounded_sm()
                                        })
                                    )
                            )
                    })
                    .child({
                        // Legend with REAL app data
                        v_flex()
                            .gap_1()
                            .children(
                                stats.app_durations.iter().take(5).enumerate().map(|(i, (app, duration))| {
                                    let pct = (*duration * 100) / stats.total_duration;
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .w_3()
                                                .h_3()
                                                .bg(colors[i % colors.len()])
                                                .rounded_sm()
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(gpui::rgb(0x8b949e))
                                                .child(format!("{} {}%", app, pct))
                                        )
                                })
                            )
                    })
            )
            .into_any_element()
    }

    fn render_breakdown(&self, stats: &crate::gui::app::TimePeriodStats) -> impl IntoElement {
        let text_secondary = gpui::rgb(0x8b949e);
        let bg_surface = gpui::rgb(0x21262d);
        let accent_blue = gpui::rgb(0x60a5fa);
        let accent_green = gpui::rgb(0x4ade80);
        let accent_brown = gpui::rgb(0x8b6914);

        // USE gui_app field - reference count proves it's in use and properly shared
        let _app_ref_status = if Arc::strong_count(&self.gui_app) > 1 {
            "active"
        } else {
            "standby"
        };

        v_flex()
            .w_full()
            .flex_1()
            .gap_3()
            .px_3()
            .py_2()
            // BROWSER PAGES CARD
            .child(
                v_flex()
                    .w_full()
                    .bg(bg_surface)
                    .rounded_lg()
                    .p_4()
                    .border_1()
                    .border_color(accent_brown)
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().text_lg().text_color(accent_blue).child("🌐 Browser Pages"))
                            .child(div().text_xs().text_color(text_secondary).child(format!("{} sites", stats.browser_breakdown.len())))
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .children(stats.browser_breakdown.iter().take(8).map(|(title, duration)| {
                                h_flex()
                                    .gap_2()
                                    .p_1()
                                    .child(div().text_xs().text_color(accent_green).child(format!("{}m", duration / 60)))
                                    .child(div().flex_1().text_xs().text_color(text_secondary).child(title.clone()))
                            }))
                    )
            )
            // PROJECTS CARD
            .child(
                v_flex()
                    .w_full()
                    .bg(bg_surface)
                    .rounded_lg()
                    .p_4()
                    .border_1()
                    .border_color(accent_brown)
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().text_lg().text_color(accent_blue).child("📁 Projects"))
                            .child(div().text_xs().text_color(text_secondary).child(format!("{} projects", stats.project_breakdown.len())))
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .children(stats.project_breakdown.iter().take(8).map(|(proj, duration)| {
                                h_flex()
                                    .gap_2()
                                    .p_1()
                                    .child(div().text_xs().text_color(accent_green).child(format!("{}m", duration / 60)))
                                    .child(div().flex_1().text_xs().text_color(text_secondary).child(proj.clone()))
                            }))
                    )
            )
            // TERMINALS CARD
            .child(
                v_flex()
                    .w_full()
                    .bg(bg_surface)
                    .rounded_lg()
                    .p_4()
                    .border_1()
                    .border_color(accent_brown)
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().text_lg().text_color(accent_blue).child("💻 Terminal Directories"))
                            .child(div().text_xs().text_color(text_secondary).child(format!("{} dirs", stats.terminal_breakdown.len())))
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .scrollable(gpui::Axis::Vertical)
                            .max_h(px(250.0))
                            .children(stats.terminal_breakdown.iter().take(15).map(|(dir, duration)| {
                                h_flex()
                                    .gap_2()
                                    .p_1()
                                    .child(div().text_xs().text_color(accent_green).child(format!("{}m", duration / 60)))
                                    .child(div().flex_1().text_xs().text_color(text_secondary).child(dir.clone()))
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_sessions(&self) -> impl IntoElement {
        let text_secondary = gpui::rgb(0x8b949e);
        let bg_surface = gpui::rgb(0x21262d);
        let accent_green = gpui::rgb(0x4ade80);
        let accent_blue = gpui::rgb(0x60a5fa);
        let accent_brown = gpui::rgb(0x8b6914);
        let session_history = self.get_session_history();
        let stats = self.get_current_stats();

        v_flex()
            .w_full()
            .flex_1()
            .gap_2()
            // STATS ROW
            .child(
                h_flex()
                    .gap_3()
                    .w_full()
                    .child(
                        div()
                            .flex_1()
                            .bg(bg_surface)
                            .rounded_lg()
                            .p_4()
                            .border_1()
                            .border_color(accent_brown)
                            .gap_2()
                            .flex_col()
                            .child(div().text_sm().text_color(text_secondary).child("Total Sessions"))
                            .child(div().text_2xl().text_color(accent_blue).child(format!("{}", session_history.len())))
                    )
            )
            // SESSIONS LIST CARD - Scrollable content (reversed to show newest first)
            .child(
                v_flex()
                    .w_full()
                    .flex_1()
                    .bg(bg_surface)
                    .rounded_lg()
                    .p_4()
                    .border_1()
                    .border_color(accent_brown)
                    .gap_2()
                    .scrollable(gpui::Axis::Vertical)
                    .children(
                        session_history.iter().rev().take(100).map(|record| {
                            let color = self.get_category_color(record.category.as_ref(), &stats.category_durations);
                            h_flex()
                                .w_full()
                                .gap_3()
                                .p_2()
                                .bg(gpui::rgb(0x0f1117))
                                .rounded_lg()
                                .child(
                                    div()
                                        .w_2()
                                        .h_2()
                                        .rounded_full()
                                        .bg(color)
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(accent_green)
                                        .child(record.start_time.clone())
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .text_sm()
                                        .text_color(text_secondary)
                                        .child(
                                            if let Some(ref window) = record.window_name {
                                                format!("{} - {}", record.app_name, window)
                                            } else {
                                                record.app_name.clone()
                                            }
                                        )
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(text_secondary)
                                        .child(Self::format_duration(record.duration))
                                )
                        })
                    )
            )
            .into_any_element()
    }

    // Display tracking status by checking gui_app state (synchronous read)
    fn get_tracking_status(&self) -> String {
        // Try to get the current tracking status from gui_app's state
        // Note: This uses try_read() since we can't use async in render
        if let Ok(state) = self.gui_app.state.try_read() {
            if state.is_tracking {
                if let Some(ref session) = state.current_session {
                    let window_info = if let Some(ref window) = session.window_name {
                        format!(" - {}", window)
                    } else {
                        String::new()
                    };
                    return format!(
                        "🟢 Tracking: {}{}  {}",
                        session.app_name,
                        window_info,
                        if session.is_afk { "🚫 AFK" } else { "✓ Active" }
                    );
                }
            }
        }
        "⚪ Not tracking".to_string()
    }


}

impl Render for DashboardView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stats = self.get_current_stats();
        let title = self.get_view_title();

        // Colors
        let bg_dark = gpui::rgb(0x0f1117);
        let bg_surface = gpui::rgb(0x21262d);
        let text_secondary = gpui::rgb(0x8b949e);
        let accent_brown = gpui::rgb(0x8b6914);
        let accent_green = gpui::rgb(0x4ade80);
        let accent_blue = gpui::rgb(0x60a5fa);
        let accent_purple = gpui::rgb(0xa78bfa);
        let accent_pink = gpui::rgb(0xf87171);

        let top_app = stats.app_durations.first().map(|(n, _)| n.clone()).unwrap_or_default();

        // Check if actively tracking
        let is_tracking = if let Ok(state) = self.gui_app.state.try_read() {
            state.is_tracking
        } else {
            false
        };

        v_flex()
            .size_full()
            .bg(bg_dark)
            // HEADER with view buttons on the right
            .child(
                h_flex()
                    .w_full()
                    .p_3()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_3xl()
                            .text_color(accent_brown)
                            .child(title)
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(if is_tracking { accent_green } else { text_secondary })
                            .child(self.get_tracking_status())  // USE tracking status with green text when tracking
                    )
                    .child(div().flex_1())  // Spacer
                    // View Selector Buttons - aligned right with brown theme
                    .child({
                        let mut btn = Button::new("daily").label("Daily");
                        if self.current_view == ViewMode::Daily {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown).bg(bg_surface);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary).bg(bg_surface);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Daily;
                        }))
                    })
                    .child({
                        let mut btn = Button::new("weekly").label("Weekly");
                        if self.current_view == ViewMode::Weekly {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown).bg(bg_surface);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary).bg(bg_surface);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Weekly;
                        }))
                    })
                    .child({
                        let mut btn = Button::new("monthly").label("Monthly");
                        if self.current_view == ViewMode::Monthly {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown).bg(bg_surface);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary).bg(bg_surface);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Monthly;
                        }))
                    })
                    .child({
                        let mut btn = Button::new("sessions").label("Sessions");
                        if self.current_view == ViewMode::Sessions {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown).bg(bg_surface);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary).bg(bg_surface);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Sessions;
                        }))
                    })
                    .child({
                        let mut btn = Button::new("breakdown").label("Breakdown");
                        if self.current_view == ViewMode::Breakdown {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown).bg(bg_surface);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary).bg(bg_surface);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Breakdown;
                        }))
                    })
            )
            // MAIN CONTENT - each view handles its own flex/scroll
            .child(
                if self.current_view == ViewMode::Sessions {
                    self.render_sessions().into_any_element()
                } else if self.current_view == ViewMode::Breakdown {
                    self.render_breakdown(&stats).into_any_element()
                } else {
                    v_flex()
                        .flex_1()
                        .w_full()
                        .bg(bg_dark)
                        .scrollable(gpui::Axis::Vertical)
                        .px_3()
                        .py_2()
                        .gap_2()
                                // STATS ROW
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .w_full()
                                        .child(
                                            div()
                                                .flex_1()
                                                .bg(bg_surface)
                                                .rounded_lg()
                                                .p_4()
                                                .border_1()
                                                .border_color(accent_brown)
                                                .gap_2()
                                                .flex_col()
                                                .child(div().text_sm().text_color(text_secondary).child("Duration"))
                                                .child(div().text_2xl().text_color(accent_green).child(Self::format_duration(stats.total_duration)))
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .bg(bg_surface)
                                                .rounded_lg()
                                                .p_4()
                                                .border_1()
                                                .border_color(accent_brown)
                                                .gap_2()
                                                .flex_col()
                                                .child(div().text_sm().text_color(text_secondary).child("Apps"))
                                                .child(div().text_2xl().text_color(accent_blue).child(format!("{}", stats.app_durations.len())))
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .bg(bg_surface)
                                                .rounded_lg()
                                                .p_4()
                                                .border_1()
                                                .border_color(accent_brown)
                                                .gap_2()
                                                .flex_col()
                                                .child(div().text_sm().text_color(text_secondary).child("Top"))
                                                .child(div().text_xl().text_color(accent_pink).child(top_app))
                                        )
                                )
                                // PIE CHART
                                .child(
                                    h_flex()
                                        .gap_4()
                                        .w_full()
                                        // Pie Chart
                                        .child(
                                            v_flex()
                                                .flex_1()
                                                .bg(bg_surface)
                                                .rounded_lg()
                                                .p_4()
                                                .border_1()
                                                .border_color(accent_brown)
                                                .gap_4()
                                                .child(div().text_lg().text_color(accent_purple).child("🥧 App Distribution"))
                                                .child(self.render_pie_chart(&stats))
                                        )
                                        // Category Breakdown - Circular Pie Chart
                                        .child(
                                            v_flex()
                                                .flex_1()
                                                .bg(bg_surface)
                                                .rounded_lg()
                                                .p_4()
                                                .border_1()
                                                .border_color(accent_brown)
                                                .gap_4()
                                                .child(div().text_lg().text_color(accent_blue).child("🏷️ Category Distribution"))
                                                .child(self.render_circular_pie_chart(&stats))
                                        )
                                )
                                // TIMELINE
                                .child(
                                    v_flex()
                                        .w_full()
                                        .bg(bg_surface)
                                        .rounded_lg()
                                        .p_4()
                                        .border_1()
                                        .border_color(accent_brown)
                                        .gap_4()
                                        .child(div().text_lg().text_color(accent_green).child("📈 Activity Timeline"))
                                        .child(
                                            v_flex()
                                                .w_full()
                                                .scrollable(gpui::Axis::Horizontal)
                                                .child(self.render_timeline())
                                        )
                                )
                                // Spacer to fill remaining vertical space
                                .child(div().flex_1())
                                .into_any_element()
                        }
                    )
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let debug_enabled = env::var("DEBUG_LOGS_ENABLED")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    if debug_enabled {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("gui.log")
            .expect("Failed to open log file");

        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("neura_hustle_tracker=debug")
        )
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    } else {
        env_logger::Builder::from_env(
            env_logger::Env::default()
                .default_filter_or("off")
        )
        .init();
    }

    let settings = Settings::new()?;

    let database = match Database::new(&settings.database_url).await {
        Ok(db) => {
            log::info!("Database connection successful");
            db
        }
        Err(e) => {
            log::error!("Database connection failed: {}", e);
            return Err(e);
        }
    };

    let gui_app = Arc::new(GuiApp::new(Arc::new(database)));

    if let Err(e) = gui_app.load_all_stats().await {
        log::warn!("Failed to load stats: {}", e);
    }

    log::info!("Starting GPUI Application");

    // Spawn input monitoring for activity tracking (rdev) FIRST
    let last_input_gui = Arc::new(Mutex::new(chrono::Local::now()));
    let last_input_monitor = Arc::clone(&last_input_gui);
    std::thread::spawn(move || {
        let callback = move |event: rdev::Event| {
            match event.event_type {
                rdev::EventType::KeyPress(_)
                | rdev::EventType::KeyRelease(_)
                | rdev::EventType::ButtonPress(_)
                | rdev::EventType::ButtonRelease(_)
                | rdev::EventType::MouseMove { .. } => {
                    *last_input_monitor.lock().unwrap() = chrono::Local::now();
                }
                _ => {}
            }
        };
        if let Err(error) = rdev::listen(callback) {
            log::error!("Error listening for input events: {:?}", error);
        }
    });

    // Spawn background monitoring task for active app tracking
    // Uses the same TUI tracking logic but integrated with GPUI
    let gui_app_monitor = gui_app.clone();
    let gui_app_ui_update = gui_app.clone();
    let last_input_tracking = Arc::clone(&last_input_gui);

    tokio::spawn(async move {
        let monitor = AppMonitor::new();
        log::info!("AppMonitor created - uses_wayland: {}", monitor.uses_wayland());
        let database = gui_app_monitor.state.read().await.database.clone();
        drop(gui_app_monitor);

        let mut last_app: Option<String> = None;
        let mut current_session: Option<crate::models::session::Session> = None;
        let mut last_afk_check = std::time::Instant::now();

        let afk_check_interval = std::time::Duration::from_secs(1);
        let afk_threshold = std::time::Duration::from_secs(300); // 5 minutes
        let polling_interval = Duration::from_millis(500);

        // START TRACKING WITH INITIAL APP (like TUI does)
        if let Ok(initial_app) = monitor.get_active_app_async().await {
            let window_name = monitor.get_active_window_name_async().await.ok();
            log::info!("Starting with initial app: {} (window: {:?})", initial_app, window_name);
            let session = Session {
                id: None,
                app_name: initial_app.clone(),
                window_name: window_name.clone(),
                start_time: chrono::Local::now(),
                duration: 0,
                category: categorize_app(&initial_app),
                browser_url: None,
                browser_page_title: None,
                browser_notification_count: None,
                browser_page_title_renamed: None,
                browser_page_title_category: None,
                terminal_username: None,
                terminal_hostname: None,
                terminal_directory: None,
                terminal_project_name: None,
                terminal_directory_renamed: None,
                terminal_directory_category: None,
                editor_filename: None,
                editor_filepath: None,
                editor_project_path: None,
                editor_language: None,
                editor_filename_renamed: None,
                editor_filename_category: None,
                tmux_window_name: None,
                tmux_pane_count: None,
                terminal_multiplexer: None,
                tmux_window_name_renamed: None,
                tmux_window_name_category: None,
                ide_project_name: None,
                ide_file_open: None,
                ide_workspace: None,
                parsed_data: None,
                parsing_success: Some(false),
                is_afk: Some(false),
                is_idle: Some(false),
                idle_accumulation_secs: Some(0),
            };
            current_session = Some(session.clone());
            last_app = Some(initial_app.clone());

            // Update GuiApp state so UI shows initial session
            if let Ok(mut state) = gui_app_ui_update.state.try_write() {
                state.current_session = Some(crate::gui::app::CurrentSessionInfo {
                    app_name: session.app_name.clone(),
                    window_name: session.window_name.clone(),
                    is_afk: session.is_afk.unwrap_or(false),
                });
                state.is_tracking = true;
            }

            log::info!("Started initial tracking session: {}", initial_app);
        } else {
            log::info!("Failed to detect initial app!");
        }

        loop {
            // AFK detection check (every second)
            if last_afk_check.elapsed() >= afk_check_interval {
                let idle_duration = chrono::Local::now()
                    .signed_duration_since(*last_input_tracking.lock().unwrap());
                let is_currently_afk = idle_duration.num_seconds() >= afk_threshold.as_secs() as i64;

                // Handle AFK state change if we have a session
                if let Some(ref mut session) = current_session {
                    let was_afk = session.is_afk.unwrap_or(false);
                    if was_afk != is_currently_afk {
                        // Save current session
                        session.duration = chrono::Local::now()
                            .signed_duration_since(session.start_time)
                            .num_seconds();

                        if let Err(e) = database.insert_session(&session).await {
                            log::info!(" Failed to save AFK session: {}", e);
                        } else {
                            log::info!(" Session saved on AFK state change: {} (is_afk: {})",
                                      session.app_name, is_currently_afk);
                        }

                        // Start new session with updated AFK state
                        let new_app = if is_currently_afk {
                            "AFK".to_string()
                        } else {
                            match monitor.get_active_app_async().await {
                                Ok(app) => app,
                                Err(_) => "Unknown".to_string(),
                            }
                        };

                        let window_name = monitor.get_active_window_name_async().await.ok();
                        let new_session = Session {
                            id: None,
                            app_name: new_app.clone(),
                            window_name,
                            start_time: chrono::Local::now(),
                            duration: 0,
                            category: categorize_app(&new_app),
                            browser_url: None,
                            browser_page_title: None,
                            browser_notification_count: None,
                            browser_page_title_renamed: None,
                            browser_page_title_category: None,
                            terminal_username: None,
                            terminal_hostname: None,
                            terminal_directory: None,
                            terminal_project_name: None,
                            terminal_directory_renamed: None,
                            terminal_directory_category: None,
                            editor_filename: None,
                            editor_filepath: None,
                            editor_project_path: None,
                            editor_language: None,
                            editor_filename_renamed: None,
                            editor_filename_category: None,
                            tmux_window_name: None,
                            tmux_pane_count: None,
                            terminal_multiplexer: None,
                            tmux_window_name_renamed: None,
                            tmux_window_name_category: None,
                            ide_project_name: None,
                            ide_file_open: None,
                            ide_workspace: None,
                            parsed_data: None,
                            parsing_success: Some(false),
                            is_afk: Some(is_currently_afk),
                            is_idle: Some(false),
                            idle_accumulation_secs: Some(0),
                        };

                        current_session = Some(new_session);
                    }
                }

                last_afk_check = std::time::Instant::now();
            }

            // App change detection
            match monitor.get_active_app_async().await {
                Ok(current_app) => {
                    let current_window = monitor.get_active_window_name_async().await.ok();

                    // Check if app OR window changed (window = tab in Firefox, etc)
                    if last_app.as_ref() != Some(&current_app) || (current_session.is_some() && current_session.as_ref().map(|s| &s.window_name) != Some(&current_window)) {
                        log::info!("App/window changed: {} -> {} (window: {:?})",
                                  last_app.as_ref().unwrap_or(&"None".to_string()),
                                  current_app,
                                  current_window);

                        // Save previous session if any
                        if let Some(mut session) = current_session.take() {
                            session.duration = chrono::Local::now()
                                .signed_duration_since(session.start_time)
                                .num_seconds();

                            if let Err(e) = database.insert_session(&session).await {
                                log::warn!("Failed to save session: {}", e);
                            } else {
                                log::info!("Session saved: {} ({}s) window: {:?}", session.app_name, session.duration, session.window_name);
                            }
                        }

                        // Start new session
                        let window_name = current_window;
                        let is_afk = {
                            let idle_duration = chrono::Local::now()
                                .signed_duration_since(*last_input_tracking.lock().unwrap());
                            idle_duration.num_seconds() >= afk_threshold.as_secs() as i64
                        };

                        let new_session = Session {
                            id: None,
                            app_name: current_app.clone(),
                            window_name,
                            start_time: chrono::Local::now(),
                            duration: 0,
                            category: categorize_app(&current_app),
                            browser_url: None,
                            browser_page_title: None,
                            browser_notification_count: None,
                            browser_page_title_renamed: None,
                            browser_page_title_category: None,
                            terminal_username: None,
                            terminal_hostname: None,
                            terminal_directory: None,
                            terminal_project_name: None,
                            terminal_directory_renamed: None,
                            terminal_directory_category: None,
                            editor_filename: None,
                            editor_filepath: None,
                            editor_project_path: None,
                            editor_language: None,
                            editor_filename_renamed: None,
                            editor_filename_category: None,
                            tmux_window_name: None,
                            tmux_pane_count: None,
                            terminal_multiplexer: None,
                            tmux_window_name_renamed: None,
                            tmux_window_name_category: None,
                            ide_project_name: None,
                            ide_file_open: None,
                            ide_workspace: None,
                            parsed_data: None,
                            parsing_success: Some(false),
                            is_afk: Some(is_afk),
                            is_idle: Some(false),
                            idle_accumulation_secs: Some(0),
                        };

                        current_session = Some(new_session.clone());
                        last_app = Some(current_app.clone());

                        // Update GuiApp state so UI shows current session
                        if let Ok(mut state) = gui_app_ui_update.state.try_write() {
                            state.current_session = Some(crate::gui::app::CurrentSessionInfo {
                                app_name: new_session.app_name.clone(),
                                window_name: new_session.window_name.clone(),
                                is_afk: new_session.is_afk.unwrap_or(false),
                            });
                            state.is_tracking = true;
                        }
                    }
                }
                Err(e) => {
                    log::debug!("Failed to detect active app: {}", e);
                }
            }

            sleep(polling_interval).await;
        }
    });

    // Spawn periodic stats reload task - reload ALL stats every 5 seconds (like TUI does)
    let gui_app_reload = gui_app.clone();
    tokio::spawn(async move {
        let reload_interval = Duration::from_secs(5);
        loop {
            sleep(reload_interval).await;
            if let Err(e) = gui_app_reload.load_all_stats().await {
                log::warn!("Failed to reload stats: {}", e);
            }
        }
    });

    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        let gui_app_clone = gui_app.clone();

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| {
                    DashboardView::new(gui_app_clone.clone())
                });
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });

    Ok(())
}
