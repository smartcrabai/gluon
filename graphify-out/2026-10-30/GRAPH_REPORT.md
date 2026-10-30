# Graph Report - layouts  (2026-10-30)

## Corpus Check
- 163 files · ~77,596 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 16 file(s) not represented in the graph (top: (none) 9, .qnt 2, .feature 1)

## Summary
- 2043 nodes · 4533 edges · 115 communities (80 shown, 35 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 196 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `8a3cd3fe`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- container.rs
- Value Object Extraction
- Boot
- csrf.rs
- gluon-build/src/lib.rs
- Destroy Command Logic
- flash.rs
- Wiring File Parsing
- redirect.rs
- AppError
- gluon-cli/src/main.rs
- session.rs
- gluon application
- gluon application
- inject.rs
- BuildError
- fresh_app
- String
- generate_domain
- oauth.rs
- examples.rs
- Entity Derive Macro
- Commands
- db.rs
- DestroyKind
- dev.rs
- path
- scan
- Mod.rs Insertion Utilities
- auth_oauth_providers.rs
- review-pr SKILL.md
- csrf_spec.rs
- boot.rs
- check-csrf-runners.test.cjs
- AuthContext
- scan_app_dir
- e2e_destroy_loop.rs
- common/mod.rs
- package.json
- release.yml
- AuthError
- ContainerBuilder
- GenerateKind
- auth_http.rs
- store.rs
- auth_passkey.rs
- public_key
- with
- Remote Session Script
- serve_with_shutdown
- Gluon Readme
- Authentication
- gluon
- OtpEmail
- AGENTS.md
- gluon-build
- Entry
- validate_route
- .run
- check-csrf-quint.sh
- new.rs
- jobs
- testing_dogfood.rs
- view.rs
- process.rs
- Container
- templating.rs
- auth_jwt.rs
- auth_boot.rs
- Command
- auth_contracts.rs
- htmx_middleware.rs
- .context
- steps
- arc
- arc
- result
- with
- UserId
- build-local-artifacts
- publish-homebrew-formula
- host
- make_entry
- ViewError
- service.rs
- gluon/src/lib.rs
- deserialize
- testing.rs
- session_server
- env
- EnabledMethod
- src/auth/mod.rs

## God Nodes (most connected - your core abstractions)
1. `AuthError` - 116 edges
2. `AuthContext` - 58 edges
3. `Auth` - 42 edges
4. `fixture()` - 39 edges
5. `CsrfWorld` - 33 edges
6. `fresh_app()` - 30 edges
7. `AuthUser` - 26 edges
8. `AuthConfig` - 26 edges
9. `ContainerBuilder` - 26 edges
10. `fixture()` - 26 edges

## Surprising Connections (you probably didn't know these)
- `Contract and production mappings` --references--> `CsrfToken()`  [INFERRED]
  crates/gluon/tests/specs/VERIFICATION.md → crates/gluon-cli/templates/new/app/components/csrf_token.tsx
- `run()` --references--> `AuthConfig`  [EXTRACTED]
  examples/auth/src/lib.rs → crates/gluon/src/auth/mod.rs
- `build_container()` --references--> `ContainerBuilder`  [EXTRACTED]
  examples/di/src/wiring.rs → crates/gluon/src/container.rs
- `boot_child()` --calls--> `post()`  [INFERRED]
  crates/gluon/tests/auth_boot.rs → examples/sessions/app/counter/route.rs
- `Contract and production mappings` --references--> `csrf_blocks_state_changing_without_token()`  [EXTRACTED]
  crates/gluon/tests/specs/VERIFICATION.md → crates/gluon-cli/tests/http_csrf.rs

## Import Cycles
- 1-file cycle: `crates/gluon-cli/src/main.rs -> crates/gluon-cli/src/main.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/destroy.rs -> crates/gluon-cli/src/commands/destroy.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/generate.rs -> crates/gluon-cli/src/commands/generate.rs`
- 1-file cycle: `crates/gluon-cli/src/commands/dev.rs -> crates/gluon-cli/src/commands/dev.rs`
- 1-file cycle: `crates/gluon-cli/src/templating.rs -> crates/gluon-cli/src/templating.rs`
- 1-file cycle: `crates/gluon-macros/src/lib.rs -> crates/gluon-macros/src/lib.rs`
- 2-file cycle: `crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/store.rs -> crates/gluon/src/auth/mod.rs`
- 2-file cycle: `crates/gluon/src/auth/mail.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/mail.rs`
- 2-file cycle: `crates/gluon/src/auth/extractors.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/extractors.rs`

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (115 total, 35 thin omitted)

### Community 0 - "container.rs"
Cohesion: 0.20
Nodes (14): Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), default_builder_builds_empty_container(), factories_see_prior_bindings_during_build(), factory_can_resolve_multiple_prior_bindings(), Greeter, Hello (+6 more)

