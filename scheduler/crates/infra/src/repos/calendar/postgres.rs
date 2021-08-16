use super::ICalendarRepo;
use crate::repos::shared::query_structs::{extract_metadata, to_metadata, MetadataFindQuery};
use nettu_scheduler_domain::{Calendar, ID};
use sqlx::{
    types::{Json, Uuid},
    Done, FromRow, PgPool,
};
use tracing::error;

pub struct PostgresCalendarRepo {
    pool: PgPool,
}

impl PostgresCalendarRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct CalendarRaw {
    calendar_uid: Uuid,
    user_uid: Uuid,
    account_uid: Uuid,
    settings: serde_json::Value,
    metadata: Vec<String>,
}

impl From<CalendarRaw> for Calendar {
    fn from(e: CalendarRaw) -> Self {
        Self {
            id: e.calendar_uid.into(),
            user_id: e.user_uid.into(),
            account_id: e.account_uid.into(),
            settings: serde_json::from_value(e.settings).unwrap(),
            metadata: extract_metadata(e.metadata),
        }
    }
}

#[async_trait::async_trait]
impl ICalendarRepo for PostgresCalendarRepo {
    async fn insert(&self, calendar: &Calendar) -> anyhow::Result<()> {
        let metadata = to_metadata(calendar.metadata.clone());
        sqlx::query!(
            r#"
            INSERT INTO calendars(calendar_uid, user_uid, settings, metadata)
            VALUES($1, $2, $3, $4)
            "#,
            calendar.id.inner_ref(),
            calendar.user_id.inner_ref(),
            Json(&calendar.settings) as _,
            &metadata
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn save(&self, calendar: &Calendar) -> anyhow::Result<()> {
        let metadata = to_metadata(calendar.metadata.clone());
        sqlx::query!(
            r#"
            UPDATE calendars
            SET settings = $2,
            metadata = $3
            WHERE calendar_uid = $1
            "#,
            calendar.id.inner_ref(),
            Json(&calendar.settings) as _,
            &metadata
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Unable to update calendar: {:?}", e);
            e
        })?
        .rows_affected();
        Ok(())
    }

    async fn find(&self, calendar_id: &ID) -> Option<Calendar> {
        let calendar: CalendarRaw = match sqlx::query_as!(
            CalendarRaw,
            r#"
            SELECT c.*, u.account_uid FROM calendars AS c
            INNER JOIN users AS u
                ON u.user_uid = c.user_uid
            WHERE c.calendar_uid = $1
            "#,
            calendar_id.inner_ref(),
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(cal) => cal,
            Err(_) => return None,
        };
        Some(calendar.into())
    }

    async fn find_by_user(&self, user_id: &ID) -> Vec<Calendar> {
        let calendars: Vec<CalendarRaw> = sqlx::query_as!(
            CalendarRaw,
            r#"
            SELECT c.*, u.account_uid FROM calendars AS c
            INNER JOIN users AS u
                ON u.user_uid = c.user_uid
            WHERE c.user_uid = $1
            "#,
            user_id.inner_ref(),
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        calendars.into_iter().map(|c| c.into()).collect()
    }

    async fn delete(&self, calendar_id: &ID) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM calendars AS c
            WHERE c.calendar_uid = $1
            "#,
            calendar_id.inner_ref(),
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(anyhow::Error::new)
    }

    async fn find_by_metadata(&self, query: MetadataFindQuery) -> Vec<Calendar> {
        let key = format!("{}_{}", query.metadata.key, query.metadata.value);

        let calendars: Vec<CalendarRaw> = sqlx::query_as!(
            CalendarRaw,
            r#"
            SELECT c.*, u.account_uid FROM calendars AS c
            INNER JOIN users AS u
                ON u.user_uid = c.user_uid
            WHERE u.account_uid = $1 AND c.metadata @> ARRAY[$2]
            LIMIT $3
            OFFSET $4
            "#,
            query.account_id.inner_ref(),
            key,
            query.limit as i64,
            query.skip as i64,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        calendars.into_iter().map(|c| c.into()).collect()
    }
}
