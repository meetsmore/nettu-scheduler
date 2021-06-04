use super::IReminderRepo;
use crate::repos::shared::{query_structs::MetadataFindQuery, repo::DeleteResult};
use nettu_scheduler_domain::{
    Calendar, CalendarEvent, EventRemindersExpansionJob, Metadata, Reminder, ID,
};
use sqlx::{types::Uuid, Done, FromRow, PgPool};

pub struct PostgresReminderRepo {
    pool: PgPool,
}

impl PostgresReminderRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct ReminderRaw {
    reminder_uid: Uuid,
    event_uid: Uuid,
    account_uid: Uuid,
    remind_at: i64,
    priority: i16,
}

impl Into<Reminder> for ReminderRaw {
    fn into(self) -> Reminder {
        Reminder {
            id: Default::default(),
            event_id: Default::default(),
            account_id: Default::default(),
            remind_at: self.remind_at,
            priority: self.priority as i64,
        }
    }
}

#[async_trait::async_trait]
impl IReminderRepo for PostgresReminderRepo {
    async fn bulk_insert(&self, reminders: &[Reminder]) -> anyhow::Result<()> {
        for reminder in reminders {
            let id = Uuid::new_v4();
            let event_id = Uuid::new_v4();
            let account_id = Uuid::new_v4();
            sqlx::query!(
                r#"
            INSERT INTO reminders 
            (reminder_uid, event_uid, account_uid, remind_at, priority)
            VALUES($1, $2, $3, $4, $5)
            "#,
                id,
                event_id,
                account_id,
                reminder.remind_at,
                reminder.priority as i16
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn delete_all_before(&self, before: i64) -> Vec<Reminder> {
        sqlx::query_as!(
            ReminderRaw,
            r#"
            DELETE FROM reminders AS r
            WHERE r.remind_at <= $1
            RETURNING *
            "#,
            before,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or(vec![])
        .into_iter()
        .map(|reminder| reminder.into())
        .collect()
    }

    async fn delete_by_events(&self, event_ids: &[ID]) -> anyhow::Result<DeleteResult> {
        let ids = vec![Uuid::new_v4()];
        let res = sqlx::query!(
            r#"
            DELETE FROM reminders AS r
            WHERE r.event_uid = ANY($1)
            "#,
            &ids,
        )
        .execute(&self.pool)
        .await?;
        Ok(DeleteResult {
            deleted_count: res.rows_affected() as i64,
        })
    }

    async fn find_by_event_and_priority(&self, event_id: &ID, priority: i64) -> Option<Reminder> {
        let event_id = Uuid::new_v4();
        match sqlx::query_as!(
            ReminderRaw,
            r#"
            SELECT * FROM reminders AS r
            WHERE r.event_uid = $1 AND
            r.priority = $2
            "#,
            event_id,
            priority as i16
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(reminder) => Some(reminder.into()),
            Err(_) => None,
        }
    }
}
