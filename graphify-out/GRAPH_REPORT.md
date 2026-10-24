# Graph Report - 20261023163131497_5cbe0a9d75e74c628276d19eb80e7382  (2026-10-24)

## Corpus Check
- 160 files · ~73,349 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 14 file(s) not represented in the graph (top: (none) 9, .feature 1, .als 1)

## Summary
- 1852 nodes · 4172 edges · 115 communities (73 shown, 42 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 164 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `f5dcd32a`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- container.rs
- Value Object Extraction
- Boot
- csrf.rs
- gluon-build/src/lib.rs
- Destroy Command Logic
- [id]/route.rs
- Wiring File Parsing
- AuthBuilder
- AppError
- gluon-cli/src/main.rs
- session.rs
- gluon application
- gluon application
- inject.rs
- BuildError
- fresh_app
- String
- parse_fields
- oauth.rs
- examples.rs
- Entity Derive Macro
- csrf_middleware.rs
- db.rs
- validate_identifier
- dev.rs
- path
- scan
- Mod.rs Insertion Utilities
- auth_oauth_providers.rs
- Review Skills Repo
- csrf_spec.rs
- boot.rs
- check-csrf-runners.test.cjs
- http.rs
- scan_app_dir
- e2e_destroy_loop.rs
- common/mod.rs
- package.json
- Release Automation Config
- AuthError
- ContainerBuilder
- auth_http.rs
- store.rs
- auth_passkey.rs
- command
- CI Workflow Config
- Remote Session Script
- build_session_layer
- Gluon Readme
- Authentication
- gluon
- common/auth/mod.rs
- AGENTS.md
- gluon-build
- Entry
- validate_route
- .run
- check-csrf-quint.sh
- prelude.rs
- serve_with_shutdown
- testing_dogfood.rs
- view.rs
- redirect.rs
- testing.rs
- auth_oauth_transport.rs
- auth_jwt.rs
- auth_boot.rs
- htmx.rs
- auth_contracts.rs
- htmx_middleware.rs
- .context
- e2e.rs
- arc
- mail.rs
- result
- gluon/src/lib.rs
- gluon-cli SKILL.md
- new.rs
- middleware/static_files.rs
- templating.rs
- UserId
- process.rs
- service.rs
- post
- ChildGuard
- tests/static_files.rs
- Container
- validate_config
- get
- auth_smtp.rs

## God Nodes (most connected - your core abstractions)
1. `AuthError` - 116 edges
2. `AuthContext` - 58 edges
3. `Auth` - 42 edges
4. `fixture()` - 39 edges
5. `CsrfWorld` - 33 edges
6. `fresh_app()` - 26 edges
7. `AuthUser` - 26 edges
8. `AuthConfig` - 26 edges
9. `ContainerBuilder` - 26 edges
10. `fixture()` - 26 edges

## Surprising Connections (you probably didn't know these)
- `Contract and production mappings` --references--> `CsrfToken()`  [INFERRED]
  crates/gluon/tests/specs/VERIFICATION.md → crates/gluon-cli/templates/new/app/components/csrf_token.tsx
- `router()` --calls--> `post()`  [INFERRED]
  crates/gluon/src/auth/http.rs → examples/sessions/app/counter/route.rs
- `run()` --references--> `AuthConfig`  [EXTRACTED]
  examples/auth/src/lib.rs → crates/gluon/src/auth/mod.rs
- `build_container()` --references--> `ContainerBuilder`  [EXTRACTED]
  examples/di/src/wiring.rs → crates/gluon/src/container.rs
- `boot_child()` --calls--> `post()`  [INFERRED]
  crates/gluon/tests/auth_boot.rs → examples/sessions/app/counter/route.rs

## Import Cycles
- 1-file cycle: `crates/gluon-cli/src/main.rs -> crates/gluon-cli/src/main.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/destroy.rs -> crates/gluon-cli/src/commands/destroy.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/generate.rs -> crates/gluon-cli/src/commands/generate.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/dev.rs -> crates/gluon-cli/src/commands/dev.rs`
- 1-file cycle: `crates/gluon-cli/src/templating.rs -> crates/gluon-cli/src/templating.rs`
- 1-file cycle: `crates/gluon-macros/src/lib.rs -> crates/gluon-macros/src/lib.rs`
- 2-file cycle: `crates/gluon/src/auth/extractors.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/extractors.rs`
- 2-file cycle: `crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/store.rs -> crates/gluon/src/auth/mod.rs`
- 2-file cycle: `crates/gluon/src/auth/mail.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/mail.rs`

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (115 total, 42 thin omitted)

### Community 0 - "container.rs"
Cohesion: 0.21
Nodes (13): Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), default_builder_builds_empty_container(), factories_see_prior_bindings_during_build(), factory_can_resolve_multiple_prior_bindings(), Greeter, Hello (+5 more)

