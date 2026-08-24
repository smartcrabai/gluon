# Graph Report - gluon  (2026-08-24)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 919 nodes · 1660 edges · 64 communities (55 shown, 9 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS · INFERRED: 7 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c7e4aaff`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- container.rs
- boot.rs
- csrf_middleware.rs
- gluon-build/src/lib.rs
- destroy.rs
- htmx_middleware.rs
- wiring.rs
- .from_request_parts
- error.rs
- main.rs
- PostgresSessionStore
- http_csrf.rs
- http_smoke.rs
- inject.rs
- BuildError
- e2e_idempotency.rs
- String
- parse_fields
- templating.rs
- e2e.rs
- gluon-macros/src/lib.rs
- flash.rs
- redirect.rs
- validate_identifier
- dev.rs
- e2e_destroy_loop.rs
- scan
- insert_pub_mod_if_present
- gluon-cli SKILL.md
- review-pr SKILL.md
- validate_route
- server_for
- repository_postgres.rs
- validate_field_type
- scan_app_dir
- db_postgres.rs
- build_run.rs
- entity_basic.rs
- .github/workflows/release.yml
- run
- gluon
- gluon-build
- embed.rs
- .github/workflows/ci.yml
- remote_session_start.sh
- README.md (gluon)

## God Nodes (most connected - your core abstractions)
1. `validate_identifier()` - 19 edges
2. `Container` - 16 edges
3. `scan()` - 16 edges
4. `AppError` - 16 edges
5. `serve_with_shutdown()` - 15 edges
6. `PostgresSessionStore` - 14 edges
7. `parse_fields()` - 14 edges
8. `Boot` - 14 edges
9. `csrf_middleware()` - 14 edges
10. `generate_domain()` - 13 edges

## Surprising Connections (you probably didn't know these)
- `destroy_domain()` --calls--> `validate_identifier()`  [INFERRED]
  crates/gluon-cli/src/commands/destroy.rs → crates/gluon-cli/src/commands/generate.rs
- `destroy_dto()` --calls--> `validate_identifier()`  [INFERRED]
  crates/gluon-cli/src/commands/destroy.rs → crates/gluon-cli/src/commands/generate.rs
- `destroy_migration()` --calls--> `validate_identifier()`  [INFERRED]
  crates/gluon-cli/src/commands/destroy.rs → crates/gluon-cli/src/commands/generate.rs
- `destroy_resource()` --calls--> `validate_identifier()`  [INFERRED]
  crates/gluon-cli/src/commands/destroy.rs → crates/gluon-cli/src/commands/generate.rs
- `destroy_usecase()` --calls--> `validate_identifier()`  [INFERRED]
  crates/gluon-cli/src/commands/destroy.rs → crates/gluon-cli/src/commands/generate.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (64 total, 9 thin omitted)

### Community 0 - "container.rs"
Cohesion: 0.06
Nodes (40): Any, Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), Container, ContainerBuilder, default_builder_builds_empty_container(), factories_see_prior_bindings_during_build() (+32 more)

### Community 2 - "boot.rs"
Cohesion: 0.08
Nodes (44): ContainerFactory, add_test_header(), Boot, build_session_layer(), clear(), counter(), init_tracing(), insecure_development_cookie_omits_secure_attribute() (+36 more)

### Community 3 - "csrf_middleware.rs"
Cohesion: 0.08
Nodes (41): constant_time_eq(), csrf_middleware(), CsrfToken, encode_hex(), ensure_session_token(), extract_form_token(), extract_header_token(), generate_token() (+33 more)

### Community 4 - "gluon-build/src/lib.rs"
Cohesion: 0.05
Nodes (3): preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "destroy.rs"
Cohesion: 0.13
Nodes (29): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+21 more)

### Community 6 - "htmx_middleware.rs"
Cohesion: 0.10
Nodes (33): body_string(), Greeting, invalid_template_path_returns_500(), Display, Error, Formatter, IntoResponse, Option (+25 more)

### Community 7 - "wiring.rs"
Cohesion: 0.09
Nodes (20): Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker(), open_marker_key(), parse_blocks(), parse_blocks_errors_on_unclosed_block(), parse_blocks_single_block() (+12 more)

### Community 8 - ".from_request_parts"
Cohesion: 0.07
Nodes (27): Body, extractor_defaults_when_middleware_was_skipped(), htmx_middleware(), HtmxRequest, FromRequestParts, Future, Next, Output (+19 more)

