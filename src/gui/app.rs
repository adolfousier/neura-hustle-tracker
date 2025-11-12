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
}

impl Default for TimePeriodStats {
    fn default() -> Self {
        Self {
            total_duration: 0,
            app_durations: vec![],
            category_durations: vec![],
        }
    }
}

#[derive(Clone)]
pub struct SessionRecord {
    pub start_time: String,
    pub app_name: String,
    pub duration: i64,
}

pub struct GuiAppState {
    pub database: Arc<Database>,
    pub daily_stats: TimePeriodStats,
    pub weekly_stats: TimePeriodStats,
    pub monthly_stats: TimePeriodStats,
    pub session_history: Vec<SessionRecord>,
    pub all_categories: Vec<String>,
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

        let daily = self.fetch_period_stats(database.get_daily_sessions().await?).await?;
        let weekly = self.fetch_period_stats(database.get_weekly_sessions().await?).await?;
        let monthly = self.fetch_period_stats(database.get_monthly_sessions().await?).await?;
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

    async fn fetch_period_stats(&self, sessions: Vec<Session>) -> anyhow::Result<TimePeriodStats> {

        let mut app_durations: HashMap<String, i64> = HashMap::new();
        let mut category_durations: HashMap<String, i64> = HashMap::new();
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
        }

        let mut app_list: Vec<_> = app_durations.into_iter().collect();
        app_list.sort_by(|a, b| b.1.cmp(&a.1));

        let mut category_list: Vec<_> = category_durations.into_iter().collect();
        category_list.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(TimePeriodStats {
            total_duration,
            app_durations: app_list,
            category_durations: category_list,
        })
    }

    async fn fetch_session_history(&self, database: &Database) -> anyhow::Result<Vec<SessionRecord>> {
        let sessions = database.get_daily_sessions().await?;
        let mut records: Vec<SessionRecord> = sessions
            .into_iter()
            .map(|s| SessionRecord {
                start_time: s.start_time.format("%H:%M").to_string(),
                app_name: s.app_name.clone(),
                duration: s.duration,
            })
            .collect();

        records.reverse();
        Ok(records)
    }

}
