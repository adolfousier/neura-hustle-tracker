use crate::database::connection::Database;
use crate::models::session::Session;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TimePeriodStats {
    pub total_duration: i64,
    pub app_durations: Vec<(String, i64)>,
    pub category_durations: Vec<(String, i64)>,
    pub browser_breakdown: Vec<(String, i64)>,
    pub project_breakdown: Vec<(String, i64)>,
    pub terminal_breakdown: Vec<(String, i64)>,
}

impl Default for TimePeriodStats {
    fn default() -> Self {
        Self {
            total_duration: 0,
            app_durations: vec![],
            category_durations: vec![],
            browser_breakdown: vec![],
            project_breakdown: vec![],
            terminal_breakdown: vec![],
        }
    }
}

#[derive(Clone)]
pub struct SessionRecord {
    pub start_time: String,
    pub app_name: String,
    pub window_name: Option<String>,
    pub duration: i64,
    pub category: Option<String>,
}

#[derive(Clone)]
pub struct CurrentSessionInfo {
    pub app_name: String,
    pub window_name: Option<String>,
    pub is_afk: bool,
}

pub struct GuiAppState {
    pub database: Arc<Database>,
    pub daily_stats: TimePeriodStats,
    pub weekly_stats: TimePeriodStats,
    pub monthly_stats: TimePeriodStats,
    pub session_history: Vec<SessionRecord>,
    pub all_categories: Vec<String>,
    pub current_session: Option<CurrentSessionInfo>,
    pub is_tracking: bool,
}

impl GuiAppState {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            daily_stats: TimePeriodStats::default(),
            weekly_stats: TimePeriodStats::default(),
            monthly_stats: TimePeriodStats::default(),
            session_history: vec![],
            all_categories: vec![],
            current_session: None,
            is_tracking: true,  // GUI always tracks in background
        }
    }
}

pub struct GuiApp {
    pub state: Arc<RwLock<GuiAppState>>,
}

impl GuiApp {
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            state: Arc::new(RwLock::new(GuiAppState::new(database))),
        }
    }

    pub async fn load_all_stats(&self) -> anyhow::Result<()> {
        let state = self.state.read().await;
        let database = state.database.clone();
        drop(state);

        let daily_sessions = database.get_daily_sessions().await?;
        let weekly_sessions = database.get_weekly_sessions().await?;
        let monthly_sessions = database.get_monthly_sessions().await?;

        let daily = self.fetch_period_stats_with_breakdown(daily_sessions).await?;
        let weekly = self.fetch_period_stats_with_breakdown(weekly_sessions).await?;
        let monthly = self.fetch_period_stats_with_breakdown(monthly_sessions).await?;

        let session_history = self.fetch_session_history(&database).await?;
        let all_categories = database.get_custom_categories().await.unwrap_or_default();

        let mut state = self.state.write().await;
        state.daily_stats = daily;
        state.weekly_stats = weekly;
        state.monthly_stats = monthly;
        state.session_history = session_history;
        state.all_categories = all_categories;

        Ok(())
    }

    async fn fetch_period_stats_with_breakdown(&self, sessions: Vec<Session>) -> anyhow::Result<TimePeriodStats> {
        let mut app_durations: HashMap<String, i64> = HashMap::new();
        let mut category_durations: HashMap<String, i64> = HashMap::new();
        let mut browser_breakdown: HashMap<String, i64> = HashMap::new();
        let mut project_breakdown: HashMap<String, i64> = HashMap::new();
        let mut terminal_breakdown: HashMap<String, i64> = HashMap::new();
        let mut total_duration = 0i64;

        for session in sessions {
            total_duration += session.duration;
            let app_name = session.app_name.clone();
            app_durations
                .entry(app_name)
                .and_modify(|d| *d += session.duration)
                .or_insert(session.duration);

            if let Some(category) = &session.category {
                category_durations
                    .entry(category.clone())
                    .and_modify(|d| *d += session.duration)
                    .or_insert(session.duration);
            }

            // Aggregate breakdown data
            if let Some(title) = &session.browser_page_title {
                browser_breakdown
                    .entry(title.clone())
                    .and_modify(|d| *d += session.duration)
                    .or_insert(session.duration);
            }

            if let Some(project) = &session.terminal_project_name {
                project_breakdown
                    .entry(project.clone())
                    .and_modify(|d| *d += session.duration)
                    .or_insert(session.duration);
            }

            if let Some(dir) = &session.terminal_directory {
                terminal_breakdown
                    .entry(dir.clone())
                    .and_modify(|d| *d += session.duration)
                    .or_insert(session.duration);
            }
        }

        let mut app_list: Vec<_> = app_durations.into_iter().collect();
        app_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut category_list: Vec<_> = category_durations.into_iter().collect();
        category_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut browser_list: Vec<_> = browser_breakdown.into_iter().collect();
        browser_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut project_list: Vec<_> = project_breakdown.into_iter().collect();
        project_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut terminal_list: Vec<_> = terminal_breakdown.into_iter().collect();
        terminal_list.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(TimePeriodStats {
            total_duration,
            app_durations: app_list,
            category_durations: category_list,
            browser_breakdown: browser_list,
            project_breakdown: project_list,
            terminal_breakdown: terminal_list,
        })
    }

    async fn fetch_session_history(&self, database: &Database) -> anyhow::Result<Vec<SessionRecord>> {
        let sessions = database.get_daily_sessions().await?;
        let mut records: Vec<SessionRecord> = sessions
            .into_iter()
            .map(|s| SessionRecord {
                start_time: s.start_time.format("%H:%M").to_string(),
                app_name: s.app_name.clone(),
                window_name: s.window_name.clone(),
                duration: s.duration,
                category: s.category.clone(),
            })
            .collect();

        records.reverse();
        Ok(records)
    }


}
