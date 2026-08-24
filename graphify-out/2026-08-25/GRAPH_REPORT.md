# Graph Report - gluon  (2026-08-25)

## Corpus Check
- 114 files · ~39,003 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1039 nodes · 1813 edges · 76 communities (67 shown, 9 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 16 edges (avg confidence: 0.83)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `984f5f4e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- ContainerBuilder
- boot.rs
- csrf.rs
- Build Script Utilities
- Destroy Command Logic
- view.rs
- Wiring File Parsing
- htmx_middleware.rs
- App Error Handling
- CLI Command Definitions
- Postgres Session Store
- inject.rs
- BuildError
- E2E Idempotency Tests
- String
- Domain Field Parsing
- Project Scaffolding
- E2E Lifecycle Tests
- Entity Derive Macro
- Flash Session Messages
- Redirect Response Helper
- Result
- Dev Server File Watcher
- E2E Destroy Loop Tests
- App Directory Scanning
- Mod.rs Insertion Utilities
- Gluon CLI Skill Docs
- Review Skills Repo
- examples.rs
- Static File Serving Tests
- Postgres Repository Generation
- Field Type Validation
- Entry
- Postgres DB Lifecycle Test
- Build Run Command
- Entity Example Test
- Release Automation Config
- Routes Command
- csrf_middleware.rs
- get
- Template Embedding
- CI Workflow Config
- Remote Session Script
- Gluon Readme
- validate_identifier
- gluon
- get
- AGENTS.md
- gluon-build

## God Nodes (most connected - your core abstractions)
1. `ContainerBuilder` - 20 edges
2. `validate_identifier()` - 19 edges
3. `scan()` - 16 edges
4. `Container` - 16 edges
5. `AppError` - 16 edges
6. `serve_with_shutdown()` - 15 edges
7. `parse_fields()` - 14 edges
8. `Boot` - 14 edges
9. `csrf_middleware()` - 14 edges
10. `PostgresSessionStore` - 14 edges

## Surprising Connections (you probably didn't know these)
- `build_container()` --references--> `ContainerBuilder`  [EXTRACTED]
  examples/di/src/wiring.rs → crates/gluon/src/container.rs
- `sample-api` --depends_on--> `gluon`  [EXTRACTED]
  examples/api/Cargo.toml → crates/gluon/Cargo.toml
- `sample-di` --depends_on--> `gluon`  [EXTRACTED]
  examples/di/Cargo.toml → crates/gluon/Cargo.toml
- `sample-htmx` --depends_on--> `gluon`  [EXTRACTED]
  examples/htmx/Cargo.toml → crates/gluon/Cargo.toml
- `sample-pages` --depends_on--> `gluon`  [EXTRACTED]
  examples/pages/Cargo.toml → crates/gluon/Cargo.toml

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (76 total, 9 thin omitted)

### Community 0 - "ContainerBuilder"
Cohesion: 0.07
Nodes (40): Any, Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), Container, ContainerBuilder, default_builder_builds_empty_container(), factories_see_prior_bindings_during_build() (+32 more)

### Community 2 - "boot.rs"
Cohesion: 0.06
Nodes (54): ContainerFactory, add_test_header(), Boot, build_session_layer(), clear(), counter(), init_tracing(), insecure_development_cookie_omits_secure_attribute() (+46 more)

### Community 3 - "csrf.rs"
Cohesion: 0.13
Nodes (26): constant_time_eq(), csrf_middleware(), CsrfToken, encode_hex(), ensure_session_token(), extract_form_token(), extract_header_token(), generate_token() (+18 more)

### Community 4 - "Build Script Utilities"
Cohesion: 0.05
Nodes (3): preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "Destroy Command Logic"
Cohesion: 0.13
Nodes (29): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+21 more)

### Community 6 - "view.rs"
Cohesion: 0.08
Nodes (33): body_string(), Greeting, invalid_template_path_returns_500(), Display, Error, Formatter, IntoResponse, Option (+25 more)

### Community 7 - "Wiring File Parsing"
Cohesion: 0.09
Nodes (20): Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker(), open_marker_key(), parse_blocks(), parse_blocks_errors_on_unclosed_block(), parse_blocks_single_block() (+12 more)

### Community 8 - "htmx_middleware.rs"
Cohesion: 0.06
Nodes (38): Body, extractor_defaults_when_middleware_was_skipped(), htmx_middleware(), HtmxRequest, FromRequestParts, Future, Next, Output (+30 more)

### Community 9 - "App Error Handling"
Cohesion: 0.10
Nodes (24): AppError, bad_request_body_includes_message(), body_string(), conflict_body_includes_message(), FieldError, internal_error_body_does_not_leak_source(), Box, Display (+16 more)

### Community 10 - "CLI Command Definitions"
Cohesion: 0.11
Nodes (14): Path, Result, run(), run_async(), seed_requires_seed_file(), Cli, Commands, DbOp (+6 more)

### Community 11 - "Postgres Session Store"
Cohesion: 0.12
Nodes (17): PostgresSessionStore, Error, Option, Result, Self, counter(), postgres_sessions_persist_across_instances(), Key (+9 more)

### Community 14 - "inject.rs"
Cohesion: 0.05
Nodes (35): empty_parts(), Inject, Inject<T>, resolves_arc_when_bound(), returns_internal_error_when_unbound(), Arc, FromRequestParts, Future (+27 more)

### Community 15 - "BuildError"
Cohesion: 0.12
Nodes (19): BuildError, check_url_collisions(), check_url_collisions_detects_duplicate(), check_url_collisions_detects_group_normalization(), empty_generated(), empty_generated_contains_router_fn(), entry_url_path_delegates_to_url_path_for(), generate_includes_router_call_for_entry() (+11 more)

### Community 16 - "E2E Idempotency Tests"
Cohesion: 0.26
Nodes (20): controller_api_flag_skips_tsx(), controller_without_api_creates_both(), db_seed_requires_database_url(), destroy_then_generate_restores_wiring_byte_equal(), destroy_unknown_target_is_idempotent_with_nothing_to_remove(), fix_paths(), fresh_app(), generate_usecase_twice_refuses_overwrite() (+12 more)

### Community 17 - "String"
Cohesion: 0.26
Nodes (13): emit_mod_tree(), emit_router_fn(), emit_router_fn_uses_axum08_path_syntax(), is_group_segment(), mangle_segment_for_mod(), mod_path_for(), push_indent(), String (+5 more)

### Community 18 - "Domain Field Parsing"
Cohesion: 0.17
Nodes (19): extract_value_objects(), generate_domain(), is_value_object_type(), is_well_known_type(), option_inner(), parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input() (+11 more)

### Community 19 - "Project Scaffolding"
Cohesion: 0.19
Nodes (17): expand_scaffold(), Path, Result, run(), run_cargo_fetch(), run_git_init(), validate_project_name(), load_environment() (+9 more)

### Community 20 - "E2E Lifecycle Tests"
Cohesion: 0.31
Nodes (17): destroy_migration_uses_exact_match(), fix_paths(), fresh_app(), full_lifecycle_builds_after_each_generate(), gluon_bin(), routes_command_lists_generated_routes(), Path, PathBuf (+9 more)

### Community 21 - "Entity Derive Macro"
Cohesion: 0.22
Nodes (17): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+9 more)

### Community 22 - "Flash Session Messages"
Cohesion: 0.22
Nodes (14): Flash, new_session(), HashMap, Into, Option, Result, Self, Session (+6 more)

### Community 23 - "Redirect Response Helper"
Cohesion: 0.23
Nodes (14): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, Into, IntoResponse (+6 more)

### Community 24 - "Result"
Cohesion: 0.18
Nodes (19): generate_controller(), generate_dto(), generate_migration(), generate_resource(), generate_usecase(), insert_bind_if_present(), render_to_file(), Result (+11 more)

### Community 25 - "Dev Server File Watcher"
Cohesion: 0.20
Nodes (7): make_event(), Child, Result, run(), should_restart(), spawn_app(), Event

### Community 26 - "E2E Destroy Loop Tests"
Cohesion: 0.37
Nodes (13): destroy_resource_cleans_empty_dirs(), fix_paths(), fresh_app(), gluon_bin(), migrations_in_same_second_collide(), Path, PathBuf, String (+5 more)

### Community 27 - "App Directory Scanning"
Cohesion: 0.29
Nodes (13): generate(), generate_emits_concat_env_for_tsx(), generate_emits_layer_for_page_with_tsx(), generate_omits_layer_for_route_rs(), Path, scan(), scan_deep_nesting(), scan_finds_route_and_page_in_different_dirs() (+5 more)

### Community 28 - "Mod.rs Insertion Utilities"
Cohesion: 0.24
Nodes (12): insert_pub_mod_if_present(), insert_pub_mod_inserts_in_sorted_order(), insert_pub_mod_is_idempotent_for_same_name(), insert_pub_mod_is_no_op_when_file_missing(), insert_pub_mod_is_no_op_when_marker_missing(), insert_pub_mod_preserves_crlf(), insert_pub_mod_sorts_inserts(), route_to_dir() (+4 more)

### Community 29 - "Gluon CLI Skill Docs"
Cohesion: 0.33
Nodes (12): gluon CLI binary, gluon-cli references/commands.md, gluon-cli references/conventions.md, Domain と Table 境界の非1:1原則, View<P> テンプレートパス非公開の原則, gluon-cli references/environment.md, gluon-cli references/limitations.md, gluon-cli references/testing.md (+4 more)

### Community 30 - "Review Skills Repo"
Cohesion: 0.36
Nodes (10): smartcrabai/agent-skills (upstream skills repo), ai-antipattern skill (companion), .claude/skills directory, improve-review-from-pr-gap SKILL.md, improve-review-from-session SKILL.md, review-pr SKILL.md, review-uncommitted SKILL.md, code-review skill (companion) (+2 more)

### Community 31 - "examples.rs"
Cohesion: 0.06
Nodes (57): ChildGuard, drain_to_void(), pick_port(), Child, Option, Result, Self, checked_in_examples_match_their_contracts() (+49 more)

### Community 32 - "Static File Serving Tests"
Cohesion: 0.39
Nodes (8): directory_index_disabled(), nonexistent_returns_404(), root_created_after_service_is_available(), Path, TestServer, server_for(), serves_existing_file(), symlink_cannot_escape_public_directory()

### Community 33 - "Postgres Repository Generation"
Cohesion: 0.39
Nodes (7): generated_postgres_repository_supports_crud(), gluon_bin(), Option, Path, PathBuf, run(), workspace_root()

### Community 34 - "Field Type Validation"
Cohesion: 0.29
Nodes (7): validate_field_type(), validate_field_type_accepts_box_dyn(), validate_field_type_accepts_hashmap_two_args(), validate_field_type_accepts_option(), validate_field_type_accepts_qualified_path(), validate_field_type_accepts_simple(), validate_field_type_accepts_vec_primitive()

### Community 35 - "Entry"
Cohesion: 0.21
Nodes (12): BTreeMap, build_mod_tree(), build_mod_tree_merges_page_and_route_at_same_path(), Entry, extract_http_methods(), is_pub_async(), ModNode, ItemFn (+4 more)

### Community 36 - "Postgres DB Lifecycle Test"
Cohesion: 0.70
Nodes (4): database_lifecycle_and_seed_work_against_postgres(), gluon_bin(), Path, run_gluon()

### Community 37 - "Build Run Command"
Cohesion: 0.67
Nodes (3): build(), Result, run()

### Community 43 - "csrf_middleware.rs"
Cohesion: 0.12
Nodes (27): build_app(), echo_body(), get_passes_through_and_sets_token(), head_and_options_are_safe(), options_handler(), post_exceeding_max_body_returns_413(), post_with_mismatched_token_is_forbidden(), post_with_valid_form_token_passes() (+19 more)

### Community 44 - "get"
Cohesion: 0.31
Nodes (8): get(), ItemQuery, Json, Path, Result, String, Value, Query

### Community 64 - "validate_identifier"
Cohesion: 0.29
Nodes (7): validate_identifier(), validate_identifier_accepts_leading_underscore(), validate_identifier_accepts_lower_word(), validate_identifier_accepts_pascal_with_digit(), validate_identifier_accepts_single_underscore(), validate_identifier_accepts_snake_case(), validate_identifier_accepts_trailing_underscore()

### Community 65 - "gluon"
Cohesion: 0.29
Nodes (7): gluon, gluon-macros, sample-api, sample-di, sample-htmx, sample-pages, sample-sessions

### Community 66 - "get"
Cohesion: 0.60
Nodes (4): get(), Json, Result, Value

## Knowledge Gaps
- **20 isolated node(s):** `gluon-build`, `gluon-cli`, `Templates`, `gluon-macros`, `Greeting` (+15 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `serve_with_shutdown()` connect `boot.rs` to `ContainerBuilder`, `examples.rs`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **Why does `Container` connect `ContainerBuilder` to `boot.rs`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._
- **Are the 5 inferred relationships involving `validate_identifier()` (e.g. with `destroy_domain()` and `destroy_dto()`) actually correct?**
  _`validate_identifier()` has 5 INFERRED edges - model-reasoned connections that need verification._
- **What connects `gluon-build`, `gluon-cli`, `Templates` to the rest of the system?**
  _20 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `ContainerBuilder` be split into smaller, more focused modules?**
  _Cohesion score 0.06654567453115548 - nodes in this community are weakly interconnected._
- **Should `Value Object Extraction` be split into smaller, more focused modules?**
  _Cohesion score 0.03571428571428571 - nodes in this community are weakly interconnected._
- **Should `boot.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.055178652193577565 - nodes in this community are weakly interconnected._