### Community 1 - "Value Object Extraction"
Cohesion: 0.03
Nodes (8): validate_field_type(), validate_field_type_accepts_box_dyn(), validate_field_type_accepts_hashmap_two_args(), validate_field_type_accepts_option(), validate_field_type_accepts_qualified_path(), validate_field_type_accepts_simple(), validate_field_type_accepts_vec_primitive(), WIRING_PATH

### Community 3 - "csrf.rs"
Cohesion: 0.08
Nodes (23): CsrfToken(), constant_time_eq(), CSRF_FORM_FIELD, CSRF_HEADER, csrf_middleware(), CSRF_SESSION_KEY, encode_hex(), ensure_session_token() (+15 more)

### Community 4 - "gluon-build/src/lib.rs"
Cohesion: 0.05
Nodes (6): empty_generated(), empty_generated_contains_router_fn(), HTTP_METHODS, preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "Destroy Command Logic"
Cohesion: 0.09
Nodes (32): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+24 more)

### Community 7 - "Wiring File Parsing"
Cohesion: 0.08
Nodes (19): BIND_CLOSE_PREFIX, BIND_OPEN_PREFIX, BINDS_END, BINDS_START, Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker() (+11 more)

### Community 8 - "AuthBuilder"
Cohesion: 0.22
Nodes (3): AuthBuilder, Clock, SystemClock

### Community 9 - "AppError"
Cohesion: 0.11
Nodes (15): AppError, BadRequest, Conflict, Forbidden, Internal, NotFound, Unauthorized, Validation (+7 more)

### Community 10 - "gluon-cli/src/main.rs"
Cohesion: 0.05
Nodes (25): Cli, Commands, Build, Db, Destroy, Dev, Generate, New (+17 more)

### Community 11 - "session.rs"
Cohesion: 0.08
Nodes (5): PostgresSessionStore, FaultStore, counter(), postgres_sessions_persist_across_instances(), server()

### Community 12 - "gluon application"
Cohesion: 0.33
Nodes (5): Authentication (opt-in), CLI, Conventions, gluon application, Verification

### Community 13 - "gluon application"
Cohesion: 0.33
Nodes (5): Authentication (opt-in), CLI, Conventions, gluon application, Verification

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
Cohesion: 0.19
Nodes (15): extract_value_objects(), generate_domain(), is_value_object_type(), is_well_known_type(), option_inner(), parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input() (+7 more)

### Community 19 - "oauth.rs"
Cohesion: 0.06
Nodes (23): oauth_callback(), OAuthExchange, OAuthIdentity, OAuthProvider, authorization(), failure(), GitHubProvider, GoogleProvider (+15 more)

### Community 20 - "examples.rs"
Cohesion: 0.21
Nodes (15): auth_command(), checked_in_examples_match_their_contracts(), client(), compile_examples(), repository_root(), response_json(), run_api(), run_auth() (+7 more)

### Community 21 - "Entity Derive Macro"
Cohesion: 0.15
Nodes (13): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+5 more)

### Community 22 - "csrf_middleware.rs"
Cohesion: 0.08
Nodes (21): Flash, FLASH_KEY, new_session(), set_and_take_round_trip(), set_multiple_keys_round_trip(), take_clears_state(), take_when_unset_returns_empty(), build_app() (+13 more)

### Community 23 - "db.rs"
Cohesion: 0.15
Nodes (11): run(), run_async(), seed_requires_seed_file(), DbOp, Create, Drop, Migrate, Prepare (+3 more)

### Community 24 - "validate_identifier"
Cohesion: 0.23
Nodes (11): generate_controller(), generate_dto(), generate_migration(), generate_resource(), generate_usecase(), insert_bind_if_present(), render_to_file(), route_to_dir() (+3 more)

### Community 25 - "dev.rs"
Cohesion: 0.14
Nodes (4): make_event(), run(), should_restart(), spawn_app()

### Community 26 - "path"
Cohesion: 0.27
Nodes (7): database_lifecycle_and_seed_work_against_postgres(), failed_migration_rolls_back_and_can_be_retried(), gluon_bin(), run_gluon(), routes_command_lists_generated_routes(), generated_postgres_repository_supports_crud(), run()

