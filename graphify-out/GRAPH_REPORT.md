# Graph Report - gluon  (2026-10-27)

## Corpus Check
- 163 files · ~75,925 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 15 file(s) not represented in the graph (top: (none) 8, .qnt 2, .feature 1)

## Summary
- 2006 nodes · 4436 edges · 116 communities (81 shown, 35 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 172 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d91f786c`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- container.rs
- Value Object Extraction
- Boot
- csrf_middleware.rs
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
- http.rs
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
- JwtKey
- with
- Remote Session Script
- serve_with_shutdown
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
- htmx.rs
- jobs
- testing_dogfood.rs
- view.rs
- CSRF specifications verification evidence
- testing.rs
- EmailSender
- auth_jwt.rs
- auth_boot.rs
- auth_contracts.rs
- htmx_middleware.rs
- .context
- steps
- arc
- mail.rs
- result
- with
- HtmxRequest
- build-local-artifacts
- publish-homebrew-formula
- host
- UserId
- process.rs
- service.rs
- plan
- common/mod.rs
- deserialize
- Container
- session_server
- env
- entity_basic.rs
- EnabledMethod
- AuthConfig

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
- `router()` --calls--> `post()`  [INFERRED]
  crates/gluon/src/auth/http.rs → examples/sessions/app/counter/route.rs
- `run()` --references--> `AuthConfig`  [EXTRACTED]
  examples/auth/src/lib.rs → crates/gluon/src/auth/mod.rs
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
- 2-file cycle: `crates/gluon/src/auth/extractors.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/extractors.rs`
- 2-file cycle: `crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/store.rs -> crates/gluon/src/auth/mod.rs`
- 2-file cycle: `crates/gluon/src/auth/mail.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/mail.rs`

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (116 total, 35 thin omitted)

### Community 0 - "container.rs"
Cohesion: 0.20
Nodes (14): Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), default_builder_builds_empty_container(), factories_see_prior_bindings_during_build(), factory_can_resolve_multiple_prior_bindings(), Greeter, Hello (+6 more)

### Community 1 - "Value Object Extraction"
Cohesion: 0.03
Nodes (16): parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input(), parse_fields_preserves_generic_with_comma(), parse_fields_single_field(), parse_fields_trims_whitespace(), parse_fields_two_fields(), ParsedField (+8 more)

### Community 3 - "csrf_middleware.rs"
Cohesion: 0.05
Nodes (34): constant_time_eq(), CSRF_FORM_FIELD, CSRF_HEADER, csrf_middleware(), CSRF_SESSION_KEY, CsrfToken, encode_hex(), ensure_session_token() (+26 more)

### Community 4 - "gluon-build/src/lib.rs"
Cohesion: 0.05
Nodes (6): empty_generated(), empty_generated_contains_router_fn(), HTTP_METHODS, preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "Destroy Command Logic"
Cohesion: 0.09
Nodes (32): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+24 more)

### Community 6 - "flash.rs"
Cohesion: 0.23
Nodes (7): Flash, FLASH_KEY, new_session(), set_and_take_round_trip(), set_multiple_keys_round_trip(), take_clears_state(), take_when_unset_returns_empty()

### Community 7 - "Wiring File Parsing"
Cohesion: 0.08
Nodes (19): BIND_CLOSE_PREFIX, BIND_OPEN_PREFIX, BINDS_END, BINDS_START, Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker() (+11 more)

### Community 8 - "redirect.rs"
Cohesion: 0.26
Nodes (8): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, to_sets_303_and_location(), url_with_query_string_preserved()

### Community 9 - "AppError"
Cohesion: 0.13
Nodes (17): AppError, BadRequest, Conflict, Forbidden, Internal, NotFound, Unauthorized, Validation (+9 more)

### Community 11 - "session.rs"
Cohesion: 0.08
Nodes (6): PostgresSessionStore, FaultStore, assert_snapshot_save_is_last_write_wins(), counter(), postgres_sessions_persist_across_instances(), server()

### Community 12 - "gluon application"
Cohesion: 0.33
Nodes (5): Authentication (opt-in), CLI, Conventions, gluon application, Verification

### Community 13 - "gluon application"
Cohesion: 0.33
Nodes (5): Authentication (opt-in), CLI, Conventions, gluon application, Verification

### Community 14 - "inject.rs"
Cohesion: 0.17
Nodes (6): empty_parts(), Inject<T>, resolves_arc_when_bound(), returns_internal_error_when_unbound(), TestImpl, TestTrait

### Community 15 - "BuildError"
Cohesion: 0.15
Nodes (14): build_error_display_io(), build_error_display_other(), build_error_display_syn(), build_error_source_propagates(), BuildError, Io, Other, Syn (+6 more)

### Community 16 - "fresh_app"
Cohesion: 0.19
Nodes (23): fresh_app(), run_gluon_yes(), destroy_migration_uses_exact_match(), domain_collision_leaves_generated_files_and_registrations_unchanged(), dto_dangling_output_symlink_is_rejected_without_side_effects(), full_lifecycle_builds_after_each_generate(), controller_api_flag_skips_tsx(), controller_without_api_creates_both() (+15 more)

### Community 17 - "String"
Cohesion: 0.22
Nodes (10): emit_router_fn(), emit_router_fn_uses_axum08_path_syntax(), is_group_segment(), mangle_segment_for_mod(), mod_path_for(), sanitize_ident(), strip_brackets(), template_path_expr() (+2 more)

### Community 18 - "generate_domain"
Cohesion: 0.17
Nodes (17): ensure_no_existing_file(), ensure_no_existing_outputs(), ensure_no_symlinked_parent_dirs(), extract_value_objects(), generate_domain(), generate_dto(), generate_migration(), generate_usecase() (+9 more)

### Community 19 - "oauth.rs"
Cohesion: 0.06
Nodes (23): oauth_callback(), OAuthExchange, OAuthIdentity, OAuthProvider, authorization(), failure(), GitHubProvider, GoogleProvider (+15 more)

### Community 20 - "examples.rs"
Cohesion: 0.21
Nodes (15): auth_command(), checked_in_examples_match_their_contracts(), client(), compile_examples(), repository_root(), response_json(), run_api(), run_auth() (+7 more)

### Community 21 - "Entity Derive Macro"
Cohesion: 0.15
Nodes (13): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+5 more)

### Community 22 - "Commands"
Cohesion: 0.20
Nodes (10): Cli, Commands, Build, Db, Destroy, Dev, Generate, New (+2 more)

### Community 23 - "db.rs"
Cohesion: 0.19
Nodes (10): run(), run_async(), seed_requires_seed_file(), DbOp, Create, Drop, Migrate, Prepare (+2 more)

### Community 24 - "DestroyKind"
Cohesion: 0.29
Nodes (7): DestroyKind, Controller, Domain, Dto, Migration, Resource, Usecase

### Community 25 - "dev.rs"
Cohesion: 0.05
Nodes (28): make_event(), run(), should_restart(), spawn_app(), AgentSupport, copy_skill(), create_skill_symlink(), expand_scaffold() (+20 more)

### Community 26 - "path"
Cohesion: 0.18
Nodes (8): git_output(), main(), database_lifecycle_and_seed_work_against_postgres(), failed_migration_rolls_back_and_can_be_retried(), gluon_bin(), run_gluon(), generated_postgres_repository_supports_crud(), run()

### Community 27 - "scan"
Cohesion: 0.30
Nodes (12): generate(), generate_emits_concat_env_for_tsx(), generate_emits_layer_for_page_with_tsx(), generate_omits_layer_for_route_rs(), scan(), scan_deep_nesting(), scan_finds_route_and_page_in_different_dirs(), scan_finds_simple_page() (+4 more)

### Community 28 - "Mod.rs Insertion Utilities"
Cohesion: 0.39
Nodes (8): insert_pub_mod_if_present(), insert_pub_mod_inserts_in_sorted_order(), insert_pub_mod_is_idempotent_for_same_name(), insert_pub_mod_is_no_op_when_file_missing(), insert_pub_mod_is_no_op_when_marker_missing(), insert_pub_mod_preserves_crlf(), insert_pub_mod_sorts_inserts(), write_mod_rs()

### Community 29 - "auth_oauth_providers.rs"
Cohesion: 0.13
Nodes (31): claims(), client_config(), current_claims(), discovery_cannot_substitute_a_different_issuer(), exchange(), generic_oidc_discovers_trusted_issuer_and_resolves_verified_subject(), github_authorization_uses_configured_callback_state_and_pkce(), github_identity_survives_login_name_and_email_changes() (+23 more)

### Community 30 - "review-pr SKILL.md"
Cohesion: 0.09
Nodes (29): smartcrabai/agent-skills (upstream skills repo), ai-antipattern skill (companion), .claude/skills directory, improve-review-from-pr-gap SKILL.md, improve-review-from-session SKILL.md, review-pr SKILL.md, review-uncommitted SKILL.md, code-review skill (companion) (+21 more)

### Community 31 - "csrf_spec.rs"
Cohesion: 0.05
Nodes (28): serve_file(), service(), oauth_http_redirect_is_not_followed(), oauth_http_response_limit_allows_exact_boundary_but_rejects_one_byte_more(), oauth_http_timeout_bounds_a_nonresponding_provider(), Server, admitted_only(), application() (+20 more)

### Community 32 - "boot.rs"
Cohesion: 0.10
Nodes (13): clear(), counter(), insecure_development_mode_allows_ephemeral_secret(), read_secure_cookie_env_default_is_true(), read_secure_cookie_env_disabled_when_1(), read_secure_cookie_env_disabled_when_true_caseless(), read_secure_cookie_env_enabled_for_other_values(), secure_cookie_requires_long_secret() (+5 more)

### Community 33 - "check-csrf-runners.test.cjs"
Cohesion: 0.23
Nodes (3): checkReceipt(), model, validate()

### Community 34 - "http.rs"
Cohesion: 0.20
Nodes (26): authentication_finish(), authentication_start(), AuthenticationStart, Callback, csrf(), EmailRequest, Finish, logout() (+18 more)

### Community 35 - "scan_app_dir"
Cohesion: 0.40
Nodes (3): extract_http_methods(), is_pub_async(), scan_app_dir()

### Community 36 - "e2e_destroy_loop.rs"
Cohesion: 0.17
Nodes (12): assert_destroy_cancelled_without_changes(), DESTROY_CASES, destroy_declined_confirmation_preserves_generated_files_and_registrations(), destroy_eof_preserves_generated_files_and_registrations(), destroy_later_decline_keeps_every_target_unchanged(), destroy_resource_cleans_empty_dirs(), DOMAIN_FILES, DTO_FILES (+4 more)

### Community 37 - "common/mod.rs"
Cohesion: 0.31
Nodes (6): drain_to_void(), pick_port(), run_cargo_build(), wait_until_ready(), csrf_blocks_state_changing_without_token(), http_smoke_serves_basic_routes()

### Community 38 - "package.json"
Cohesion: 0.17
Nodes (12): devDependencies, @informalsystems/quint@0.29.1, engines, node, npm, name, overrides, adm-zip (+4 more)

### Community 39 - "release.yml"
Cohesion: 0.17
Nodes (11): cargo-dist (axodotdev), env, ARTIFACT_RETENTION_DAYS, name, permissions, contents, tags, True (+3 more)

### Community 40 - "AuthError"
Cohesion: 0.13
Nodes (17): AuthError, AuthFlow, UserId, VerifiedIdentity, Auth, Ceremony, check_deletion(), decode_key() (+9 more)

### Community 41 - "ContainerBuilder"
Cohesion: 0.14
Nodes (7): ContainerBuilder, Factory, build_container(), build_container(), build_container(), build_container(), build_container()

### Community 42 - "GenerateKind"
Cohesion: 0.29
Nodes (7): GenerateKind, Controller, Domain, Dto, Migration, Resource, Usecase

### Community 43 - "auth_http.rs"
Cohesion: 0.07
Nodes (26): AuthContext, BROWSER_KEY, CurrentUser, internal(), assert_cors_for_configured_origin(), assert_extractor_session_failure(), assert_flow_response(), assert_no_store() (+18 more)

### Community 44 - "store.rs"
Cohesion: 0.09
Nodes (19): OAuthStart, AuthStore, check_schema_connection(), load(), lock(), LOCK_ID, LOGIN_MIGRATION, MemoryAuthStore (+11 more)

### Community 45 - "auth_passkey.rs"
Cohesion: 0.09
Nodes (46): anonymous_cannot_delete_an_existing_credential(), anonymous_cannot_list_credentials(), anonymous_registration_finish_cannot_choose_a_victim_user(), anonymous_registration_start_rejects(), another_user_cannot_delete_owner_credential(), assert_option_schema(), assert_public_only(), assertion() (+38 more)

### Community 46 - "JwtKey"
Cohesion: 0.20
Nodes (10): Auth, invalid_config(), invalid_token(), public_key(), validate_jwt(), JwtConfig, JwtKey, jwt_config() (+2 more)

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

### Community 66 - "common/auth/mod.rs"
Cohesion: 0.23
Nodes (5): OtpEmail, config(), Fixture, Mailbox, TestClock

### Community 78 - "Entry"
Cohesion: 0.22
Nodes (9): build_mod_tree(), build_mod_tree_merges_page_and_route_at_same_path(), emit_mod_tree(), Entry, entry_url_path_delegates_to_url_path_for(), generate_includes_router_call_for_entry(), make_entry(), ModNode (+1 more)

### Community 79 - "validate_route"
Cohesion: 0.13
Nodes (20): controller_output_paths(), ControllerOutputs, Api, Pages, ensure_controller_outputs(), generate_controller(), generate_resource(), render_controller() (+12 more)

### Community 80 - ".run"
Cohesion: 0.22
Nodes (5): add_test_header(), init_tracing(), read_secure_cookie_env(), shutdown_signal(), telemetry_disabled()

### Community 81 - "check-csrf-quint.sh"
Cohesion: 0.25
Nodes (4): check-csrf-alloy.sh script, QUINT_HOME, check-csrf-quint.sh script, setup-csrf-specs.sh script

### Community 83 - "jobs"
Cohesion: 0.18
Nodes (13): env, if, needs, runs-on, env, needs, runs-on, env (+5 more)

### Community 85 - "view.rs"
Cohesion: 0.17
Nodes (10): body_string(), Greeting, invalid_template_path_returns_500(), MIN_PAGE, task_local_isolation(), task_local_template_renders_html(), template_missing_returns_500(), View<P> (+2 more)

### Community 86 - "CSRF specifications verification evidence"
Cohesion: 0.13
Nodes (11): CsrfToken(), CI and local artifacts, Concurrent snapshot follow-up, Contract and production mappings, CSRF specifications verification evidence, Exact executed commands and observed results, Exact remaining blocker, Files and repairs (+3 more)

### Community 88 - "EmailSender"
Cohesion: 0.17
Nodes (4): SmtpEmailSender, EmailSender, UnusedSender, PausedSender

### Community 89 - "auth_jwt.rs"
Cohesion: 0.08
Nodes (28): TokenClaims, anonymous_session_cannot_mint_tokens(), fixture(), issued_token_is_verifiable_using_only_public_jwks_and_session_subject(), jwks_contains_public_ed25519_keys_but_no_private_material(), logout_prevents_new_tokens_but_does_not_revoke_offline_tokens(), rejected_claim!, required_claim! (+20 more)

### Community 90 - "auth_boot.rs"
Cohesion: 0.19
Nodes (19): OtpChallenge, assert_cookie_only_token_authentication(), boot_child(), boot_csrf_rejections_are_not_cached_only_under_the_auth_mount(), boot_http_and_injected_direct_api_use_the_with_container_auth_override(), boot_http_disabled_leaves_auth_in_di_but_mounts_no_auth_endpoints(), boot_otp_rotates_session_csrf_and_preserves_cookie_only_authentication(), boot_public_jwks_is_cacheable_without_session_or_private_key_material() (+11 more)

### Community 92 - "auth_contracts.rs"
Cohesion: 0.16
Nodes (17): absent_database_does_not_silently_select_memory_storage(), active_jwt_kid_must_reference_a_configured_key(), base(), builder_default_attempt_limit_is_five(), builder_default_otp_is_eight_digits(), builder_default_otp_lifetime_is_five_minutes(), builder_default_resend_interval_is_sixty_seconds(), callbackless_provider_accepts_https_fallback_with_custom_mount() (+9 more)

### Community 93 - "htmx_middleware.rs"
Cohesion: 0.26
Nodes (9): flag_is_false_with_false_header_value(), flag_is_false_without_header(), flag_is_true_case_insensitive(), flag_is_true_with_header_present(), htmx_view_returns_fragment_while_normal_request_returns_document(), page(), PageProps, server() (+1 more)

### Community 94 - ".context"
Cohesion: 0.06
Nodes (70): concurrent_verification_of_one_challenge_succeeds_once(), configured_maximum_failed_attempts_permanently_exhausts_challenge(), delivered_code_is_eight_decimal_characters_and_authenticates_once(), delivery_failure_invalidates_only_failed_challenge(), destination_limit_is_shared_across_sources(), disabled_user_cannot_login_or_remain_authenticated(), identity_resolution_is_shared_across_auth_instances(), incorrect() (+62 more)

### Community 95 - "steps"
Cohesion: 0.39
Nodes (12): steps, steps, steps, steps, steps, steps, id, if (+4 more)

### Community 96 - "arc"
Cohesion: 0.27
Nodes (4): Inject, get(), DefaultGreetingService, GreetingService

### Community 97 - "mail.rs"
Cohesion: 0.25
Nodes (3): SmtpConfig, ServerTask, smtp_without_tls_cannot_receive_credentials_or_otp_message()

### Community 98 - "result"
Cohesion: 0.14
Nodes (9): Result, View, get(), get(), PageProps, get(), HomeProps, get() (+1 more)

### Community 99 - "with"
Cohesion: 0.20
Nodes (10): with, merge-multiple, name, path, pattern, persist-credentials, repository, retention-days (+2 more)

### Community 100 - "HtmxRequest"
Cohesion: 0.25
Nodes (3): extractor_defaults_when_middleware_was_skipped(), HtmxRequest, handler()

### Community 101 - "build-local-artifacts"
Cohesion: 0.22
Nodes (9): container, if, name, needs, runs-on, strategy, build-local-artifacts, fail-fast (+1 more)

### Community 102 - "publish-homebrew-formula"
Cohesion: 0.25
Nodes (8): GITHUB_EMAIL, GITHUB_USER, PLAN, publish-homebrew-formula, env, if, needs, runs-on

### Community 103 - "host"
Cohesion: 0.29
Nodes (7): env, if, needs, outputs, runs-on, host, val

### Community 104 - "UserId"
Cohesion: 0.32
Nodes (3): RegistrationPolicy, ExistingSubject, Reject

### Community 105 - "process.rs"
Cohesion: 0.27
Nodes (5): csrf(), fixture_process(), Mailbox, otp_session_jwt_logout_through_shared_example_process(), Process

### Community 106 - "service.rs"
Cohesion: 0.17
Nodes (10): AuthUser, error(), random_id(), Auth, Login, normalize_email(), otp_mac(), OtpState (+2 more)

### Community 107 - "plan"
Cohesion: 0.29
Nodes (7): plan, publishing, tag, tag-flag, env, outputs, runs-on

### Community 108 - "common/mod.rs"
Cohesion: 0.17
Nodes (8): ChildGuard, fix_paths(), git_dep(), gluon_bin(), run_gluon(), run_gluon_expect_failure(), workspace_root(), new_agent_flags_install_expected_files()

### Community 111 - "session_server"
Cohesion: 0.40
Nodes (3): insecure_development_cookie_omits_secure_attribute(), session_cookie_is_signed_and_hardened(), session_server()

### Community 112 - "env"
Cohesion: 0.40
Nodes (5): ANNOUNCEMENT_BODY, ANNOUNCEMENT_TITLE, PRERELEASE_FLAG, RELEASE_COMMIT, env

### Community 114 - "EnabledMethod"
Cohesion: 0.50
Nodes (4): EnabledMethod, Email, None, OAuth

### Community 119 - "AuthConfig"
Cohesion: 0.12
Nodes (9): AuthBuilder, AuthConfig, Clock, EmailOtpConfig, PasskeyConfig, SystemClock, validate_config(), main() (+1 more)

## Knowledge Gaps
- **50 isolated node(s):** `gluon-build`, `gluon-cli`, `Pages`, `New`, `Generate` (+45 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 603 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **35 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `context()` connect `dev.rs` to `Value Object Extraction`, `common/auth/mod.rs`, `http.rs`, `Destroy Command Logic`, `Wiring File Parsing`, `auth_passkey.rs`, `db.rs`, `auth_jwt.rs`, `.context`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **What connects `gluon-build`, `gluon-cli`, `Pages` to the rest of the system?**
  _50 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Value Object Extraction` be split into smaller, more focused modules?**
  _Cohesion score 0.03221029248426509 - nodes in this community are weakly interconnected._
- **Why does `AuthError` connect `AuthError` to `http.rs`, `result`, `common/auth/mod.rs`, `UserId`, `process.rs`, `service.rs`, `auth_http.rs`, `store.rs`, `JwtKey`, `oauth.rs`, `AuthConfig`, `EmailSender`, `auth_jwt.rs`, `auth_boot.rs`, `auth_oauth_providers.rs`, `.context`?**
  _High betweenness centrality (0.059) - this node is a cross-community bridge._
- **Should `Boot` be split into smaller, more focused modules?**
  _Cohesion score 0.11578947368421053 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `result` to `boot.rs`, `Value Object Extraction`, `arc`, `csrf_middleware.rs`, `Destroy Command Logic`, `Wiring File Parsing`, `AppError`, `deserialize`, `dev.rs`?**
  _High betweenness centrality (0.039) - this node is a cross-community bridge._
- **Should `csrf_middleware.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05200341005967604 - nodes in this community are weakly interconnected._