### Community 1 - "Value Object Extraction"
Cohesion: 0.03
Nodes (16): parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input(), parse_fields_preserves_generic_with_comma(), parse_fields_single_field(), parse_fields_trims_whitespace(), parse_fields_two_fields(), ParsedField (+8 more)

### Community 3 - "csrf.rs"
Cohesion: 0.08
Nodes (24): CsrfToken(), constant_time_eq(), CSRF_FORM_FIELD, CSRF_HEADER, csrf_middleware(), CSRF_SESSION_KEY, encode_hex(), ensure_session_token() (+16 more)

### Community 4 - "gluon-build/src/lib.rs"
Cohesion: 0.05
Nodes (6): empty_generated(), empty_generated_contains_router_fn(), HTTP_METHODS, preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "Destroy Command Logic"
Cohesion: 0.09
Nodes (32): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+24 more)

### Community 6 - "flash.rs"
Cohesion: 0.26
Nodes (7): Flash, FLASH_KEY, new_session(), set_and_take_round_trip(), set_multiple_keys_round_trip(), take_clears_state(), take_when_unset_returns_empty()

### Community 7 - "Wiring File Parsing"
Cohesion: 0.08
Nodes (19): BIND_CLOSE_PREFIX, BIND_OPEN_PREFIX, BINDS_END, BINDS_START, Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker() (+11 more)

### Community 8 - "redirect.rs"
Cohesion: 0.26
Nodes (8): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, to_sets_303_and_location(), url_with_query_string_preserved()

### Community 9 - "AppError"
Cohesion: 0.11
Nodes (17): AppError, BadRequest, Conflict, Forbidden, Internal, NotFound, Unauthorized, Validation (+9 more)

### Community 11 - "session.rs"
Cohesion: 0.08
Nodes (6): PostgresSessionStore, FaultStore, assert_snapshot_save_is_last_write_wins(), counter(), postgres_sessions_persist_across_instances(), server()

### Community 12 - "gluon application"
Cohesion: 0.29
Nodes (6): Authentication (opt-in), CLI, Conventions, gluon application, Pages, layouts, and errors, Verification

### Community 13 - "gluon application"
Cohesion: 0.33
Nodes (5): Authentication (opt-in), CLI, Conventions, gluon application, Verification

### Community 14 - "inject.rs"
Cohesion: 0.16
Nodes (6): empty_parts(), Inject<T>, resolves_arc_when_bound(), returns_internal_error_when_unbound(), TestImpl, TestTrait

### Community 15 - "BuildError"
Cohesion: 0.15
Nodes (14): build_error_display_io(), build_error_display_other(), build_error_display_syn(), build_error_source_propagates(), BuildError, Io, Other, Syn (+6 more)

### Community 16 - "fresh_app"
Cohesion: 0.19
Nodes (23): fresh_app(), run_gluon_yes(), destroy_resource_cleans_empty_dirs(), destroy_migration_uses_exact_match(), domain_collision_leaves_generated_files_and_registrations_unchanged(), dto_dangling_output_symlink_is_rejected_without_side_effects(), full_lifecycle_builds_after_each_generate(), controller_api_flag_skips_tsx() (+15 more)

### Community 17 - "String"
Cohesion: 0.19
Nodes (12): emit_router_fn(), emit_router_fn_uses_axum08_path_syntax(), Entry, is_group_segment(), mangle_segment_for_mod(), mod_path_for(), page_layout_paths(), sanitize_ident() (+4 more)

### Community 18 - "generate_domain"
Cohesion: 0.17
Nodes (17): ensure_no_existing_file(), ensure_no_existing_outputs(), ensure_no_symlinked_parent_dirs(), extract_value_objects(), generate_domain(), generate_dto(), generate_migration(), generate_usecase() (+9 more)