### Community 27 - "scan"
Cohesion: 0.30
Nodes (12): generate(), generate_emits_concat_env_for_tsx(), generate_emits_layer_for_page_with_tsx(), generate_omits_layer_for_route_rs(), scan(), scan_deep_nesting(), scan_finds_route_and_page_in_different_dirs(), scan_finds_simple_page() (+4 more)

### Community 28 - "Mod.rs Insertion Utilities"
Cohesion: 0.39
Nodes (8): insert_pub_mod_if_present(), insert_pub_mod_inserts_in_sorted_order(), insert_pub_mod_is_idempotent_for_same_name(), insert_pub_mod_is_no_op_when_file_missing(), insert_pub_mod_is_no_op_when_marker_missing(), insert_pub_mod_preserves_crlf(), insert_pub_mod_sorts_inserts(), write_mod_rs()

### Community 29 - "auth_oauth_providers.rs"
Cohesion: 0.13
Nodes (31): claims(), client_config(), current_claims(), discovery_cannot_substitute_a_different_issuer(), exchange(), generic_oidc_discovers_trusted_issuer_and_resolves_verified_subject(), github_authorization_uses_configured_callback_state_and_pkce(), github_identity_survives_login_name_and_email_changes() (+23 more)

### Community 30 - "Review Skills Repo"
Cohesion: 0.36
Nodes (10): smartcrabai/agent-skills (upstream skills repo), ai-antipattern skill (companion), .claude/skills directory, improve-review-from-pr-gap SKILL.md, improve-review-from-session SKILL.md, review-pr SKILL.md, review-uncommitted SKILL.md, code-review skill (companion) (+2 more)

### Community 31 - "csrf_spec.rs"
Cohesion: 0.08
Nodes (23): CsrfToken, admitted_only(), application(), backend_error(), binary(), COOKIE, cross_session(), CsrfWorld (+15 more)

### Community 32 - "boot.rs"
Cohesion: 0.10
Nodes (13): clear(), counter(), insecure_development_mode_allows_ephemeral_secret(), read_secure_cookie_env_default_is_true(), read_secure_cookie_env_disabled_when_1(), read_secure_cookie_env_disabled_when_true_caseless(), read_secure_cookie_env_enabled_for_other_values(), secure_cookie_requires_long_secret() (+5 more)

### Community 33 - "check-csrf-runners.test.cjs"
Cohesion: 0.11
Nodes (12): assert, checkReceipt(), fs, assert, { checkReceipt }, fs, model, os (+4 more)

### Community 34 - "http.rs"
Cohesion: 0.20
Nodes (25): authentication_finish(), authentication_start(), AuthenticationStart, Callback, csrf(), EmailRequest, Finish, logout() (+17 more)

### Community 35 - "scan_app_dir"
Cohesion: 0.40
Nodes (3): extract_http_methods(), is_pub_async(), scan_app_dir()

### Community 36 - "e2e_destroy_loop.rs"
Cohesion: 0.15
Nodes (12): assert_destroy_cancelled_without_changes(), DESTROY_CASES, destroy_declined_confirmation_preserves_generated_files_and_registrations(), destroy_eof_preserves_generated_files_and_registrations(), destroy_later_decline_keeps_every_target_unchanged(), destroy_resource_cleans_empty_dirs(), DOMAIN_FILES, DTO_FILES (+4 more)

### Community 37 - "common/mod.rs"
Cohesion: 0.19
Nodes (9): drain_to_void(), gluon_bin(), pick_port(), run_cargo_build(), run_gluon(), run_gluon_expect_failure(), wait_until_ready(), csrf_blocks_state_changing_without_token() (+1 more)

### Community 38 - "package.json"
Cohesion: 0.15
Nodes (12): devDependencies, @informalsystems/quint, engines, node, npm, name, overrides, adm-zip (+4 more)

### Community 40 - "AuthError"
Cohesion: 0.12
Nodes (21): AuthError, AuthFlow, Auth, Ceremony, check_deletion(), decode_key(), different_configured_provider_does_not_enable_historical_identity(), enabled_email_allows_passkey_deletion() (+13 more)

### Community 41 - "ContainerBuilder"
Cohesion: 0.23
Nodes (5): ContainerBuilder, build_container(), build_container(), build_container(), build_container()

### Community 43 - "auth_http.rs"
Cohesion: 0.10
Nodes (22): assert_cors_for_configured_origin(), assert_extractor_session_failure(), assert_flow_response(), assert_no_store(), assert_passkey_error(), auth_session_and_csrf_responses_are_not_cached(), cors_allows_only_configured_origin(), cors_canonicalizes_configured_origin_with_trailing_slash() (+14 more)

