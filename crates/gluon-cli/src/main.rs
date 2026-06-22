//! gluon CLI.
//!
//! Implements `new`, `generate`/`g`, `destroy`/`d`, `db`, `dev`, `build`,
//! `run`, and `routes` subcommands.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::allow_attributes,
        clippy::missing_panics_doc
    )
)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod commands;
mod embed;
mod templating;
mod wiring;

/// Top-level CLI for the gluon framework.
#[derive(Debug, Parser)]
#[command(name = "gluon", version, about = "CLI for the gluon framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new gluon application.
    New {
        /// Name of the application to create.
        name: String,
        /// Skip git repository initialization.
        #[arg(long)]
        no_git: bool,
        /// Skip running the dependency installer.
        #[arg(long)]
        no_install: bool,
    },
    /// Generate a new code artifact.
    #[command(alias = "g")]
    Generate {
        #[command(subcommand)]
        kind: GenerateKind,
    },
    /// Remove a previously-generated code artifact.
    #[command(alias = "d")]
    Destroy {
        #[command(subcommand)]
        kind: DestroyKind,
    },
    /// Database management commands.
    Db {
        #[command(subcommand)]
        op: DbOp,
    },
    /// Run the development server with live reload.
    Dev,
    /// Build the application for production.
    Build,
    /// Run the compiled application.
    Run {
        /// Run in release mode.
        #[arg(long)]
        release: bool,
    },
    /// List the registered HTTP routes.
    Routes,
}

/// Kinds of artifacts the `generate` subcommand can scaffold.
#[derive(Debug, Subcommand)]
pub(crate) enum GenerateKind {
    /// Generate a controller for a given route.
    Controller {
        /// Route path (e.g. "/users/[id]").
        route: String,
        /// Generate a JSON API controller (no view template).
        #[arg(long)]
        api: bool,
    },
    /// Generate a `RESTful` resource (multiple controllers).
    Resource {
        /// Resource name (plural, e.g. "users").
        name: String,
    },
    /// Generate a use case.
    Usecase {
        /// Use case name.
        name: String,
    },
    /// Generate a domain object (entity + value objects + repository).
    Domain {
        /// Domain object name.
        name: String,
        /// Field definition in `name:Type` form. May be repeated.
        #[arg(long = "field", value_name = "NAME:TYPE")]
        fields: Vec<String>,
    },
    /// Generate a data transfer object.
    Dto {
        /// DTO name.
        name: String,
    },
    /// Generate a database migration.
    Migration {
        /// Migration name.
        name: String,
    },
}

/// Kinds of artifacts the `destroy` subcommand can remove.
#[derive(Debug, Subcommand)]
pub(crate) enum DestroyKind {
    /// Remove a previously-generated controller.
    Controller {
        /// Route path (e.g. "/users/[id]").
        route: String,
    },
    /// Remove a previously-generated `RESTful` resource.
    Resource {
        /// Resource name.
        name: String,
    },
    /// Remove a previously-generated use case.
    Usecase {
        /// Use case name.
        name: String,
    },
    /// Remove a previously-generated domain object.
    Domain {
        /// Domain object name.
        name: String,
    },
    /// Remove a previously-generated DTO.
    Dto {
        /// DTO name.
        name: String,
    },
    /// Remove a previously-generated migration.
    Migration {
        /// Migration name.
        name: String,
    },
}