### Community 19 - "oauth.rs"
Cohesion: 0.06
Nodes (22): OAuthExchange, OAuthIdentity, OAuthProvider, authorization(), failure(), GitHubProvider, GoogleProvider, HttpBridge (+14 more)

### Community 20 - "examples.rs"
Cohesion: 0.17
Nodes (16): ChildGuard, auth_command(), checked_in_examples_match_their_contracts(), client(), compile_examples(), repository_root(), response_json(), run_api() (+8 more)

### Community 21 - "Entity Derive Macro"
Cohesion: 0.15
Nodes (13): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+5 more)

### Community 22 - "Commands"
Cohesion: 0.20
Nodes (10): Cli, Commands, Build, Db, Destroy, Dev, Generate, New (+2 more)

### Community 23 - "db.rs"
Cohesion: 0.13
Nodes (11): run(), run_async(), seed_requires_seed_file(), DbOp, Create, Drop, Migrate, Prepare (+3 more)

### Community 24 - "DestroyKind"
Cohesion: 0.29
Nodes (7): DestroyKind, Controller, Domain, Dto, Migration, Resource, Usecase

### Community 25 - "dev.rs"
Cohesion: 0.14
Nodes (4): make_event(), run(), should_restart(), spawn_app()

### Community 26 - "path"
Cohesion: 0.29
Nodes (7): database_lifecycle_and_seed_work_against_postgres(), failed_migration_rolls_back_and_can_be_retried(), gluon_bin(), run_gluon(), routes_command_lists_generated_routes(), generated_postgres_repository_supports_crud(), run()

### Community 27 - "scan"
Cohesion: 0.29
Nodes (13): generate(), generate_collects_nested_and_route_group_layouts(), generate_emits_concat_env_for_tsx(), generate_emits_page_render_middleware(), generate_omits_page_middleware_for_route_rs(), scan(), scan_deep_nesting(), scan_finds_route_and_page_in_different_dirs() (+5 more)

### Community 28 - "Mod.rs Insertion Utilities"
Cohesion: 0.39
Nodes (8): insert_pub_mod_if_present(), insert_pub_mod_inserts_in_sorted_order(), insert_pub_mod_is_idempotent_for_same_name(), insert_pub_mod_is_no_op_when_file_missing(), insert_pub_mod_is_no_op_when_marker_missing(), insert_pub_mod_preserves_crlf(), insert_pub_mod_sorts_inserts(), write_mod_rs()

### Community 29 - "auth_oauth_providers.rs"
Cohesion: 0.12
Nodes (31): claims(), client_config(), current_claims(), discovery_cannot_substitute_a_different_issuer(), exchange(), generic_oidc_discovers_trusted_issuer_and_resolves_verified_subject(), github_authorization_uses_configured_callback_state_and_pkce(), github_identity_survives_login_name_and_email_changes() (+23 more)

### Community 30 - "review-pr SKILL.md"
Cohesion: 0.09
Nodes (29): smartcrabai/agent-skills (upstream skills repo), ai-antipattern skill (companion), .claude/skills directory, improve-review-from-pr-gap SKILL.md, improve-review-from-session SKILL.md, review-pr SKILL.md, review-uncommitted SKILL.md, code-review skill (companion) (+21 more)

### Community 31 - "csrf_spec.rs"
Cohesion: 0.09
Nodes (21): admitted_only(), application(), backend_error(), binary(), COOKIE, cross_session(), CsrfWorld, failure() (+13 more)

### Community 32 - "boot.rs"
Cohesion: 0.10
Nodes (13): clear(), counter(), insecure_development_mode_allows_ephemeral_secret(), read_secure_cookie_env_default_is_true(), read_secure_cookie_env_disabled_when_1(), read_secure_cookie_env_disabled_when_true_caseless(), read_secure_cookie_env_enabled_for_other_values(), secure_cookie_requires_long_secret() (+5 more)

### Community 33 - "check-csrf-runners.test.cjs"
Cohesion: 0.12
Nodes (11): assert, checkReceipt(), fs, assert, fs, model, os, path (+3 more)

### Community 34 - "AuthContext"
Cohesion: 0.06
Nodes (47): authentication_finish(), authentication_start(), AuthenticationStart, Callback, csrf(), EmailRequest, Finish, logout() (+39 more)

