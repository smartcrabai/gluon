//! Atomic shared storage for all authentication protocols.
//!
//! Operations must contain only local, synchronous work. In particular, do not
//! perform delivery or provider requests while holding a transaction. An operation
//! error does NOT roll back its mutations: failed-attempt counters and consumed
//! flows must survive rejected authentication attempts.
use super::{AuthError, AuthUser, UserId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredUser {
    pub user: AuthUser,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredChallenge {
    pub kind: String,
    pub binding: String,
    pub user: Option<String>,
    pub expires: OffsetDateTime,
    pub data: Value,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredPasskey {
    pub user: String,
    pub data: Value,
    pub version: u64,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredRate {
    pub count: u32,
    pub expires: OffsetDateTime,
}
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StoreData {
    pub users: HashMap<String, StoredUser>,
    pub identities: HashMap<String, String>,
    pub challenges: HashMap<String, StoredChallenge>,
    pub passkeys: HashMap<String, StoredPasskey>,
    pub rates: HashMap<String, StoredRate>,
    #[serde(default)]
    pub logins: HashMap<String, StoredLogin>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredLogin {
    pub user: String,
    pub authenticated: OffsetDateTime,
    pub last_seen: OffsetDateTime,
    pub idle_expires: OffsetDateTime,
    pub absolute_expires: OffsetDateTime,
}
impl StoreData {
    fn cleanup(&mut self, now: OffsetDateTime) -> u64 {
        let before = self.challenges.len() + self.rates.len() + self.logins.len();
        self.challenges.retain(|_, item| item.expires > now);
        self.rates.retain(|_, item| item.expires > now);
        self.logins
            .retain(|_, item| item.idle_expires > now && item.absolute_expires > now);
        (before - self.challenges.len() - self.rates.len() - self.logins.len()) as u64
    }
    fn len(&self) -> usize {
        self.users.len()
            + self.identities.len()
            + self.challenges.len()
            + self.passkeys.len()
            + self.rates.len()
            + self.logins.len()
    }
}
fn storage_error(_: impl std::fmt::Display) -> AuthError {
    AuthError {
        code: "storage_error",
    }
}

#[async_trait]
pub trait AuthStore: Send + Sync {
    /// Reclaim bounded-store capacity using the application's clock.
    /// Persistent stores retain expired rows until explicit cleanup is requested.
    ///
    /// # Errors
    /// Returns an error if capacity maintenance cannot complete.
    async fn prepare_capacity(&self, _now: OffsetDateTime) -> Result<(), AuthError> {
        Ok(())
    }

    async fn transact(
        &self,
        operation: &mut (
                 dyn for<'data> FnMut(&'data mut StoreData) -> Result<(), AuthError> + Send
             ),
    ) -> Result<(), AuthError>;

    async fn set_user_enabled(&self, user: &UserId, enabled: bool) -> Result<(), AuthError> {
        self.transact(&mut |data| {
            let stored = data.users.get_mut(&user.0).ok_or(AuthError {
                code: "user_not_found",
            })?;
            stored.enabled = enabled;
            Ok(())
        })
        .await
    }
    async fn cleanup_expired(&self, now: OffsetDateTime) -> Result<u64, AuthError> {
        let mut removed = 0;
        self.transact(&mut |data| {
            removed = data.cleanup(now);
            Ok(())
        })
        .await?;
        Ok(removed)
    }
}

/// Explicit development/test store. Capacity counts all stored records.
///
/// Call [`AuthStore::prepare_capacity`] with the application's configured clock
/// before creating flows. The store never consults the system clock, so custom
/// clocks and deterministic expiry checks behave identically across backends.
pub struct MemoryAuthStore {
    data: Mutex<StoreData>,
    capacity: usize,
}
impl MemoryAuthStore {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Mutex::new(StoreData::default()),
            capacity,
        }
    }
}
#[async_trait]
impl AuthStore for MemoryAuthStore {
    async fn prepare_capacity(&self, now: OffsetDateTime) -> Result<(), AuthError> {
        self.cleanup_expired(now).await?;
        Ok(())
    }

    async fn transact(
        &self,
        operation: &mut (
                 dyn for<'data> FnMut(&'data mut StoreData) -> Result<(), AuthError> + Send
             ),
    ) -> Result<(), AuthError> {
        let mut data = self.data.lock().await;
        let mut next = data.clone();
        let result = operation(&mut next);
        if next.len() > self.capacity {
            return Err(AuthError {
                code: "storage_capacity",
            });
        }
        *data = next;
        result
    }
}

// Shared by migrations and ordinary transactions across processes and pools.
const LOCK_ID: i64 = 0x676c_756f_6e61_7574;
const MIGRATION: &str = include_str!("migrations/0001_auth.sql");
const LOGIN_MIGRATION: &str = include_str!("migrations/0002_auth_logins.sql");
const TABLES: [&str; 6] = [
    "gluon_auth_users",
    "gluon_auth_identities",
    "gluon_auth_challenges",
    "gluon_auth_passkeys",
    "gluon_auth_rate_limits",
    "gluon_auth_logins",
];

#[derive(Clone)]
pub struct PostgresAuthStore {
    pool: PgPool,
}
impl PostgresAuthStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Explicit opt-in migration. All DDL and history roll back together.
    ///
    /// # Errors
    /// Returns an error on database failure or incompatible authentication schema.
    pub async fn migrate(&self) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?;
        lock(&mut tx).await?;
        sqlx::raw_sql("CREATE TABLE IF NOT EXISTS gluon_auth_schema_migrations (version BIGINT PRIMARY KEY, description TEXT NOT NULL)")
            .execute(&mut *tx).await.map_err(storage_error)?;
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM gluon_auth_schema_migrations ORDER BY version")
                .fetch_all(&mut *tx)
                .await
                .map_err(storage_error)?;
        if !versions.is_empty() && versions != [1] && versions != [1, 2] {
            return Err(AuthError {
                code: "schema_mismatch",
            });
        }
        if versions.is_empty() {
            sqlx::raw_sql(MIGRATION)
                .execute(&mut *tx)
                .await
                .map_err(storage_error)?;
            sqlx::query("INSERT INTO gluon_auth_schema_migrations (version, description) VALUES (1, 'authentication storage v1')")
                .execute(&mut *tx).await.map_err(storage_error)?;
        }
        if !versions.contains(&2) {
            sqlx::raw_sql(LOGIN_MIGRATION)
                .execute(&mut *tx)
                .await
                .map_err(storage_error)?;
            sqlx::query("INSERT INTO gluon_auth_schema_migrations (version, description) VALUES (2, 'authoritative login liveness')")
                .execute(&mut *tx).await.map_err(storage_error)?;
        }
        check_schema_connection(&mut tx).await?;
        tx.commit().await.map_err(storage_error)
    }

    /// Read-only compatibility check. Never creates or repairs schema.
    ///
    /// # Errors
    /// Returns an error when the schema is missing, incompatible, or inaccessible.
    pub async fn check_schema(&self) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?;
        lock(&mut tx).await?;
        check_schema_connection(&mut tx).await?;
        tx.commit().await.map_err(storage_error)
    }
}
async fn lock(connection: &mut PgConnection) -> Result<(), AuthError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(LOCK_ID)
        .execute(connection)
        .await
        .map_err(storage_error)?;
    Ok(())
}
async fn check_schema_connection(connection: &mut PgConnection) -> Result<(), AuthError> {
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM gluon_auth_schema_migrations ORDER BY version")
            .fetch_all(&mut *connection)
            .await
            .map_err(storage_error)?;
    if versions != [1, 2] {
        return Err(AuthError {
            code: "schema_mismatch",
        });
    }
    for table in TABLES {
        // These identifiers are compile-time constants, never caller input.
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT id::text, data::jsonb FROM {table} LIMIT 0"
        )))
        .execute(&mut *connection)
        .await
        .map_err(storage_error)?;
        let valid: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_constraint c JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey) WHERE c.conrelid = to_regclass($1) AND c.contype = 'p' AND array_length(c.conkey, 1) = 1 AND a.attname = 'id')")
            .bind(table).fetch_one(&mut *connection).await.map_err(storage_error)?;
        let columns_valid: bool = sqlx::query_scalar("SELECT COUNT(*) = 2 FROM pg_attribute WHERE attrelid = to_regclass($1) AND NOT attisdropped AND attnum > 0 AND attnotnull AND ((attname = 'id' AND atttypid = 'text'::regtype) OR (attname = 'data' AND atttypid = 'jsonb'::regtype))")
            .bind(table).fetch_one(&mut *connection).await.map_err(storage_error)?;
        if !valid || !columns_valid {
            return Err(AuthError {
                code: "schema_mismatch",
            });
        }
    }
    Ok(())
}
async fn load<T: DeserializeOwned>(
    connection: &mut PgConnection,
    table: &str,
) -> Result<HashMap<String, T>, AuthError> {
    let rows: Vec<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT id, data::text FROM {table}"
    )))
    .fetch_all(connection)
    .await
    .map_err(storage_error)?;
    rows.into_iter()
        .map(|(id, json)| {
            serde_json::from_str(&json)
                .map(|value| (id, value))
                .map_err(storage_error)
        })
        .collect()
}
async fn persist<T: Serialize + PartialEq>(
    connection: &mut PgConnection,
    table: &str,
    before: &HashMap<String, T>,
    after: &HashMap<String, T>,
) -> Result<(), AuthError> {
    for id in before.keys().filter(|id| !after.contains_key(*id)) {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE id = $1"
        )))
        .bind(id)
        .execute(&mut *connection)
        .await
        .map_err(storage_error)?;
    }
    for (id, value) in after {
        if before.get(id) == Some(value) {
            continue;
        }
        let json = serde_json::to_string(value).map_err(storage_error)?;
        sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO {table} (id, data) VALUES ($1, $2::jsonb) ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data")))
            .bind(id).bind(json).execute(&mut *connection).await.map_err(storage_error)?;
    }
    Ok(())
}
#[async_trait]
impl AuthStore for PostgresAuthStore {
    async fn transact(
        &self,
        operation: &mut (
                 dyn for<'data> FnMut(&'data mut StoreData) -> Result<(), AuthError> + Send
             ),
    ) -> Result<(), AuthError> {
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?;
        // READ COMMITTED takes each snapshot after acquiring this transaction lock.
        // SERIALIZABLE snapshots acquired while waiting could otherwise be stale.
        lock(&mut tx).await?;
        let before = StoreData {
            users: load(&mut tx, TABLES[0]).await?,
            identities: load(&mut tx, TABLES[1]).await?,
            challenges: load(&mut tx, TABLES[2]).await?,
            passkeys: load(&mut tx, TABLES[3]).await?,
            rates: load(&mut tx, TABLES[4]).await?,
            logins: load(&mut tx, TABLES[5]).await?,
        };
        let mut after = before.clone();
        let result = operation(&mut after);
        persist(&mut tx, TABLES[0], &before.users, &after.users).await?;
        persist(&mut tx, TABLES[1], &before.identities, &after.identities).await?;
        persist(&mut tx, TABLES[2], &before.challenges, &after.challenges).await?;
        persist(&mut tx, TABLES[3], &before.passkeys, &after.passkeys).await?;
        persist(&mut tx, TABLES[4], &before.rates, &after.rates).await?;
        persist(&mut tx, TABLES[5], &before.logins, &after.logins).await?;
        tx.commit().await.map_err(storage_error)?;
        result
    }
}