/// Database management operations.
#[derive(Clone, Copy, Debug, Subcommand)]
pub(crate) enum DbOp {
    /// Create the database.
    Create,
    /// Drop the database.
    Drop,
    /// Apply pending migrations.
    Migrate,
    /// Roll back the most recent migration.
    Rollback,
    /// Prepare the database for compile-time query verification.
    Prepare,
    /// Seed the database with initial data.
    Seed,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result: Result<(), anyhow::Error> = match cli.command {
        Commands::New {
            name,
            no_git,
            no_install,
        } => commands::new::run(&name, no_git, no_install),
        Commands::Generate { kind } => commands::generate::run(kind),
        Commands::Destroy { kind } => commands::destroy::run(kind),
        Commands::Db { op } => commands::db::run(op),
        Commands::Dev => commands::dev::run(),
        Commands::Build => commands::build_run::build(),
        Commands::Run { release } => commands::build_run::run(release),
        Commands::Routes => commands::routes::run(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands, DbOp, DestroyKind, GenerateKind};
    use clap::Parser;

    #[test]
    fn parse_new_with_no_git_no_install() {
        let cli =
            Cli::try_parse_from(["gluon", "new", "myapp", "--no-git", "--no-install"]).unwrap();
        match cli.command {
            Commands::New {
                name,
                no_git,
                no_install,
            } => {
                assert_eq!(name, "myapp");
                assert!(no_git);
                assert!(no_install);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_generate_controller_with_api() {
        let cli = Cli::try_parse_from(["gluon", "g", "controller", "users", "--api"]).unwrap();
        match cli.command {
            Commands::Generate {
                kind: GenerateKind::Controller { route, api },
            } => {
                assert_eq!(route, "users");
                assert!(api);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_generate_controller_without_api() {
        let cli = Cli::try_parse_from(["gluon", "g", "controller", "users"]).unwrap();
        match cli.command {
            Commands::Generate {
                kind: GenerateKind::Controller { api, .. },
            } => assert!(!api),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_generate_domain_with_fields() {
        let cli = Cli::try_parse_from([
            "gluon",
            "g",
            "domain",
            "user",
            "--field",
            "name:String",
            "--field",
            "age:i32",
        ])
        .unwrap();
        match cli.command {
            Commands::Generate {
                kind: GenerateKind::Domain { name, fields },
            } => {
                assert_eq!(name, "user");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0], "name:String");
                assert_eq!(fields[1], "age:i32");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_destroy_migration() {
        let cli = Cli::try_parse_from(["gluon", "d", "migration", "create_users"]).unwrap();
        match cli.command {
            Commands::Destroy {
                kind: DestroyKind::Migration { name },
            } => assert_eq!(name, "create_users"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_db_subcommands_alias() {
        let cli = Cli::try_parse_from(["gluon", "db", "create"]).unwrap();
        match cli.command {
            Commands::Db { op } => assert!(matches!(op, DbOp::Create)),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_run_with_release() {
        let cli = Cli::try_parse_from(["gluon", "run", "--release"]).unwrap();
        match cli.command {
            Commands::Run { release } => assert!(release),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_run_without_release() {
        let cli = Cli::try_parse_from(["gluon", "run"]).unwrap();
        match cli.command {
            Commands::Run { release } => assert!(!release),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_alias_generate_short() {
        let long = Cli::try_parse_from(["gluon", "generate", "dto", "post"]).unwrap();
        let short = Cli::try_parse_from(["gluon", "g", "dto", "post"]).unwrap();
        match (long.command, short.command) {
            (
                Commands::Generate {
                    kind: GenerateKind::Dto { name: l },
                },
                Commands::Generate {
                    kind: GenerateKind::Dto { name: s },
                },
            ) => assert_eq!(l, s),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_alias_destroy_short() {
        let long = Cli::try_parse_from(["gluon", "destroy", "dto", "post"]).unwrap();
        let short = Cli::try_parse_from(["gluon", "d", "dto", "post"]).unwrap();
        match (long.command, short.command) {
            (
                Commands::Destroy {
                    kind: DestroyKind::Dto { name: l },
                },
                Commands::Destroy {
                    kind: DestroyKind::Dto { name: s },
                },
            ) => assert_eq!(l, s),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_fails_without_subcommand() {
        assert!(Cli::try_parse_from(["gluon"]).is_err());
    }

    #[test]
    fn parse_fails_unknown_subcommand() {
        assert!(Cli::try_parse_from(["gluon", "frobnicate"]).is_err());
    }
}