### Community 35 - "scan_app_dir"
Cohesion: 0.40
Nodes (3): extract_http_methods(), is_pub_async(), scan_app_dir()

### Community 36 - "e2e_destroy_loop.rs"
Cohesion: 0.17
Nodes (11): assert_destroy_cancelled_without_changes(), DESTROY_CASES, destroy_declined_confirmation_preserves_generated_files_and_registrations(), destroy_eof_preserves_generated_files_and_registrations(), destroy_later_decline_keeps_every_target_unchanged(), DOMAIN_FILES, DTO_FILES, migrations_in_same_second_collide() (+3 more)

### Community 37 - "common/mod.rs"
Cohesion: 0.16
Nodes (13): drain_to_void(), fix_paths(), git_dep(), gluon_bin(), pick_port(), run_cargo_build(), run_gluon(), run_gluon_expect_failure() (+5 more)

### Community 38 - "package.json"
Cohesion: 0.17
Nodes (12): devDependencies, @informalsystems/quint@0.29.1, engines, node, npm, name, overrides, adm-zip (+4 more)

### Community 39 - "release.yml"
Cohesion: 0.17
Nodes (11): cargo-dist (axodotdev), env, ARTIFACT_RETENTION_DAYS, name, on, pull_request, push, permissions (+3 more)

### Community 40 - "AuthError"
Cohesion: 0.17
Nodes (15): AuthError, AuthFlow, Auth, Ceremony, check_deletion(), decode_key(), different_configured_provider_does_not_enable_historical_identity(), enabled_email_allows_passkey_deletion() (+7 more)

### Community 41 - "ContainerBuilder"
Cohesion: 0.23
Nodes (6): ContainerBuilder, Factory, build_container(), build_container(), build_container(), build_container()

### Community 42 - "GenerateKind"
Cohesion: 0.29
Nodes (7): GenerateKind, Controller, Domain, Dto, Migration, Resource, Usecase

### Community 43 - "auth_http.rs"
Cohesion: 0.07
Nodes (26): AuthContext, BROWSER_KEY, CurrentUser, internal(), assert_cors_for_configured_origin(), assert_extractor_session_failure(), assert_flow_response(), assert_no_store() (+18 more)

### Community 44 - "store.rs"
Cohesion: 0.09
Nodes (18): AuthStore, check_schema_connection(), load(), lock(), LOCK_ID, LOGIN_MIGRATION, MemoryAuthStore, MIGRATION (+10 more)

### Community 45 - "auth_passkey.rs"
Cohesion: 0.09
Nodes (46): anonymous_cannot_delete_an_existing_credential(), anonymous_cannot_list_credentials(), anonymous_registration_finish_cannot_choose_a_victim_user(), anonymous_registration_start_rejects(), another_user_cannot_delete_owner_credential(), assert_option_schema(), assert_public_only(), assertion() (+38 more)

### Community 46 - "public_key"
Cohesion: 0.43
Nodes (4): Auth, invalid_config(), public_key(), validate_jwt()

### Community 53 - "with"
Cohesion: 0.06
Nodes (39): permissions, runs-on, steps, timeout-minutes, jobs, csrf-spec, platform-test, test (+31 more)

### Community 62 - "serve_with_shutdown"
Cohesion: 0.22
Nodes (5): build_session_layer(), read_session_key(), RouterFactory, serve_with_shutdown(), server_applies_custom_middleware_and_shuts_down()

### Community 64 - "Authentication"
Cohesion: 0.17
Nodes (10): Authentication, Development checks, HTTP contract and CSRF, JWT lifetime and operations, Minimal integration, OAuth and Passkeys, Registration policy and business users, Screenless example (+2 more)

### Community 65 - "gluon"
Cohesion: 0.25
Nodes (8): gluon, gluon-macros, sample-api, sample-auth, sample-di, sample-htmx, sample-pages, sample-sessions

### Community 66 - "OtpEmail"
Cohesion: 0.10
Nodes (8): SmtpConfig, SmtpEmailSender, EmailSender, OtpEmail, UnusedSender, PausedSender, ServerTask, smtp_without_tls_cannot_receive_credentials_or_otp_message()

### Community 78 - "Entry"
Cohesion: 0.40
Nodes (5): build_mod_tree(), build_mod_tree_merges_page_and_route_at_same_path(), emit_mod_tree(), ModNode, push_indent()

