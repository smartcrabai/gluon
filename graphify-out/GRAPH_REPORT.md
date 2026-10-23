# Graph Report - gluon  (2026-10-23)

## Corpus Check
- 124 files · ~43,994 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 13 file(s) not represented in the graph (top: (none) 8, .feature 1, .als 1)

## Summary
- 1121 nodes · 1989 edges · 84 communities (49 shown, 35 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 49 edges (avg confidence: 0.84)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c5f2040c`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- ContainerBuilder
- Value Object Extraction
- Boot
- csrf.rs
- gluon-build/src/lib.rs
- Destroy Command Logic
- view.rs
- Wiring File Parsing
- htmx_middleware.rs
- App Error Handling
- gluon-cli/src/main.rs
- Postgres Session Store
- gluon application
- gluon application
- inject.rs
- BuildError
- fresh_app
- String
- parse_fields
- new.rs
- examples.rs
- Entity Derive Macro
- Flash Session Messages
- Redirect Response Helper
- validate_identifier
- Dev Server File Watcher
- command
- scan
- Mod.rs Insertion Utilities
- Gluon CLI Skill Docs
- Review Skills Repo
- csrf_spec.rs
- boot.rs
- check-csrf-runners.test.cjs
- Field Type Validation
- scan_app_dir
- e2e.rs
- http_csrf.rs
- package.json
- Release Automation Config
- path
- middleware/static_files.rs
- csrf_middleware.rs
- get
- common/mod.rs
- CI Workflow Config
- Remote Session Script
- serve_with_shutdown
- Gluon Readme
- Inject
- gluon
- prelude.rs
- AGENTS.md
- gluon-build
- Entry
- validate_route
- .run
- check-csrf-quint.sh
- generate_domain
- session_server

## God Nodes (most connected - your core abstractions)
1. `CsrfWorld` - 33 edges
2. `fresh_app()` - 25 edges
3. `ContainerBuilder` - 24 edges
4. `AppError` - 21 edges
5. `validate_identifier()` - 19 edges
6. `Boot` - 18 edges
7. `Container` - 17 edges
8. `csrf_middleware()` - 16 edges
9. `PostgresSessionStore` - 15 edges
10. `scan()` - 13 edges

## Surprising Connections (you probably didn't know these)
- `sample-api` --depends_on--> `gluon`  [EXTRACTED]
  examples/api/Cargo.toml → crates/gluon/Cargo.toml
- `sample-di` --depends_on--> `gluon`  [EXTRACTED]
  examples/di/Cargo.toml → crates/gluon/Cargo.toml
- `sample-htmx` --depends_on--> `gluon`  [EXTRACTED]
  examples/htmx/Cargo.toml → crates/gluon/Cargo.toml
- `sample-pages` --depends_on--> `gluon`  [EXTRACTED]
  examples/pages/Cargo.toml → crates/gluon/Cargo.toml
- `sample-sessions` --depends_on--> `gluon`  [EXTRACTED]
  examples/sessions/Cargo.toml → crates/gluon/Cargo.toml

## Import Cycles
- 1-file cycle: `crates/gluon-cli/src/main.rs -> crates/gluon-cli/src/main.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/destroy.rs -> crates/gluon-cli/src/commands/destroy.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/generate.rs -> crates/gluon-cli/src/commands/generate.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/dev.rs -> crates/gluon-cli/src/commands/dev.rs`
- 1-file cycle: `crates/gluon-cli/src/templating.rs -> crates/gluon-cli/src/templating.rs`
- 1-file cycle: `crates/gluon-macros/src/lib.rs -> crates/gluon-macros/src/lib.rs`

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (84 total, 35 thin omitted)

### Community 0 - "ContainerBuilder"
Cohesion: 0.06
Nodes (24): Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), Container, ContainerBuilder, default_builder_builds_empty_container(), factories_see_prior_bindings_during_build(), factory_can_resolve_multiple_prior_bindings() (+16 more)

### Community 3 - "csrf.rs"
Cohesion: 0.11
Nodes (15): constant_time_eq(), CSRF_FORM_FIELD, CSRF_HEADER, csrf_middleware(), CSRF_SESSION_KEY, CsrfToken, encode_hex(), ensure_session_token() (+7 more)

### Community 4 - "gluon-build/src/lib.rs"
Cohesion: 0.05
Nodes (6): empty_generated(), empty_generated_contains_router_fn(), HTTP_METHODS, preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "Destroy Command Logic"
Cohesion: 0.11
Nodes (25): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+17 more)

### Community 6 - "view.rs"
Cohesion: 0.16
Nodes (10): body_string(), Greeting, invalid_template_path_returns_500(), MIN_PAGE, task_local_isolation(), task_local_template_renders_html(), template_missing_returns_500(), View<P> (+2 more)

### Community 7 - "Wiring File Parsing"
Cohesion: 0.08
Nodes (19): BIND_CLOSE_PREFIX, BIND_OPEN_PREFIX, BINDS_END, BINDS_START, Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker() (+11 more)

### Community 8 - "htmx_middleware.rs"
Cohesion: 0.09
Nodes (13): extractor_defaults_when_middleware_was_skipped(), htmx_middleware(), HtmxRequest, flag_is_false_with_false_header_value(), flag_is_false_without_header(), flag_is_true_case_insensitive(), flag_is_true_with_header_present(), handler() (+5 more)

### Community 9 - "App Error Handling"
Cohesion: 0.08
Nodes (18): User, AppError, BadRequest, Conflict, Forbidden, Internal, NotFound, Unauthorized (+10 more)

### Community 10 - "gluon-cli/src/main.rs"
Cohesion: 0.05
Nodes (25): Cli, Commands, Build, Db, Destroy, Dev, Generate, New (+17 more)

### Community 11 - "Postgres Session Store"
Cohesion: 0.12
Nodes (4): PostgresSessionStore, counter(), postgres_sessions_persist_across_instances(), server()

### Community 12 - "gluon application"
Cohesion: 0.40
Nodes (4): CLI, Conventions, gluon application, Verification

### Community 13 - "gluon application"
Cohesion: 0.40
Nodes (4): CLI, Conventions, gluon application, Verification

### Community 14 - "inject.rs"
Cohesion: 0.15
Nodes (6): empty_parts(), Inject<T>, resolves_arc_when_bound(), returns_internal_error_when_unbound(), TestImpl, TestTrait

### Community 15 - "BuildError"
Cohesion: 0.15
Nodes (14): build_error_display_io(), build_error_display_other(), build_error_display_syn(), build_error_source_propagates(), BuildError, Io, Other, Syn (+6 more)

### Community 16 - "fresh_app"
Cohesion: 0.30
Nodes (14): fresh_app(), destroy_migration_uses_exact_match(), controller_api_flag_skips_tsx(), controller_without_api_creates_both(), db_seed_requires_database_url(), destroy_then_generate_restores_wiring_byte_equal(), destroy_unknown_target_is_idempotent_with_nothing_to_remove(), generate_usecase_twice_refuses_overwrite() (+6 more)

### Community 17 - "String"
Cohesion: 0.22
Nodes (10): emit_router_fn(), emit_router_fn_uses_axum08_path_syntax(), is_group_segment(), mangle_segment_for_mod(), mod_path_for(), sanitize_ident(), strip_brackets(), template_path_expr() (+2 more)

### Community 18 - "parse_fields"
Cohesion: 0.25
Nodes (8): parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input(), parse_fields_preserves_generic_with_comma(), parse_fields_single_field(), parse_fields_trims_whitespace(), parse_fields_two_fields(), ParsedField

### Community 19 - "new.rs"
Cohesion: 0.06
Nodes (33): run(), run_async(), seed_requires_seed_file(), AgentSupport, copy_skill(), create_skill_symlink(), expand_scaffold(), install_agent_support() (+25 more)

### Community 20 - "examples.rs"
Cohesion: 0.17
Nodes (14): ChildGuard, checked_in_examples_match_their_contracts(), client(), compile_examples(), repository_root(), response_json(), run_api(), run_di() (+6 more)

### Community 21 - "Entity Derive Macro"
Cohesion: 0.15
Nodes (13): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+5 more)

### Community 22 - "Flash Session Messages"
Cohesion: 0.23
Nodes (7): Flash, FLASH_KEY, new_session(), set_and_take_round_trip(), set_multiple_keys_round_trip(), take_clears_state(), take_when_unset_returns_empty()

### Community 23 - "Redirect Response Helper"
Cohesion: 0.26
Nodes (8): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, to_sets_303_and_location(), url_with_query_string_preserved()

### Community 24 - "validate_identifier"
Cohesion: 0.15
Nodes (18): generate_controller(), generate_dto(), generate_migration(), generate_resource(), generate_usecase(), insert_bind_if_present(), render_to_file(), route_to_dir() (+10 more)

### Community 25 - "Dev Server File Watcher"
Cohesion: 0.14
Nodes (4): make_event(), run(), should_restart(), spawn_app()

### Community 26 - "command"
Cohesion: 0.17
Nodes (7): git_output(), main(), database_lifecycle_and_seed_work_against_postgres(), gluon_bin(), run_gluon(), generated_postgres_repository_supports_crud(), run()

### Community 27 - "scan"
Cohesion: 0.30
Nodes (12): generate(), generate_emits_concat_env_for_tsx(), generate_emits_layer_for_page_with_tsx(), generate_omits_layer_for_route_rs(), scan(), scan_deep_nesting(), scan_finds_route_and_page_in_different_dirs(), scan_finds_simple_page() (+4 more)

### Community 28 - "Mod.rs Insertion Utilities"
Cohesion: 0.39
Nodes (8): insert_pub_mod_if_present(), insert_pub_mod_inserts_in_sorted_order(), insert_pub_mod_is_idempotent_for_same_name(), insert_pub_mod_is_no_op_when_file_missing(), insert_pub_mod_is_no_op_when_marker_missing(), insert_pub_mod_preserves_crlf(), insert_pub_mod_sorts_inserts(), write_mod_rs()

### Community 29 - "Gluon CLI Skill Docs"
Cohesion: 0.36
Nodes (12): gluon CLI binary, gluon-cli references/commands.md, gluon-cli references/conventions.md, Domain と Table 境界の非1:1原則, View<P> テンプレートパス非公開の原則, gluon-cli references/environment.md, gluon-cli references/limitations.md, gluon-cli references/testing.md (+4 more)

### Community 30 - "Review Skills Repo"
Cohesion: 0.36
Nodes (10): smartcrabai/agent-skills (upstream skills repo), ai-antipattern skill (companion), .claude/skills directory, improve-review-from-pr-gap SKILL.md, improve-review-from-session SKILL.md, review-pr SKILL.md, review-uncommitted SKILL.md, code-review skill (companion) (+2 more)

### Community 31 - "csrf_spec.rs"
Cohesion: 0.09
Nodes (21): admitted_only(), application(), backend_error(), binary(), COOKIE, cross_session(), CsrfWorld, failure() (+13 more)

### Community 32 - "boot.rs"
Cohesion: 0.10
Nodes (13): clear(), counter(), insecure_development_mode_allows_ephemeral_secret(), read_secure_cookie_env_default_is_true(), read_secure_cookie_env_disabled_when_1(), read_secure_cookie_env_disabled_when_true_caseless(), read_secure_cookie_env_enabled_for_other_values(), secure_cookie_requires_long_secret() (+5 more)

### Community 33 - "check-csrf-runners.test.cjs"
Cohesion: 0.11
Nodes (12): assert, checkReceipt(), fs, assert, { checkReceipt }, fs, model, os (+4 more)

### Community 34 - "Field Type Validation"
Cohesion: 0.29
Nodes (7): validate_field_type(), validate_field_type_accepts_box_dyn(), validate_field_type_accepts_hashmap_two_args(), validate_field_type_accepts_option(), validate_field_type_accepts_qualified_path(), validate_field_type_accepts_simple(), validate_field_type_accepts_vec_primitive()

### Community 35 - "scan_app_dir"
Cohesion: 0.40
Nodes (3): extract_http_methods(), is_pub_async(), scan_app_dir()

### Community 36 - "e2e.rs"
Cohesion: 0.17
Nodes (9): run_gluon_yes(), workspace_root(), destroy_resource_cleans_empty_dirs(), migrations_in_same_second_collide(), sleep_until_next_second(), full_lifecycle_builds_after_each_generate(), new_agent_flags_install_expected_files(), routes_command_lists_generated_routes() (+1 more)

### Community 37 - "http_csrf.rs"
Cohesion: 0.31
Nodes (6): drain_to_void(), pick_port(), run_cargo_build(), wait_until_ready(), csrf_blocks_state_changing_without_token(), http_smoke_serves_basic_routes()

### Community 38 - "package.json"
Cohesion: 0.15
Nodes (12): devDependencies, @informalsystems/quint, engines, node, npm, name, overrides, adm-zip (+4 more)

### Community 40 - "path"
Cohesion: 0.29
Nodes (7): View, get(), PageProps, get(), HomeProps, get(), UserProps

### Community 43 - "csrf_middleware.rs"
Cohesion: 0.10
Nodes (17): build_app(), echo_body(), get_passes_through_and_sets_token(), head_and_options_are_safe(), options_handler(), post_exceeding_max_body_returns_413(), post_with_mismatched_token_is_forbidden(), post_with_valid_form_token_passes() (+9 more)

### Community 45 - "common/mod.rs"
Cohesion: 0.27
Nodes (5): fix_paths(), git_dep(), gluon_bin(), run_gluon(), run_gluon_expect_failure()

### Community 62 - "serve_with_shutdown"
Cohesion: 0.22
Nodes (4): build_session_layer(), read_session_key(), serve_with_shutdown(), server_applies_custom_middleware_and_shuts_down()

### Community 64 - "Inject"
Cohesion: 0.27
Nodes (4): Inject, get(), DefaultGreetingService, GreetingService

### Community 65 - "gluon"
Cohesion: 0.29
Nodes (7): gluon, gluon-macros, sample-api, sample-di, sample-htmx, sample-pages, sample-sessions

### Community 78 - "Entry"
Cohesion: 0.22
Nodes (9): build_mod_tree(), build_mod_tree_merges_page_and_route_at_same_path(), emit_mod_tree(), Entry, entry_url_path_delegates_to_url_path_for(), generate_includes_router_call_for_entry(), make_entry(), ModNode (+1 more)

### Community 79 - "validate_route"
Cohesion: 0.22
Nodes (9): validate_route(), validate_route_accepts_catch_all_segment(), validate_route_accepts_dynamic_segment(), validate_route_accepts_group_segment(), validate_route_accepts_hyphenated_segment(), validate_route_accepts_nested_dynamic_segment(), validate_route_accepts_simple_segment(), validate_route_accepts_three_levels() (+1 more)

### Community 80 - ".run"
Cohesion: 0.22
Nodes (5): add_test_header(), init_tracing(), read_secure_cookie_env(), shutdown_signal(), telemetry_disabled()

### Community 81 - "check-csrf-quint.sh"
Cohesion: 0.25
Nodes (4): check-csrf-alloy.sh script, QUINT_HOME, check-csrf-quint.sh script, setup-csrf-specs.sh script

### Community 82 - "generate_domain"
Cohesion: 0.52
Nodes (7): extract_value_objects(), generate_domain(), is_value_object_type(), is_well_known_type(), option_inner(), postgres_crud_supported(), postgres_integer_storage()

### Community 83 - "session_server"
Cohesion: 0.40
Nodes (3): insecure_development_cookie_omits_secure_attribute(), session_cookie_is_signed_and_hardened(), session_server()

## Knowledge Gaps
- **47 isolated node(s):** `gluon-build`, `gluon-cli`, `New`, `Generate`, `Destroy` (+42 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 409 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **35 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ContainerBuilder` connect `ContainerBuilder` to `boot.rs`, `App Error Handling`, `prelude.rs`, `inject.rs`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Are the 16 inferred relationships involving `fresh_app()` (e.g. with `destroy_resource_cleans_empty_dirs()` and `migrations_in_same_second_collide()`) actually correct?**
  _`fresh_app()` has 16 INFERRED edges - model-reasoned connections that need verification._
- **What connects `gluon-build`, `gluon-cli`, `New` to the rest of the system?**
  _47 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `ContainerBuilder` be split into smaller, more focused modules?**
  _Cohesion score 0.061367621274108705 - nodes in this community are weakly interconnected._
- **Why does `DbOp` connect `new.rs` to `gluon-cli/src/main.rs`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Should `Value Object Extraction` be split into smaller, more focused modules?**
  _Cohesion score 0.03389830508474576 - nodes in this community are weakly interconnected._
- **Why does `Container` connect `ContainerBuilder` to `boot.rs`, `Boot`, `prelude.rs`, `App Error Handling`, `inject.rs`, `serve_with_shutdown`?**
  _High betweenness centrality (0.032) - this node is a cross-community bridge._