### Community 44 - "store.rs"
Cohesion: 0.10
Nodes (17): AuthStore, check_schema_connection(), load(), lock(), LOCK_ID, LOGIN_MIGRATION, MemoryAuthStore, MIGRATION (+9 more)

### Community 45 - "auth_passkey.rs"
Cohesion: 0.09
Nodes (46): anonymous_cannot_delete_an_existing_credential(), anonymous_cannot_list_credentials(), anonymous_registration_finish_cannot_choose_a_victim_user(), anonymous_registration_start_rejects(), another_user_cannot_delete_owner_credential(), assert_option_schema(), assert_public_only(), assertion() (+38 more)

### Community 62 - "build_session_layer"
Cohesion: 0.22
Nodes (5): build_session_layer(), insecure_development_cookie_omits_secure_attribute(), read_session_key(), session_cookie_is_signed_and_hardened(), session_server()

### Community 64 - "Authentication"
Cohesion: 0.18
Nodes (9): Authentication, HTTP contract and CSRF, JWT lifetime and operations, Minimal integration, OAuth and Passkeys, Registration policy and business users, Screenless example, Storage and migrations (+1 more)

### Community 65 - "gluon"
Cohesion: 0.25
Nodes (8): gluon, gluon-macros, sample-api, sample-auth, sample-di, sample-htmx, sample-pages, sample-sessions

### Community 66 - "common/auth/mod.rs"
Cohesion: 0.16
Nodes (9): AuthConfig, EmailOtpConfig, EmailSender, OtpEmail, PausedSender, config(), Fixture, Mailbox (+1 more)

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

### Community 82 - "prelude.rs"
Cohesion: 0.15
Nodes (4): AuthContext, BROWSER_KEY, CurrentUser, internal()

### Community 85 - "view.rs"
Cohesion: 0.21
Nodes (10): body_string(), Greeting, invalid_template_path_returns_500(), MIN_PAGE, task_local_isolation(), task_local_template_renders_html(), template_missing_returns_500(), View<P> (+2 more)

### Community 86 - "redirect.rs"
Cohesion: 0.26
Nodes (8): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, to_sets_303_and_location(), url_with_query_string_preserved()

### Community 88 - "auth_oauth_transport.rs"
Cohesion: 0.21
Nodes (4): oauth_http_redirect_is_not_followed(), oauth_http_response_limit_allows_exact_boundary_but_rejects_one_byte_more(), oauth_http_timeout_bounds_a_nonresponding_provider(), Server

### Community 89 - "auth_jwt.rs"
Cohesion: 0.08
Nodes (27): Auth, invalid_config(), invalid_token(), public_key(), validate_jwt(), JwtConfig, JwtKey, TokenClaims (+19 more)

### Community 90 - "auth_boot.rs"
Cohesion: 0.18
Nodes (17): OtpChallenge, boot_child(), boot_csrf_rejections_are_not_cached_only_under_the_auth_mount(), boot_http_and_injected_direct_api_use_the_with_container_auth_override(), boot_http_disabled_leaves_auth_in_di_but_mounts_no_auth_endpoints(), boot_otp_rotates_session_csrf_and_preserves_cookie_only_authentication(), boot_public_jwks_is_cacheable_without_session_or_private_key_material(), boot_public_jwks_rejects_unsafe_methods_without_creating_sessions() (+9 more)

### Community 91 - "htmx.rs"
Cohesion: 0.13
Nodes (4): extractor_defaults_when_middleware_was_skipped(), htmx_middleware(), HtmxRequest, handler()

### Community 92 - "auth_contracts.rs"
Cohesion: 0.15
Nodes (17): absent_database_does_not_silently_select_memory_storage(), active_jwt_kid_must_reference_a_configured_key(), base(), builder_default_attempt_limit_is_five(), builder_default_otp_is_eight_digits(), builder_default_otp_lifetime_is_five_minutes(), builder_default_resend_interval_is_sixty_seconds(), callbackless_provider_accepts_https_fallback_with_custom_mount() (+9 more)

### Community 93 - "htmx_middleware.rs"
Cohesion: 0.26
Nodes (9): flag_is_false_with_false_header_value(), flag_is_false_without_header(), flag_is_true_case_insensitive(), flag_is_true_with_header_present(), htmx_view_returns_fragment_while_normal_request_returns_document(), page(), PageProps, server() (+1 more)

### Community 94 - ".context"
Cohesion: 0.06
Nodes (70): concurrent_verification_of_one_challenge_succeeds_once(), configured_maximum_failed_attempts_permanently_exhausts_challenge(), delivered_code_is_eight_decimal_characters_and_authenticates_once(), delivery_failure_invalidates_only_failed_challenge(), destination_limit_is_shared_across_sources(), disabled_user_cannot_login_or_remain_authenticated(), identity_resolution_is_shared_across_auth_instances(), incorrect() (+62 more)

