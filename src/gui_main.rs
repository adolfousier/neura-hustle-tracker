mod config;
mod database;
mod models;
mod gui;

use anyhow::Result;
use crate::config::settings::Settings;
use crate::database::connection::Database;
use crate::gui::app::GuiApp;
use dotenvy::dotenv;
use gpui::*;
use gpui_component::*;
use gpui_component::chart::PieChart;
use gpui_component::button::{Button, ButtonVariants, ButtonCustomVariant};
use std::env;
use std::fs::OpenOptions;
use std::sync::Arc;

struct DashboardView {
    daily_stats: crate::gui::app::TimePeriodStats,
    weekly_stats: crate::gui::app::TimePeriodStats,
    monthly_stats: crate::gui::app::TimePeriodStats,
    session_history: Vec<crate::gui::app::SessionRecord>,
    current_view: ViewMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Daily,
    Weekly,
    Monthly,
    Sessions,
}

impl DashboardView {
    fn new(
        daily_stats: crate::gui::app::TimePeriodStats,
        weekly_stats: crate::gui::app::TimePeriodStats,
        monthly_stats: crate::gui::app::TimePeriodStats,
        session_history: Vec<crate::gui::app::SessionRecord>,
    ) -> Self {
        Self {
            daily_stats,
            weekly_stats,
            monthly_stats,
            session_history,
            current_view: ViewMode::Daily,
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

    fn get_current_stats(&self) -> &crate::gui::app::TimePeriodStats {
        match self.current_view {
            ViewMode::Daily => &self.daily_stats,
            ViewMode::Weekly => &self.weekly_stats,
            ViewMode::Monthly => &self.monthly_stats,
            ViewMode::Sessions => &self.daily_stats,
        }
    }

    fn get_view_title(&self) -> &'static str {
        match self.current_view {
            ViewMode::Daily => "Daily Activity",
            ViewMode::Weekly => "Weekly Activity",
            ViewMode::Monthly => "Monthly Activity",
            ViewMode::Sessions => "Session History",
        }
    }

    fn render_timeline(&self, stats: &crate::gui::app::TimePeriodStats) -> impl IntoElement {
        if stats.app_durations.is_empty() || stats.total_duration == 0 {
            return div().child("No activity data").into_any_element();
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

        let time_slots = match self.current_view {
            ViewMode::Daily => 24,      // Hours in a day
            ViewMode::Weekly => 7,      // Days in a week
            ViewMode::Monthly => 30,    // Days in a month
            ViewMode::Sessions => 24,   // Default to daily
        };


        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_1()
                    .items_end()
                    .h_40()
                    .children(
                        (0..time_slots).map(|i| {
                            // Distribute total duration across time slots
                            let slot_duration = stats.total_duration / (time_slots as i64);
                            let variance = ((i as i64 * 7919) % (slot_duration + 1)).max(1);
                            let pct = ((variance as f32) / (stats.total_duration as f32)).max(0.01) * 100.0;

                            div()
                                .flex_1()
                                .h(px((pct * 1.5).max(4.0)))
                                .bg(colors[i % colors.len()])
                                .rounded_sm()
                        })
                    )
            )
            .child({
                // Time labels
                h_flex()
                    .gap_1()
                    .text_xs()
                    .text_color(gpui::rgb(0x8b949e))
                    .child(
                        match self.current_view {
                            ViewMode::Daily => "00h → 23h".to_string(),
                            ViewMode::Weekly => "Mon → Sun".to_string(),
                            ViewMode::Monthly => "Day 1 → Day 30".to_string(),
                            ViewMode::Sessions => "00h → 23h".to_string(),
                        }
                    )
            })
            .into_any_element()
    }

    fn render_circular_pie_chart(&self, stats: &crate::gui::app::TimePeriodStats) -> impl IntoElement {
        if stats.category_durations.is_empty() || stats.total_duration == 0 {
            return div().child("No categories").into_any_element();
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
                                        stats.app_durations.iter().take(8).enumerate().map(|(i, (_, duration))| {
                                            let pct = ((*duration as f32) / (stats.total_duration as f32)) * 100.0;
                                            div()
                                                .w_8()
                                                .h(px(pct * 2.0))
                                                .bg(colors[i % colors.len()])
                                                .rounded_sm()
                                        })
                                    )
                            )
                    })
                    .child({
                        // Legend
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

    fn render_sessions(&self) -> impl IntoElement {
        let text_secondary = gpui::rgb(0x8b949e);
        let bg_surface = gpui::rgb(0x21262d);
        let accent_green = gpui::rgb(0x4ade80);
        let accent_blue = gpui::rgb(0x60a5fa);
        let accent_brown = gpui::rgb(0x8b6914);

        v_flex()
            .w_full()
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
                            .child(div().text_2xl().text_color(accent_blue).child(format!("{}", self.session_history.len())))
                    )
            )
            // SESSIONS LIST CARD - Scrollable content
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
                        self.session_history.iter().take(100).map(|record| {
                            h_flex()
                                .w_full()
                                .gap_3()
                                .p_2()
                                .bg(gpui::rgb(0x0f1117))
                                .rounded_lg()
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
                                        .child(record.app_name.clone())
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
                    .child(div().flex_1())  // Spacer
                    // View Selector Buttons - aligned right with brown theme
                    .child({
                        let darker_brown = gpui::rgb(0x6d5510);
                        let mut btn = Button::new("daily").label("Daily");
                        if self.current_view == ViewMode::Daily {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Daily;
                        }))
                    })
                    .child({
                        let darker_brown = gpui::rgb(0x6d5510);
                        let mut btn = Button::new("weekly").label("Weekly");
                        if self.current_view == ViewMode::Weekly {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Weekly;
                        }))
                    })
                    .child({
                        let darker_brown = gpui::rgb(0x6d5510);
                        let mut btn = Button::new("monthly").label("Monthly");
                        if self.current_view == ViewMode::Monthly {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Monthly;
                        }))
                    })
                    .child({
                        let darker_brown = gpui::rgb(0x6d5510);
                        let mut btn = Button::new("sessions").label("Sessions");
                        if self.current_view == ViewMode::Sessions {
                            btn = btn.outline().text_color(accent_brown).border_color(accent_brown);
                        } else {
                            btn = btn.outline().text_color(text_secondary).border_color(text_secondary);
                        }
                        btn.on_click(cx.listener(|view, _, _, _| {
                            view.current_view = ViewMode::Sessions;
                        }))
                    })
            )
            // MAIN CONTENT
            .child(
                v_flex()
                    .flex_1()
                    .bg(bg_dark)
                    .px_3()
                    .py_2()
                    .gap_1()
                    // CONDITIONAL CONTENT - Show Sessions or Dashboard
                    .child(
                        if self.current_view == ViewMode::Sessions {
                            self.render_sessions().into_any_element()
                        } else {
                            v_flex()
                                .w_full()
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
                                                .child(self.render_pie_chart(stats))
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
                                                .child(self.render_circular_pie_chart(stats))
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
                                        .child(self.render_timeline(stats))
                                )
                                .into_any_element()
                        }
                    )
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

    let state = gui_app.state.read().await;
    let daily_stats = state.daily_stats.clone();
    let weekly_stats = state.weekly_stats.clone();
    let monthly_stats = state.monthly_stats.clone();
    let session_history = state.session_history.clone();
    drop(state);

    log::info!("Starting GPUI Application");

    let app = Application::new();

    app.run(move |cx| {
        gpui_component::init(cx);

        let daily_stats_clone = daily_stats.clone();
        let weekly_stats_clone = weekly_stats.clone();
        let monthly_stats_clone = monthly_stats.clone();
        let session_history_clone = session_history.clone();

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| {
                    DashboardView::new(
                        daily_stats_clone.clone(),
                        weekly_stats_clone.clone(),
                        monthly_stats_clone.clone(),
                        session_history_clone.clone(),
                    )
                });
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });

    Ok(())
}
