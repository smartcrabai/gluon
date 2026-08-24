//! `gluon db` PostgreSQL database management commands.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};
use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::{PgPool, Postgres};

use crate::DbOp;

/// Dispatches a database operation using `DATABASE_URL`.
///
/// `prepare` remains delegated to `cargo sqlx prepare`; the other operations
/// use `SQLx` directly and require no separately-installed `sqlx` executable.
///
/// # Errors
///
/// Returns an error when configuration, filesystem access, SQL execution, or
/// the delegated prepare command fails.
pub fn run(op: DbOp) -> anyhow::Result<()> {
    if matches!(op, DbOp::Prepare) {
        let status = Command::new("cargo")
            .args(["sqlx", "prepare"])
            .status()
            .context("launch `cargo sqlx prepare`")?;
        if !status.success() {
            bail!("cargo sqlx prepare failed: {status}");
        }
        return Ok(());
    }

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    tokio::runtime::Runtime::new()
        .context("create Tokio runtime")?
        .block_on(run_async(op, &database_url, Path::new(".")))
}

async fn run_async(op: DbOp, database_url: &str, root: &Path) -> anyhow::Result<()> {
    match op {
        DbOp::Create => Postgres::create_database(database_url).await?,
        DbOp::Drop => Postgres::drop_database(database_url).await?,
        DbOp::Migrate => {
            let pool = PgPool::connect(database_url).await?;
            Migrator::new(root.join("migrations"))
                .await?
                .run(&pool)
                .await?;
        }
        DbOp::Rollback => {
            let pool = PgPool::connect(database_url).await?;
            let migrator = Migrator::new(root.join("migrations")).await?;
            let has_migration_table =
                sqlx::query_scalar::<_, bool>("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
                    .fetch_one(&pool)
                    .await?;
            if !has_migration_table {
                return Ok(());
            }
            let target = sqlx::query_scalar::<_, i64>(
                "SELECT version FROM _sqlx_migrations WHERE success = TRUE \
                 ORDER BY version DESC LIMIT 1 OFFSET 1",
            )
            .fetch_optional(&pool)
            .await?
            .unwrap_or(0);
            migrator.undo(&pool, target).await?;
        }
        DbOp::Seed => {
            let sql =
                std::fs::read_to_string(root.join("db/seeds.sql")).context("read db/seeds.sql")?;
            let pool = PgPool::connect(database_url).await?;
            sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
                .execute(&pool)
                .await?;
        }
        DbOp::Prepare => unreachable!("prepare handled before starting the runtime"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn seed_requires_seed_file() {
        let root = tempfile::tempdir().unwrap();
        let error = run_async(
            DbOp::Seed,
            "postgres://invalid:invalid@127.0.0.1:1/invalid",
            root.path(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("read db/seeds.sql"));
    }
}