### Community 9 - "error.rs"
Cohesion: 0.10
Nodes (24): AppError, bad_request_body_includes_message(), body_string(), conflict_body_includes_message(), FieldError, internal_error_body_does_not_leak_source(), Box, Display (+16 more)

### Community 10 - "main.rs"
Cohesion: 0.11
Nodes (14): Path, Result, run(), run_async(), seed_requires_seed_file(), Cli, Commands, DbOp (+6 more)

### Community 11 - "PostgresSessionStore"
Cohesion: 0.12
Nodes (17): PostgresSessionStore, Error, Option, Result, Self, counter(), postgres_sessions_persist_across_instances(), Key (+9 more)

### Community 12 - "http_csrf.rs"
Cohesion: 0.16
Nodes (22): ChildGuard, csrf_blocks_state_changing_without_token(), drain_to_void(), fix_paths(), fresh_app(), gluon_bin(), pick_port(), Child (+14 more)

### Community 13 - "http_smoke.rs"
Cohesion: 0.17
Nodes (21): ChildGuard, drain_to_void(), fix_paths(), fresh_app(), gluon_bin(), http_smoke_serves_basic_routes(), pick_port(), Child (+13 more)

### Community 14 - "inject.rs"
Cohesion: 0.12
Nodes (19): empty_parts(), Inject, Inject<T>, resolves_arc_when_bound(), returns_internal_error_when_unbound(), Arc, FromRequestParts, Future (+11 more)

### Community 15 - "BuildError"
Cohesion: 0.12
Nodes (18): BuildError, check_url_collisions(), check_url_collisions_detects_duplicate(), check_url_collisions_detects_group_normalization(), empty_generated(), empty_generated_contains_router_fn(), entry_url_path_delegates_to_url_path_for(), generate_includes_router_call_for_entry() (+10 more)

### Community 16 - "e2e_idempotency.rs"
Cohesion: 0.26
Nodes (20): controller_api_flag_skips_tsx(), controller_without_api_creates_both(), db_seed_requires_database_url(), destroy_then_generate_restores_wiring_byte_equal(), destroy_unknown_target_is_idempotent_with_nothing_to_remove(), fix_paths(), fresh_app(), generate_usecase_twice_refuses_overwrite() (+12 more)

### Community 17 - "String"
Cohesion: 0.19
Nodes (19): BTreeMap, build_mod_tree(), build_mod_tree_merges_page_and_route_at_same_path(), emit_mod_tree(), emit_router_fn(), Entry, is_group_segment(), mangle_segment_for_mod() (+11 more)

### Community 18 - "parse_fields"
Cohesion: 0.17
Nodes (19): extract_value_objects(), generate_domain(), is_value_object_type(), is_well_known_type(), option_inner(), parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input() (+11 more)

### Community 19 - "templating.rs"
Cohesion: 0.19
Nodes (17): expand_scaffold(), Path, Result, run(), run_cargo_fetch(), run_git_init(), validate_project_name(), load_environment() (+9 more)

### Community 20 - "e2e.rs"
Cohesion: 0.31
Nodes (17): destroy_migration_uses_exact_match(), fix_paths(), fresh_app(), full_lifecycle_builds_after_each_generate(), gluon_bin(), routes_command_lists_generated_routes(), Path, PathBuf (+9 more)

### Community 21 - "gluon-macros/src/lib.rs"
Cohesion: 0.22
Nodes (17): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+9 more)

### Community 22 - "flash.rs"
Cohesion: 0.22
Nodes (14): Flash, new_session(), HashMap, Into, Option, Result, Self, Session (+6 more)

### Community 23 - "redirect.rs"
Cohesion: 0.23
Nodes (14): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, Into, IntoResponse (+6 more)

### Community 24 - "validate_identifier"
Cohesion: 0.23
Nodes (17): generate_controller(), generate_dto(), generate_migration(), generate_resource(), generate_usecase(), insert_bind_if_present(), render_to_file(), Result (+9 more)

### Community 25 - "dev.rs"
Cohesion: 0.20
Nodes (7): make_event(), Child, Result, run(), should_restart(), spawn_app(), Event

### Community 26 - "e2e_destroy_loop.rs"
Cohesion: 0.37
Nodes (13): destroy_resource_cleans_empty_dirs(), fix_paths(), fresh_app(), gluon_bin(), migrations_in_same_second_collide(), Path, PathBuf, String (+5 more)

