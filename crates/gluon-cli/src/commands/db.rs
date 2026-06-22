//! `gluon db` subcommand: thin wrapper over `sqlx-cli`.

use std::process::Command;

use crate::DbOp;

/// Map a [`DbOp`] to the `sqlx-cli` argument vector.
///
/// # Errors
/// Returns an error for operations that have not been implemented yet
/// (e.g. [`DbOp::Seed`]).
pub(crate) fn args_for(op: DbOp) -> anyhow::Result<Vec<&'static str>> {
    let args = match op {
        DbOp::Create => vec!["database", "create"],
        DbOp::Drop => vec!["database", "drop", "-y"],
        DbOp::Migrate => vec!["migrate", "run"],
        DbOp::Rollback => vec!["migrate", "revert"],
        DbOp::Prepare => vec!["prepare"],
        DbOp::Seed => {
            anyhow::bail!("`gluon db seed` is not yet implemented");
        }
    };
    Ok(args)
}

/// Dispatch a database operation by invoking `sqlx-cli` with the appropriate
/// arguments.
///
/// # Errors
/// Returns an error when:
/// - the operation has not been implemented yet (e.g. `seed`),
/// - the `sqlx` binary cannot be launched, or
/// - the `sqlx` invocation exits with a non-zero status.
pub fn run(op: DbOp) -> anyhow::Result<()> {
    let args = args_for(op)?;
    let status = Command::new("sqlx").args(&args).status()?;
    if !status.success() {
        anyhow::bail!("sqlx command failed: {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DbOp, args_for};

    #[test]
    fn create_args() {
        assert_eq!(args_for(DbOp::Create).unwrap(), vec!["database", "create"]);
    }

    #[test]
    fn drop_args() {
        assert_eq!(
            args_for(DbOp::Drop).unwrap(),
            vec!["database", "drop", "-y"]
        );
    }

    #[test]
    fn migrate_args() {
        assert_eq!(args_for(DbOp::Migrate).unwrap(), vec!["migrate", "run"]);
    }

    #[test]
    fn rollback_args() {
        assert_eq!(args_for(DbOp::Rollback).unwrap(), vec!["migrate", "revert"]);
    }

    #[test]
    fn prepare_args() {
        assert_eq!(args_for(DbOp::Prepare).unwrap(), vec!["prepare"]);
    }

    #[test]
    fn seed_args_errors() {
        let err = args_for(DbOp::Seed).unwrap_err();
        assert!(
            err.to_string().contains("not yet implemented"),
            "unexpected error: {err}"
        );
    }
}