### Community 95 - "e2e.rs"
Cohesion: 0.31
Nodes (7): fix_paths(), git_dep(), run_gluon_yes(), workspace_root(), full_lifecycle_builds_after_each_generate(), new_agent_flags_install_expected_files(), run_cargo_check()

### Community 96 - "arc"
Cohesion: 0.21
Nodes (5): Inject, get(), build_container(), DefaultGreetingService, GreetingService

### Community 97 - "mail.rs"
Cohesion: 0.16
Nodes (5): SmtpConfig, SmtpEmailSender, PasskeyConfig, main(), required()

### Community 98 - "result"
Cohesion: 0.18
Nodes (8): View, get(), get(), PageProps, get(), HomeProps, get(), UserProps

### Community 99 - "gluon/src/lib.rs"
Cohesion: 0.18
Nodes (3): User, FieldError, app!

### Community 100 - "gluon-cli SKILL.md"
Cohesion: 0.42
Nodes (11): gluon CLI binary, gluon-cli references/commands.md, gluon-cli references/conventions.md, Domain と Table 境界の非1:1原則, View<P> テンプレートパス非公開の原則, gluon-cli references/environment.md, gluon-cli references/limitations.md, gluon-cli references/testing.md (+3 more)

### Community 101 - "new.rs"
Cohesion: 0.26
Nodes (12): AgentSupport, copy_skill(), create_skill_symlink(), expand_scaffold(), install_agent_support(), run(), run_cargo_fetch(), run_git_init() (+4 more)

### Community 103 - "templating.rs"
Cohesion: 0.23
Nodes (5): Templates, load_environment(), render(), render_existing_template_succeeds(), render_unknown_template_errors()

### Community 104 - "UserId"
Cohesion: 0.26
Nodes (5): RegistrationPolicy, UserId, VerifiedIdentity, ExistingSubject, Reject

### Community 105 - "process.rs"
Cohesion: 0.27
Nodes (5): csrf(), fixture_process(), Mailbox, otp_session_jwt_logout_through_shared_example_process(), Process

### Community 106 - "service.rs"
Cohesion: 0.16
Nodes (11): AuthUser, error(), OAuthStart, random_id(), Auth, Login, normalize_email(), otp_mac() (+3 more)

### Community 107 - "post"
Cohesion: 0.62
Nodes (3): get(), post(), read_count()

### Community 109 - "tests/static_files.rs"
Cohesion: 0.29
Nodes (6): directory_index_disabled(), nonexistent_returns_404(), root_created_after_service_is_available(), server_for(), serves_existing_file(), symlink_cannot_escape_public_directory()

## Knowledge Gaps
- **50 isolated node(s):** `gluon-build`, `gluon-cli`, `New`, `Generate`, `Destroy` (+45 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 541 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **42 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AuthError` connect `AuthError` to `mail.rs`, `http.rs`, `common/auth/mod.rs`, `UserId`, `process.rs`, `service.rs`, `store.rs`, `validate_config`, `prelude.rs`, `oauth.rs`, `auth_jwt.rs`, `auth_boot.rs`, `auth_oauth_providers.rs`, `.context`?**
  _High betweenness centrality (0.084) - this node is a cross-community bridge._
- **What connects `gluon-build`, `gluon-cli`, `New` to the rest of the system?**
  _50 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Value Object Extraction` be split into smaller, more focused modules?**
  _Cohesion score 0.0331002331002331 - nodes in this community are weakly interconnected._
- **Why does `context()` connect `db.rs` to `auth_jwt.rs`, `Value Object Extraction`, `common/auth/mod.rs`, `http.rs`, `new.rs`, `Destroy Command Logic`, `templating.rs`, `Wiring File Parsing`, `auth_passkey.rs`, `dev.rs`, `.context`?**
  _High betweenness centrality (0.069) - this node is a cross-community bridge._
- **Should `Boot` be split into smaller, more focused modules?**
  _Cohesion score 0.11578947368421053 - nodes in this community are weakly interconnected._
- **Why does `AuthContext` connect `http.rs` to `common/auth/mod.rs`, `AuthError`, `service.rs`, `auth_http.rs`, `store.rs`, `auth_passkey.rs`, `prelude.rs`, `db.rs`, `auth_jwt.rs`, `auth_boot.rs`, `.context`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._
- **Should `csrf.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08258258258258258 - nodes in this community are weakly interconnected._