### Community 79 - "validate_route"
Cohesion: 0.13
Nodes (20): controller_output_paths(), ControllerOutputs, Api, Pages, ensure_controller_outputs(), generate_controller(), generate_resource(), render_controller() (+12 more)

### Community 80 - ".run"
Cohesion: 0.22
Nodes (5): add_test_header(), init_tracing(), read_secure_cookie_env(), shutdown_signal(), telemetry_disabled()

### Community 81 - "check-csrf-quint.sh"
Cohesion: 0.25
Nodes (4): check-csrf-alloy.sh script, QUINT_HOME, check-csrf-quint.sh script, setup-csrf-specs.sh script

### Community 82 - "new.rs"
Cohesion: 0.26
Nodes (12): AgentSupport, copy_skill(), create_skill_symlink(), expand_scaffold(), install_agent_support(), run(), run_cargo_fetch(), run_git_init() (+4 more)

### Community 83 - "jobs"
Cohesion: 0.15
Nodes (16): env, if, needs, runs-on, env, needs, runs-on, env (+8 more)

### Community 85 - "view.rs"
Cohesion: 0.17
Nodes (23): accepts_html(), add_error_vary_headers(), app_errors_keep_plain_or_json_bodies_when_html_is_not_accepted(), assert_error_vary_headers(), body_string(), failed_error_template_render_keeps_plain_response_and_vary_headers(), Greeting, html_app_errors_render_templates_through_only_the_root_layout() (+15 more)

### Community 86 - "process.rs"
Cohesion: 0.24
Nodes (5): csrf(), fixture_process(), Mailbox, otp_session_jwt_logout_through_shared_example_process(), Process

### Community 88 - "templating.rs"
Cohesion: 0.23
Nodes (5): Templates, load_environment(), render(), render_existing_template_succeeds(), render_unknown_template_errors()

### Community 89 - "auth_jwt.rs"
Cohesion: 0.07
Nodes (34): invalid_token(), JwtConfig, JwtKey, TokenClaims, anonymous_session_cannot_mint_tokens(), fixture(), issued_token_is_verifiable_using_only_public_jwks_and_session_subject(), jwks_contains_public_ed25519_keys_but_no_private_material() (+26 more)

### Community 90 - "auth_boot.rs"
Cohesion: 0.11
Nodes (25): assert_cookie_only_token_authentication(), boot_child(), boot_csrf_rejections_are_not_cached_only_under_the_auth_mount(), boot_http_and_injected_direct_api_use_the_with_container_auth_override(), boot_http_disabled_leaves_auth_in_di_but_mounts_no_auth_endpoints(), boot_otp_rotates_session_csrf_and_preserves_cookie_only_authentication(), boot_public_jwks_is_cacheable_without_session_or_private_key_material(), boot_public_jwks_rejects_unsafe_methods_without_creating_sessions() (+17 more)

### Community 92 - "auth_contracts.rs"
Cohesion: 0.16
Nodes (17): absent_database_does_not_silently_select_memory_storage(), active_jwt_kid_must_reference_a_configured_key(), base(), builder_default_attempt_limit_is_five(), builder_default_otp_is_eight_digits(), builder_default_otp_lifetime_is_five_minutes(), builder_default_resend_interval_is_sixty_seconds(), callbackless_provider_accepts_https_fallback_with_custom_mount() (+9 more)

### Community 93 - "htmx_middleware.rs"
Cohesion: 0.05
Nodes (24): extractor_defaults_when_middleware_was_skipped(), header_is_true(), htmx_middleware(), HtmxRequest, is_htmx_fragment(), observed_flag(), serve_file(), service() (+16 more)

### Community 94 - ".context"
Cohesion: 0.05
Nodes (74): concurrent_verification_of_one_challenge_succeeds_once(), configured_maximum_failed_attempts_permanently_exhausts_challenge(), delivered_code_is_eight_decimal_characters_and_authenticates_once(), delivery_failure_invalidates_only_failed_challenge(), destination_limit_is_shared_across_sources(), disabled_user_cannot_login_or_remain_authenticated(), identity_resolution_is_shared_across_auth_instances(), incorrect() (+66 more)

### Community 95 - "steps"
Cohesion: 0.39
Nodes (12): steps, steps, steps, steps, steps, steps, id, if (+4 more)