### Community 27 - "scan"
Cohesion: 0.29
Nodes (13): generate(), generate_emits_concat_env_for_tsx(), generate_emits_layer_for_page_with_tsx(), generate_omits_layer_for_route_rs(), Path, scan(), scan_deep_nesting(), scan_finds_route_and_page_in_different_dirs() (+5 more)

### Community 28 - "insert_pub_mod_if_present"
Cohesion: 0.24
Nodes (12): insert_pub_mod_if_present(), insert_pub_mod_inserts_in_sorted_order(), insert_pub_mod_is_idempotent_for_same_name(), insert_pub_mod_is_no_op_when_file_missing(), insert_pub_mod_is_no_op_when_marker_missing(), insert_pub_mod_preserves_crlf(), insert_pub_mod_sorts_inserts(), route_to_dir() (+4 more)

### Community 29 - "gluon-cli SKILL.md"
Cohesion: 0.33
Nodes (12): gluon CLI binary, gluon-cli references/commands.md, gluon-cli references/conventions.md, Domain と Table 境界の非1:1原則, View<P> テンプレートパス非公開の原則, gluon-cli references/environment.md, gluon-cli references/limitations.md, gluon-cli references/testing.md (+4 more)

### Community 30 - "review-pr SKILL.md"
Cohesion: 0.36
Nodes (10): smartcrabai/agent-skills (upstream skills repo), ai-antipattern skill (companion), .claude/skills directory, improve-review-from-pr-gap SKILL.md, improve-review-from-session SKILL.md, review-pr SKILL.md, review-uncommitted SKILL.md, code-review skill (companion) (+2 more)

### Community 31 - "validate_route"
Cohesion: 0.22
Nodes (9): validate_route(), validate_route_accepts_catch_all_segment(), validate_route_accepts_dynamic_segment(), validate_route_accepts_group_segment(), validate_route_accepts_hyphenated_segment(), validate_route_accepts_nested_dynamic_segment(), validate_route_accepts_simple_segment(), validate_route_accepts_three_levels() (+1 more)

### Community 32 - "server_for"
Cohesion: 0.39
Nodes (8): directory_index_disabled(), nonexistent_returns_404(), root_created_after_service_is_available(), Path, TestServer, server_for(), serves_existing_file(), symlink_cannot_escape_public_directory()

### Community 33 - "repository_postgres.rs"
Cohesion: 0.39
Nodes (7): generated_postgres_repository_supports_crud(), gluon_bin(), Option, Path, PathBuf, run(), workspace_root()

### Community 34 - "validate_field_type"
Cohesion: 0.29
Nodes (7): validate_field_type(), validate_field_type_accepts_box_dyn(), validate_field_type_accepts_hashmap_two_args(), validate_field_type_accepts_option(), validate_field_type_accepts_qualified_path(), validate_field_type_accepts_simple(), validate_field_type_accepts_vec_primitive()

### Community 35 - "scan_app_dir"
Cohesion: 0.40
Nodes (6): extract_http_methods(), is_pub_async(), ItemFn, Vec, scan_app_dir(), File

### Community 36 - "db_postgres.rs"
Cohesion: 0.70
Nodes (4): database_lifecycle_and_seed_work_against_postgres(), gluon_bin(), Path, run_gluon()

### Community 37 - "build_run.rs"
Cohesion: 0.67
Nodes (3): build(), Result, run()

## Knowledge Gaps
- **15 isolated node(s):** `Templates`, `remote_session_start.sh script`, `Greeting`, `View<P> テンプレートパス非公開の原則`, `gluon-cli references/environment.md` (+10 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `serve_with_shutdown()` connect `boot.rs` to `container.rs`, `http_csrf.rs`?**
  _High betweenness centrality (0.044) - this node is a cross-community bridge._
- **Why does `Container` connect `container.rs` to `boot.rs`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._
- **Are the 5 inferred relationships involving `validate_identifier()` (e.g. with `destroy_domain()` and `destroy_dto()`) actually correct?**
  _`validate_identifier()` has 5 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Templates`, `remote_session_start.sh script`, `Greeting` to the rest of the system?**
  _15 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `container.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06312098188194039 - nodes in this community are weakly interconnected._
- **Should `generate.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03571428571428571 - nodes in this community are weakly interconnected._
- **Should `boot.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08069381598793364 - nodes in this community are weakly interconnected._