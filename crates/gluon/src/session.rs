use async_trait::async_trait;
use time::OffsetDateTime;
use tower_sessions::SessionStore;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{self, ExpiredDeletion};

pub use tower_sessions::Session;

/// PostgreSQL-backed session store shared safely by multiple application
/// processes using the same database.
#[derive(Clone, Debug)]
pub struct PostgresSessionStore {
    pool: sqlx::PgPool,
}

impl PostgresSessionStore {
    #[must_use]
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Creates the session table when absent.
    ///
    /// # Errors
    ///
    /// Returns a database error when the migration cannot be applied.
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS gluon_sessions (\
                id TEXT PRIMARY KEY, \
                data TEXT NOT NULL, \
                expiry TIMESTAMPTZ NOT NULL\
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        self.delete_expired().await?;
        loop {
            let data = serde_json::to_string(&record.data)
                .map_err(|error| session_store::Error::Encode(error.to_string()))?;
            let result = sqlx::query(
                "INSERT INTO gluon_sessions (id, data, expiry) VALUES ($1, $2, $3) \
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(record.id.to_string())
            .bind(data)
            .bind(record.expiry_date)
            .execute(&self.pool)
            .await
            .map_err(|error| session_store::Error::Backend(error.to_string()))?;
            if result.rows_affected() == 1 {
                return Ok(());
            }
            record.id = Id::default();
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let data = serde_json::to_string(&record.data)
            .map_err(|error| session_store::Error::Encode(error.to_string()))?;
        sqlx::query(
            "INSERT INTO gluon_sessions (id, data, expiry) VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, expiry = EXCLUDED.expiry",
        )
        .bind(record.id.to_string())
        .bind(data)
        .bind(record.expiry_date)
        .execute(&self.pool)
        .await
        .map_err(|error| session_store::Error::Backend(error.to_string()))?;
        Ok(())
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        let row = sqlx::query_as::<_, (String, OffsetDateTime)>(
            "SELECT data, expiry FROM gluon_sessions \
             WHERE id = $1 AND expiry > $2",
        )
        .bind(id.to_string())
        .bind(OffsetDateTime::now_utc())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| session_store::Error::Backend(error.to_string()))?;
        let Some((data, expiry)) = row else {
            return Ok(None);
        };
        let data = serde_json::from_str(&data)
            .map_err(|error| session_store::Error::Decode(error.to_string()))?;
        Ok(Some(Record {
            id: *id,
            data,
            expiry_date: expiry,
        }))
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        sqlx::query("DELETE FROM gluon_sessions WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|error| session_store::Error::Backend(error.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for PostgresSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        sqlx::query("DELETE FROM gluon_sessions WHERE expiry <= $1")
            .bind(OffsetDateTime::now_utc())
            .execute(&self.pool)
            .await
            .map_err(|error| session_store::Error::Backend(error.to_string()))?;
        Ok(())
    }
}