### Community 96 - "arc"
Cohesion: 0.27
Nodes (4): Inject, get(), DefaultGreetingService, GreetingService

### Community 97 - "arc"
Cohesion: 0.20
Nodes (3): main(), required(), build_container()

### Community 98 - "result"
Cohesion: 0.17
Nodes (9): Result, View, get(), get(), PageProps, get(), HomeProps, get() (+1 more)

### Community 99 - "with"
Cohesion: 0.20
Nodes (10): with, merge-multiple, name, path, pattern, persist-credentials, repository, retention-days (+2 more)

### Community 100 - "UserId"
Cohesion: 0.29
Nodes (5): RegistrationPolicy, UserId, VerifiedIdentity, ExistingSubject, Reject

### Community 101 - "build-local-artifacts"
Cohesion: 0.22
Nodes (9): container, if, name, needs, runs-on, strategy, build-local-artifacts, fail-fast (+1 more)

### Community 102 - "publish-homebrew-formula"
Cohesion: 0.25
Nodes (8): GITHUB_EMAIL, GITHUB_USER, PLAN, publish-homebrew-formula, env, if, needs, runs-on

### Community 103 - "host"
Cohesion: 0.18
Nodes (11): env, if, needs, outputs, runs-on, host, publishing, tag (+3 more)

### Community 104 - "make_entry"
Cohesion: 0.67
Nodes (3): entry_url_path_delegates_to_url_path_for(), generate_includes_router_call_for_entry(), make_entry()

### Community 106 - "service.rs"
Cohesion: 0.15
Nodes (12): AuthUser, error(), OAuthStart, OtpChallenge, random_id(), Auth, Login, normalize_email() (+4 more)

### Community 108 - "gluon/src/lib.rs"
Cohesion: 0.18
Nodes (3): User, FieldError, app!

### Community 111 - "session_server"
Cohesion: 0.40
Nodes (3): insecure_development_cookie_omits_secure_attribute(), session_cookie_is_signed_and_hardened(), session_server()

### Community 112 - "env"
Cohesion: 0.40
Nodes (5): ANNOUNCEMENT_BODY, ANNOUNCEMENT_TITLE, PRERELEASE_FLAG, RELEASE_COMMIT, env

### Community 113 - "EnabledMethod"
Cohesion: 0.50
Nodes (4): EnabledMethod, Email, None, OAuth

### Community 114 - "src/auth/mod.rs"
Cohesion: 0.14
Nodes (8): AuthBuilder, AuthConfig, Clock, EmailOtpConfig, oauth_callback(), PasskeyConfig, SystemClock, validate_config()

## Knowledge Gaps
- **50 isolated node(s):** `gluon-build`, `gluon-cli`, `Pages`, `New`, `Generate` (+45 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 618 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **35 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `context()` connect `db.rs` to `auth_jwt.rs`, `Value Object Extraction`, `AuthContext`, `Destroy Command Logic`, `Wiring File Parsing`, `auth_passkey.rs`, `new.rs`, `templating.rs`, `dev.rs`, `.context`?**
  _High betweenness centrality (0.062) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `AuthError` (e.g. with `.exchange()` and `.send()`) actually correct?**
  _`AuthError` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `gluon-build`, `gluon-cli`, `Pages` to the rest of the system?**
  _50 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Value Object Extraction` be split into smaller, more focused modules?**
  _Cohesion score 0.03221029248426509 - nodes in this community are weakly interconnected._
- **Why does `AuthError` connect `AuthError` to `AuthContext`, `OtpEmail`, `UserId`, `service.rs`, `auth_http.rs`, `store.rs`, `public_key`, `src/auth/mod.rs`, `oauth.rs`, `process.rs`, `auth_jwt.rs`, `auth_boot.rs`, `auth_oauth_providers.rs`, `.context`?**
  _High betweenness centrality (0.061) - this node is a cross-community bridge._
- **Should `Boot` be split into smaller, more focused modules?**
  _Cohesion score 0.11578947368421053 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `result` to `boot.rs`, `Value Object Extraction`, `arc`, `AuthContext`, `Destroy Command Logic`, `Wiring File Parsing`, `AppError`, `gluon/src/lib.rs`, `deserialize`, `new.rs`, `src/auth/mod.rs`, `templating.rs`?**
  _High betweenness centrality (0.048) - this node is a cross-community bridge._