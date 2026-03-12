use chrono::{DateTime, Utc, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScheduleType {
    Cron(String),
    Interval(u64),
    OneShot(DateTime<Utc>),
    Event(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub name: String,
    pub hand_id: String,
    pub schedule_type: ScheduleType,
    pub enabled: bool,
    pub timezone: String,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub event_triggers: Vec<String>,
}

impl Schedule {
    pub fn new(id: String, name: String, hand_id: String, schedule_type: ScheduleType) -> Self {
        let next_run = Self::calculate_next_run(&schedule_type, Utc::now());
        Self {
            id,
            name,
            hand_id,
            schedule_type,
            enabled: true,
            timezone: "UTC".to_string(),
            last_run: None,
            next_run,
            created_at: Utc::now(),
            event_triggers: vec![],
        }
    }

    fn calculate_next_run(schedule_type: &ScheduleType, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match schedule_type {
            ScheduleType::Cron(cron) => Self::parse_cron_next(cron, now),
            ScheduleType::Interval(seconds) => Some(now + chrono::Duration::seconds(*seconds as i64)),
            ScheduleType::OneShot(datetime) => {
                if *datetime > now {
                    Some(*datetime)
                } else {
                    None
                }
            }
            ScheduleType::Event(_) => None,
        }
    }

    fn parse_cron_next(cron: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let parts: Vec<&str> = cron.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }

        let (minute, hour, day, month, weekday) = (
            parts[0], parts[1], parts[2], parts[3], parts[4],
        );

        let mut next = now + chrono::Duration::minutes(1);
        for _ in 0..24 * 60 {
            if Self::matches_cron_part(minute, next.format("%M").to_string().parse::<u32>().unwrap_or(0))
                && Self::matches_cron_part(hour, next.format("%H").to_string().parse::<u32>().unwrap_or(0))
                && Self::matches_cron_part(day, next.format("%d").to_string().parse::<u32>().unwrap_or(0))
                && Self::matches_cron_part(month, next.format("%m").to_string().parse::<u32>().unwrap_or(0))
                && Self::matches_cron_part(weekday, next.format("%u").to_string().parse::<u32>().unwrap_or(1).saturating_sub(1))
            {
                return Some(next);
            }
            next += chrono::Duration::minutes(1);
        }
        None
    }

    fn matches_cron_part(part: &str, value: u32) -> bool {
        if part == "*" {
            return true;
        }
        if let Ok(v) = part.parse::<u32>() {
            return v == value;
        }
        if part.contains(',') {
            return part.split(',').any(|p| {
                p.trim().parse::<u32>().map(|v| v == value).unwrap_or(false)
            });
        }
        false
    }

    pub fn should_run(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(next) = self.next_run {
            return now >= next;
        }
        false
    }

    pub fn update_next_run(&mut self) {
        let now = Utc::now();
        self.last_run = Some(now);
        self.next_run = Self::calculate_next_run(&self.schedule_type, now);
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleEvent {
    pub schedule_id: String,
    pub hand_id: String,
    pub timestamp: DateTime<Utc>,
}

pub struct ScheduleManager {
    schedules: Arc<RwLock<HashMap<String, Schedule>>>,
    event_tx: tokio::sync::mpsc::Sender<ScheduleEvent>,
}

impl ScheduleManager {
    pub fn new() -> Self {
        let (event_tx, _) = tokio::sync::mpsc::channel(100);
        Self {
            schedules: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    pub fn with_channel(event_tx: tokio::sync::mpsc::Sender<ScheduleEvent>) -> Self {
        Self {
            schedules: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    pub async fn add_schedule(&self, schedule: Schedule) {
        let mut schedules = self.schedules.write().await;
        schedules.insert(schedule.id.clone(), schedule);
    }

    pub async fn remove_schedule(&self, id: &str) -> Option<Schedule> {
        let mut schedules = self.schedules.write().await;
        schedules.remove(id)
    }

    pub async fn enable(&self, id: &str) -> bool {
        let mut schedules = self.schedules.write().await;
        if let Some(schedule) = schedules.get_mut(id) {
            schedule.enabled = true;
            schedule.next_run = Schedule::calculate_next_run(&schedule.schedule_type, Utc::now());
            true
        } else {
            false
        }
    }

    pub async fn disable(&self, id: &str) -> bool {
        let mut schedules = self.schedules.write().await;
        if let Some(schedule) = schedules.get_mut(id) {
            schedule.enabled = false;
            true
        } else {
            false
        }
    }

    pub async fn list(&self) -> Vec<Schedule> {
        let schedules = self.schedules.read().await;
        schedules.values().cloned().collect()
    }

    pub async fn list_enabled(&self) -> Vec<Schedule> {
        let schedules = self.schedules.read().await;
        schedules.values().filter(|s| s.enabled).cloned().collect()
    }

    pub async fn get(&self, id: &str) -> Option<Schedule> {
        let schedules = self.schedules.read().await;
        schedules.get(id).cloned()
    }

    pub async fn get_next_run(&self, id: &str) -> Option<DateTime<Utc>> {
        let schedules = self.schedules.read().await;
        schedules.get(id).and_then(|s| s.next_run)
    }

    pub fn get_event_receiver(&self) -> tokio::sync::mpsc::Receiver<ScheduleEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let _ = tx.clone();
        rx
    }

    pub async fn start_background_loop(&self) {
        let schedules = self.schedules.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));
            loop {
                ticker.tick().await;
                let now = Utc::now();
                let to_run: Vec<(String, String)> = {
                    let schedules = schedules.read().await;
                    schedules
                        .values()
                        .filter(|s| s.should_run(now))
                        .map(|s| (s.id.clone(), s.hand_id.clone()))
                        .collect()
                };

                for (schedule_id, hand_id) in to_run {
                    let event = ScheduleEvent {
                        schedule_id: schedule_id.clone(),
                        hand_id: hand_id.clone(),
                        timestamp: now,
                    };
                    let _ = event_tx.send(event).await;

                    let mut schedules = schedules.write().await;
                    if let Some(schedule) = schedules.get_mut(&schedule_id) {
                        schedule.update_next_run();
                    }
                }
            }
        });
    }
}

impl Default for ScheduleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_creation() {
        let schedule = Schedule::new(
            "test_1".to_string(),
            "Test Schedule".to_string(),
            "researcher".to_string(),
            ScheduleType::Interval(3600),
        );

        assert_eq!(schedule.name, "Test Schedule");
        assert_eq!(schedule.hand_id, "researcher");
        assert!(schedule.enabled);
        assert!(schedule.next_run.is_some());
    }

    #[test]
    fn test_cron_parsing() {
        let schedule = Schedule::new(
            "test_2".to_string(),
            "Cron Test".to_string(),
            "collector".to_string(),
            ScheduleType::Cron("0 6 * * *".to_string()),
        );

        assert!(schedule.next_run.is_some());
    }

    #[tokio::test]
    async fn test_add_remove_schedule() {
        let manager = ScheduleManager::new();
        let schedule = Schedule::new(
            "test_3".to_string(),
            "Test".to_string(),
            "lead".to_string(),
            ScheduleType::Interval(60),
        );

        manager.add_schedule(schedule).await;
        let retrieved = manager.get("test_3").await;
        assert!(retrieved.is_some());

        manager.remove_schedule("test_3").await;
        let removed = manager.get("test_3").await;
        assert!(removed.is_none());
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let manager = ScheduleManager::new();
        let schedule = Schedule::new(
            "test_4".to_string(),
            "Test".to_string(),
            "researcher".to_string(),
            ScheduleType::Interval(60),
        );

        manager.add_schedule(schedule).await;
        assert!(manager.disable("test_4").await);

        let disabled = manager.get("test_4").await.unwrap();
        assert!(!disabled.enabled);

        assert!(manager.enable("test_4").await);
        let enabled = manager.get("test_4").await.unwrap();
        assert!(enabled.enabled);
    }

    #[tokio::test]
    async fn test_list_enabled() {
        let manager = ScheduleManager::new();

        let s1 = Schedule::new("1".to_string(), "S1".to_string(), "h1".to_string(), ScheduleType::Interval(60));
        let s2 = Schedule::new("2".to_string(), "S2".to_string(), "h2".to_string(), ScheduleType::Interval(60));

        manager.add_schedule(s1).await;
        manager.add_schedule(s2).await;
        manager.disable("1").await;

        let enabled = manager.list_enabled().await;
        assert_eq!(enabled.len(), 1);
    }
}
