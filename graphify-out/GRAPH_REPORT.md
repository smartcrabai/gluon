# Graph Report - pr-68  (2026-10-30)

## Corpus Check
- 169 files · ~81,507 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 16 file(s) not represented in the graph (top: (none) 9, .qnt 2, .feature 1)

## Summary
- 2168 nodes · 4790 edges · 125 communities (89 shown, 36 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 196 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `0fc2fc8e`
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
- common/auth/mod.rs
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
- csrf_middleware.rs
- path
- src/sse.rs
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
- auth_postgres.rs
- e2e_destroy_loop.rs
- common/mod.rs
- package.json
- release.yml
- .finish_passkey_authentication
- ContainerBuilder
- tests/sse.rs
- auth_http.rs
- store.rs
- auth_passkey.rs
- JwtKey
- with
- Remote Session Script
- .context
- Server-Sent Events
- Authentication
- gluon
- new.rs
- AGENTS.md
- gluon-build
- Entry
- validate_route
- .run
- check-csrf-quint.sh
- new.rs
- jobs
- auth_oauth_transport.rs
- view.rs
- CSRF specifications verification evidence
- auth_session.rs
- mail.rs
- auth_jwt.rs
- auth_boot.rs
- entity_basic.rs
- auth_contracts.rs
- htmx_middleware.rs
- auth_oauth.rs
- steps
- Inject
- testing_dogfood.rs
- result
- with
- templating.rs
- build-local-artifacts
- publish-homebrew-formula
- host
- passkey.rs
- process.rs
- service.rs
- concurrent_echo
- common/mod.rs
- [id]/route.rs
- Container
- Command
- env
- testing.rs
- prelude.rs
- CsrfToken
- TelemetryConfig
- middleware/static_files.rs
- extract_http_methods
- AuthConfig
- tests/static_files.rs
- post
- health/route.rs

## God Nodes (most connected - your core abstractions)
1. `AuthError` - 116 edges
2. `AuthContext` - 58 edges
3. `Auth` - 42 edges
4. `fixture()` - 39 edges
5. `CsrfWorld` - 33 edges
6. `fresh_app()` - 30 edges
7. `ContainerBuilder` - 28 edges
8. `AuthUser` - 26 edges
9. `AuthConfig` - 26 edges
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
- 2-file cycle: `crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/store.rs -> crates/gluon/src/auth/mod.rs`
- 2-file cycle: `crates/gluon/src/auth/mail.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/mail.rs`
- 2-file cycle: `crates/gluon/src/auth/extractors.rs -> crates/gluon/src/auth/mod.rs -> crates/gluon/src/auth/extractors.rs`

## Hyperedges (group relationships)
- **Skills sourced from and kept in sync with the upstream smartcrabai/agent-skills repo** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, agent_skills_repo, github_workflows_update_skills [EXTRACTED 0.85]
- **gluon-cli skill + its reference documentation set** — skills_gluon_cli_skill, skills_gluon_cli_references_commands, skills_gluon_cli_references_conventions, skills_gluon_cli_references_environment, skills_gluon_cli_references_limitations, skills_gluon_cli_references_testing, skills_gluon_cli_references_validation, skills_gluon_cli_references_workflows [EXTRACTED 0.90]
- **Review perspective extraction & consumption ecosystem** — claude_skills_improve_review_from_pr_gap_skill, claude_skills_improve_review_from_session_skill, claude_skills_review_pr_skill, claude_skills_review_uncommitted_skill, dot_review_directory [EXTRACTED 0.90]

## Communities (125 total, 36 thin omitted)

### Community 0 - "container.rs"
Cohesion: 0.20
Nodes (14): Aloha, bind_instance_short_circuits_factory(), bind_same_type_twice_uses_last(), default_builder_builds_empty_container(), factories_see_prior_bindings_during_build(), factory_can_resolve_multiple_prior_bindings(), Greeter, Hello (+6 more)

### Community 1 - "Value Object Extraction"
Cohesion: 0.03
Nodes (16): parse_fields(), parse_fields_accepts_qualified_path_after_colon(), parse_fields_empty_input(), parse_fields_preserves_generic_with_comma(), parse_fields_single_field(), parse_fields_trims_whitespace(), parse_fields_two_fields(), ParsedField (+8 more)

### Community 3 - "csrf.rs"
Cohesion: 0.15
Nodes (13): constant_time_eq(), CSRF_FORM_FIELD, CSRF_HEADER, csrf_middleware(), CSRF_SESSION_KEY, encode_hex(), ensure_session_token(), extract_form_token() (+5 more)

### Community 4 - "gluon-build/src/lib.rs"
Cohesion: 0.05
Nodes (4): HTTP_METHODS, preserves_unicode(), rust_string_literal(), rust_string_literal_str()

### Community 5 - "Destroy Command Logic"
Cohesion: 0.09
Nodes (32): confirm(), confirm_and_remove(), destroy_controller(), destroy_domain(), destroy_dto(), destroy_migration(), destroy_resource(), destroy_usecase() (+24 more)

### Community 6 - "flash.rs"
Cohesion: 0.29
Nodes (7): Flash, FLASH_KEY, new_session(), set_and_take_round_trip(), set_multiple_keys_round_trip(), take_clears_state(), take_when_unset_returns_empty()

### Community 7 - "Wiring File Parsing"
Cohesion: 0.08
Nodes (19): BIND_CLOSE_PREFIX, BIND_OPEN_PREFIX, BINDS_END, BINDS_START, Block, insert_and_replace_round_trip(), insert_bind(), line_is_close_marker() (+11 more)

### Community 8 - "redirect.rs"
Cohesion: 0.26
Nodes (8): empty_url_is_rejected(), external_locations_are_rejected(), invalid_location_becomes_500_internal_error(), is_local_location(), permanent_sets_301(), Redirect, to_sets_303_and_location(), url_with_query_string_preserved()

### Community 9 - "common/auth/mod.rs"
Cohesion: 0.17
Nodes (7): EmailSender, OtpEmail, PausedSender, config(), Fixture, Mailbox, TestClock

### Community 10 - "gluon-cli/src/main.rs"
Cohesion: 0.05
Nodes (25): Cli, Commands, Build, Db, Destroy, Dev, Generate, New (+17 more)

### Community 11 - "session.rs"
Cohesion: 0.12
Nodes (5): PostgresSessionStore, assert_snapshot_save_is_last_write_wins(), counter(), postgres_sessions_persist_across_instances(), server()

### Community 12 - "gluon application"
Cohesion: 0.29
Nodes (6): Authentication (opt-in), CLI, Conventions, gluon application, Pages, layouts, and errors, Verification

### Community 13 - "gluon application"
Cohesion: 0.29
Nodes (6): Authentication (opt-in), CLI, Conventions, gluon application, Pages, layouts, and errors, Verification

### Community 14 - "inject.rs"
Cohesion: 0.17
Nodes (6): empty_parts(), Inject<T>, resolves_arc_when_bound(), returns_internal_error_when_unbound(), TestImpl, TestTrait

### Community 15 - "BuildError"
Cohesion: 0.14
Nodes (14): build_error_display_io(), build_error_display_other(), build_error_source_propagates(), BuildError, Io, Other, Syn, check_url_collisions() (+6 more)

### Community 16 - "fresh_app"
Cohesion: 0.20
Nodes (22): fresh_app(), destroy_migration_uses_exact_match(), domain_collision_leaves_generated_files_and_registrations_unchanged(), dto_dangling_output_symlink_is_rejected_without_side_effects(), full_lifecycle_builds_after_each_generate(), controller_api_flag_skips_tsx(), controller_without_api_creates_both(), db_seed_requires_database_url() (+14 more)

### Community 17 - "String"
Cohesion: 0.20
Nodes (11): emit_router_fn(), emit_router_fn_uses_axum08_path_syntax(), is_group_segment(), mangle_segment_for_mod(), mod_path_for(), page_layout_paths(), sanitize_ident(), strip_brackets() (+3 more)

### Community 18 - "generate_domain"
Cohesion: 0.17
Nodes (17): ensure_no_existing_file(), ensure_no_existing_outputs(), ensure_no_symlinked_parent_dirs(), extract_value_objects(), generate_domain(), generate_dto(), generate_migration(), generate_usecase() (+9 more)

### Community 19 - "oauth.rs"
Cohesion: 0.05
Nodes (26): OAuthExchange, OAuthIdentity, OAuthProvider, authorization(), client_builder(), failure(), GitHubProvider, GoogleProvider (+18 more)

### Community 20 - "examples.rs"
Cohesion: 0.16
Nodes (16): ChildGuard, auth_command(), checked_in_examples_match_their_contracts(), client(), compile_examples(), repository_root(), response_json(), run_api() (+8 more)

### Community 21 - "Entity Derive Macro"
Cohesion: 0.15
Nodes (13): derive_entity(), expand_entity(), expand_entity_fails_on_enum(), expand_entity_fails_on_tuple_struct(), expand_entity_fails_with_multiple_id_attributes(), expand_entity_fails_without_id_attribute(), expand_entity_preserves_generics_and_where_clause(), expand_entity_succeeds_with_id_field() (+5 more)

### Community 22 - "csrf_middleware.rs"
Cohesion: 0.15
Nodes (13): build_app(), concurrent_unsafe_requests_keep_token_and_restore_each_body(), get_passes_through_and_sets_token(), head_and_options_are_safe(), options_handler(), post_exceeding_max_body_returns_413(), post_with_mismatched_token_is_forbidden(), post_with_valid_form_token_passes() (+5 more)

### Community 23 - "path"
Cohesion: 0.15
Nodes (11): run(), run_async(), seed_requires_seed_file(), DbOp, Create, Drop, Migrate, Prepare (+3 more)

### Community 24 - "src/sse.rs"
Cohesion: 0.16
Nodes (3): Sse, Sse<S>, view_event()

### Community 25 - "dev.rs"
Cohesion: 0.13
Nodes (4): make_event(), run(), should_restart(), spawn_app()

### Community 26 - "path"
Cohesion: 0.31
Nodes (6): database_lifecycle_and_seed_work_against_postgres(), failed_migration_rolls_back_and_can_be_retried(), gluon_bin(), run_gluon(), generated_postgres_repository_supports_crud(), run()

### Community 27 - "scan"
Cohesion: 0.26
Nodes (15): generate(), generate_collects_nested_and_route_group_layouts(), generate_emits_concat_env_for_tsx(), generate_emits_page_render_middleware(), generate_omits_layer_for_route_without_tsx(), generate_omits_page_middleware_for_route_rs(), generate_without_entries_contains_router_fn(), scan() (+7 more)

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
Cohesion: 0.07
Nodes (22): FaultStore, admitted_only(), application(), backend_error(), binary(), COOKIE, cross_session(), CsrfWorld (+14 more)

### Community 32 - "boot.rs"
Cohesion: 0.06
Nodes (28): bind_address_environment_overrides_server_config(), build_session_layer(), clear(), counter(), database_url_env_selects_database_variable(), insecure_development_cookie_omits_secure_attribute(), insecure_development_mode_allows_ephemeral_secret(), read_secure_cookie_env_default_is_true() (+20 more)

### Community 33 - "check-csrf-runners.test.cjs"
Cohesion: 0.12
Nodes (11): assert, checkReceipt(), fs, assert, fs, model, os, path (+3 more)

### Community 34 - "http.rs"
Cohesion: 0.20
Nodes (26): authentication_finish(), authentication_start(), AuthenticationStart, Callback, csrf(), EmailRequest, Finish, logout() (+18 more)

### Community 35 - "auth_postgres.rs"
Cohesion: 0.20
Nodes (17): callbackless_provider_uses_first_https_origin_and_custom_mount_for_exchange(), Database, postgres_auth_store_contracts_cleanup_removes_expired_flows_only(), postgres_auth_store_contracts_concurrent_migration_preserves_application_history(), postgres_auth_store_contracts_consume_challenge_once_across_independent_pools(), postgres_auth_store_contracts_delivery_wait_does_not_hold_flow_locks(), postgres_auth_store_contracts_do_not_persist_plaintext_otp(), postgres_auth_store_contracts_failed_migration_rolls_back_and_is_retryable() (+9 more)

### Community 36 - "e2e_destroy_loop.rs"
Cohesion: 0.15
Nodes (13): run_gluon_yes(), assert_destroy_cancelled_without_changes(), DESTROY_CASES, destroy_declined_confirmation_preserves_generated_files_and_registrations(), destroy_eof_preserves_generated_files_and_registrations(), destroy_later_decline_keeps_every_target_unchanged(), destroy_resource_cleans_empty_dirs(), DOMAIN_FILES (+5 more)

### Community 37 - "common/mod.rs"
Cohesion: 0.40
Nodes (6): drain_to_void(), pick_port(), run_cargo_build(), wait_until_ready(), csrf_blocks_state_changing_without_token(), http_smoke_serves_basic_routes()

### Community 38 - "package.json"
Cohesion: 0.17
Nodes (12): devDependencies, @informalsystems/quint@0.29.1, engines, node, npm, name, overrides, adm-zip (+4 more)

### Community 39 - "release.yml"
Cohesion: 0.17
Nodes (11): cargo-dist (axodotdev), env, ARTIFACT_RETENTION_DAYS, name, on, pull_request, push, permissions (+3 more)

### Community 40 - ".finish_passkey_authentication"
Cohesion: 0.24
Nodes (7): AuthFlow, Auth, Ceremony, decode_key(), internal(), invalid(), SavedKey

### Community 41 - "ContainerBuilder"
Cohesion: 0.21
Nodes (6): ContainerBuilder, Factory, build_container(), build_container(), build_container(), build_container()

### Community 42 - "tests/sse.rs"
Cohesion: 0.13
Nodes (7): client(), EventProps, events(), stream_delivers_named_fragment_events_keepalive_and_no_buffer_header(), run(), EventProps, get()

### Community 43 - "auth_http.rs"
Cohesion: 0.10
Nodes (22): assert_cors_for_configured_origin(), assert_extractor_session_failure(), assert_flow_response(), assert_no_store(), assert_passkey_error(), auth_session_and_csrf_responses_are_not_cached(), cors_allows_only_configured_origin(), cors_canonicalizes_configured_origin_with_trailing_slash() (+14 more)

### Community 44 - "store.rs"
Cohesion: 0.09
Nodes (18): AuthStore, check_schema_connection(), load(), lock(), LOCK_ID, LOGIN_MIGRATION, MemoryAuthStore, MIGRATION (+10 more)

### Community 45 - "auth_passkey.rs"
Cohesion: 0.09
Nodes (46): anonymous_cannot_delete_an_existing_credential(), anonymous_cannot_list_credentials(), anonymous_registration_finish_cannot_choose_a_victim_user(), anonymous_registration_start_rejects(), another_user_cannot_delete_owner_credential(), assert_option_schema(), assert_public_only(), assertion() (+38 more)

### Community 46 - "JwtKey"
Cohesion: 0.33
Nodes (6): Auth, invalid_config(), invalid_token(), public_key(), validate_jwt(), TokenClaims

### Community 53 - "with"
Cohesion: 0.06
Nodes (39): permissions, runs-on, steps, timeout-minutes, jobs, csrf-spec, platform-test, test (+31 more)

### Community 62 - ".context"
Cohesion: 0.26
Nodes (16): concurrent_verification_of_one_challenge_succeeds_once(), configured_maximum_failed_attempts_permanently_exhausts_challenge(), delivered_code_is_eight_decimal_characters_and_authenticates_once(), delivery_failure_invalidates_only_failed_challenge(), destination_limit_is_shared_across_sources(), disabled_user_cannot_login_or_remain_authenticated(), identity_resolution_is_shared_across_auth_instances(), incorrect() (+8 more)

### Community 63 - "Server-Sent Events"
Cohesion: 0.40
Nodes (4): HTMX integration, Middleware and proxies, Server-Sent Events, README.md (gluon)

### Community 64 - "Authentication"
Cohesion: 0.17
Nodes (10): Authentication, Development checks, HTTP contract and CSRF, JWT lifetime and operations, Minimal integration, OAuth and Passkeys, Registration policy and business users, Screenless example (+2 more)

### Community 65 - "gluon"
Cohesion: 0.25
Nodes (8): gluon, gluon-macros, sample-api, sample-auth, sample-di, sample-htmx, sample-pages, sample-sessions

### Community 66 - "new.rs"
Cohesion: 0.23
Nodes (12): AgentSupport, copy_skill(), create_skill_symlink(), expand_scaffold(), install_agent_support(), run(), run_cargo_fetch(), run_git_init() (+4 more)

### Community 78 - "Entry"
Cohesion: 0.22
Nodes (9): build_mod_tree(), build_mod_tree_merges_page_and_route_at_same_path(), emit_mod_tree(), Entry, entry_url_path_delegates_to_url_path_for(), generate_includes_router_call_for_entry(), make_entry(), ModNode (+1 more)

### Community 79 - "validate_route"
Cohesion: 0.13
Nodes (20): controller_output_paths(), ControllerOutputs, Api, Pages, ensure_controller_outputs(), generate_controller(), generate_resource(), render_controller() (+12 more)

### Community 80 - ".run"
Cohesion: 0.16
Nodes (9): add_test_header(), bind_address(), bind_config(), boot_config_is_available_to_application_container(), create_pool(), database_pool_applies_configured_max_connections(), read_secure_cookie_env(), shutdown_signal() (+1 more)

### Community 81 - "check-csrf-quint.sh"
Cohesion: 0.25
Nodes (4): check-csrf-alloy.sh script, QUINT_HOME, check-csrf-quint.sh script, setup-csrf-specs.sh script

### Community 82 - "new.rs"
Cohesion: 0.12
Nodes (7): extractor_defaults_when_middleware_was_skipped(), header_is_true(), htmx_middleware(), HtmxRequest, is_htmx_fragment(), observed_flag(), handler()

### Community 83 - "jobs"
Cohesion: 0.15
Nodes (16): env, if, needs, runs-on, env, needs, runs-on, env (+8 more)

### Community 84 - "auth_oauth_transport.rs"
Cohesion: 0.07
Nodes (8): oauth_http_extra_root_certificate_is_opt_in(), oauth_http_redirect_is_not_followed(), oauth_http_response_limit_allows_exact_boundary_but_rejects_one_byte_more(), oauth_http_timeout_bounds_a_nonresponding_provider(), private_key(), Server, test_certificate(), TlsServer

### Community 85 - "view.rs"
Cohesion: 0.06
Nodes (45): AppError, BadRequest, Conflict, Forbidden, Internal, NotFound, Unauthorized, Validation (+37 more)

### Community 86 - "CSRF specifications verification evidence"
Cohesion: 0.15
Nodes (11): CsrfToken(), CI and local artifacts, Concurrent snapshot follow-up, Contract and production mappings, CSRF specifications verification evidence, Exact executed commands and observed results, Exact remaining blocker, Files and repairs (+3 more)

### Community 87 - "auth_session.rs"
Cohesion: 0.26
Nodes (10): activity_extends_idle_expiry_but_never_absolute_expiry(), cached_cookie_uses_authoritative_idle_activity_and_cleanup_cannot_be_undone(), idle_session_is_valid_just_before_expiry_and_invalid_at_exact_expiry(), login_authority_counts_toward_memory_capacity_and_expiry_reclaims_it(), login_payload_must_match_authoritative_user_and_authentication_time(), login_rotates_persisted_id_revokes_old_id_and_discards_anonymous_data(), logout_revokes_persisted_session_across_auth_instances_and_cookie_replay(), preloaded_independent_sessions_cannot_restore_revoked_login_authority() (+2 more)

### Community 88 - "mail.rs"
Cohesion: 0.15
Nodes (3): SmtpEmailSender, ServerTask, smtp_without_tls_cannot_receive_credentials_or_otp_message()

### Community 89 - "auth_jwt.rs"
Cohesion: 0.07
Nodes (32): JwtConfig, JwtKey, anonymous_session_cannot_mint_tokens(), fixture(), issued_token_is_verifiable_using_only_public_jwks_and_session_subject(), jwks_contains_public_ed25519_keys_but_no_private_material(), logout_prevents_new_tokens_but_does_not_revoke_offline_tokens(), rejected_claim! (+24 more)

### Community 90 - "auth_boot.rs"
Cohesion: 0.11
Nodes (22): AuthContext, BROWSER_KEY, CurrentUser, internal(), assert_cookie_only_token_authentication(), boot_child(), boot_csrf_rejections_are_not_cached_only_under_the_auth_mount(), boot_http_and_injected_direct_api_use_the_with_container_auth_override() (+14 more)

### Community 92 - "auth_contracts.rs"
Cohesion: 0.16
Nodes (17): absent_database_does_not_silently_select_memory_storage(), active_jwt_kid_must_reference_a_configured_key(), base(), builder_default_attempt_limit_is_five(), builder_default_otp_is_eight_digits(), builder_default_otp_lifetime_is_five_minutes(), builder_default_resend_interval_is_sixty_seconds(), callbackless_provider_accepts_https_fallback_with_custom_mount() (+9 more)

### Community 93 - "htmx_middleware.rs"
Cohesion: 0.25
Nodes (11): boosted_request_keeps_the_htmx_extractor_flag(), flag_is_false_with_false_header_value(), flag_is_false_without_header(), flag_is_true_case_insensitive(), flag_is_true_with_header_present(), history_restore_keeps_the_htmx_extractor_flag(), htmx_fragment_excludes_layout_but_boosted_and_history_restore_render_documents(), page() (+3 more)

### Community 94 - "auth_oauth.rs"
Cohesion: 0.14
Nodes (27): anonymous_explicit_link_start_rejects(), assert_no_exchange(), concurrent_callbacks_exchange_and_login_only_once(), concurrent_first_logins_for_same_identity_resolve_one_user(), exchange_receives_saved_nonce_and_trusted_callback(), expired_flow_rejects_before_exchange_while_session_remains_valid(), explicit_link_adds_identity_to_authenticated_owner(), explicit_link_rejects_identity_already_owned_by_another_user() (+19 more)

### Community 95 - "steps"
Cohesion: 0.39
Nodes (12): steps, steps, steps, steps, steps, steps, id, if (+4 more)

### Community 96 - "Inject"
Cohesion: 0.27
Nodes (4): get(), build_container(), DefaultGreetingService, GreetingService

### Community 97 - "testing_dogfood.rs"
Cohesion: 0.24
Nodes (3): Inject, missing_dependency(), MissingDependency

### Community 98 - "result"
Cohesion: 0.23
Nodes (8): Result, View, get(), PageProps, get(), HomeProps, get(), UserProps

### Community 99 - "with"
Cohesion: 0.20
Nodes (10): with, merge-multiple, name, path, pattern, persist-credentials, repository, retention-days (+2 more)

### Community 100 - "templating.rs"
Cohesion: 0.21
Nodes (5): Templates, load_environment(), render(), render_existing_template_succeeds(), render_unknown_template_errors()

### Community 101 - "build-local-artifacts"
Cohesion: 0.22
Nodes (9): container, if, name, needs, runs-on, strategy, build-local-artifacts, fail-fast (+1 more)

### Community 102 - "publish-homebrew-formula"
Cohesion: 0.25
Nodes (8): GITHUB_EMAIL, GITHUB_USER, PLAN, publish-homebrew-formula, env, if, needs, runs-on

### Community 103 - "host"
Cohesion: 0.18
Nodes (11): env, if, needs, outputs, runs-on, host, publishing, tag (+3 more)

### Community 104 - "passkey.rs"
Cohesion: 0.12
Nodes (17): RegistrationPolicy, UserId, VerifiedIdentity, check_deletion(), different_configured_provider_does_not_enable_historical_identity(), enabled_email_allows_passkey_deletion(), enabled_matching_provider_allows_passkey_deletion(), EnabledMethod (+9 more)

### Community 105 - "process.rs"
Cohesion: 0.22
Nodes (5): csrf(), fixture_process(), Mailbox, otp_session_jwt_logout_through_shared_example_process(), Process

### Community 106 - "service.rs"
Cohesion: 0.14
Nodes (13): AuthError, AuthUser, error(), OAuthStart, OtpChallenge, random_id(), Auth, Login (+5 more)

### Community 107 - "concurrent_echo"
Cohesion: 0.29
Nodes (3): concurrent_echo(), echo_body(), show_token()

### Community 108 - "common/mod.rs"
Cohesion: 0.25
Nodes (7): fix_paths(), git_dep(), gluon_bin(), run_gluon(), run_gluon_expect_failure(), workspace_root(), new_agent_flags_install_expected_files()

### Community 112 - "env"
Cohesion: 0.40
Nodes (5): ANNOUNCEMENT_BODY, ANNOUNCEMENT_TITLE, PRERELEASE_FLAG, RELEASE_COMMIT, env

### Community 114 - "prelude.rs"
Cohesion: 0.11
Nodes (14): read_database_url(), AppConfig, Config, DatabaseConfig, EMBEDDED_CONFIG, missing_file_uses_defaults(), reads_scaffolded_settings_and_rejects_unsupported_database_settings(), __register_embedded_config() (+6 more)

### Community 116 - "TelemetryConfig"
Cohesion: 0.47
Nodes (5): init_tracing(), telemetry_disabled(), telemetry_enabled(), telemetry_service_name(), TelemetryConfig

### Community 118 - "extract_http_methods"
Cohesion: 0.33
Nodes (4): extract_http_methods(), is_pub_async(), scan_app_dir(), scan_parse_error_includes_source_file_path()

### Community 119 - "AuthConfig"
Cohesion: 0.10
Nodes (11): SmtpConfig, AuthBuilder, AuthConfig, Clock, EmailOtpConfig, oauth_callback(), PasskeyConfig, SystemClock (+3 more)

### Community 120 - "tests/static_files.rs"
Cohesion: 0.33
Nodes (6): directory_index_disabled(), nonexistent_returns_404(), root_created_after_service_is_available(), server_for(), serves_existing_file(), symlink_cannot_escape_public_directory()

### Community 123 - "post"
Cohesion: 0.62
Nodes (3): get(), post(), read_count()

## Knowledge Gaps
- **53 isolated node(s):** `gluon-build`, `Syn`, `gluon-cli`, `Pages`, `New` (+48 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 658 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **36 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AuthError` connect `service.rs` to `http.rs`, `passkey.rs`, `common/auth/mod.rs`, `.finish_passkey_authentication`, `process.rs`, `store.rs`, `JwtKey`, `prelude.rs`, `oauth.rs`, `.context`, `AuthConfig`, `mail.rs`, `auth_jwt.rs`, `auth_boot.rs`, `auth_oauth_providers.rs`, `auth_oauth.rs`?**
  _High betweenness centrality (0.073) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `AuthError` (e.g. with `.exchange()` and `.send()`) actually correct?**
  _`AuthError` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `gluon-build`, `Syn`, `gluon-cli` to the rest of the system?**
  _53 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Value Object Extraction` be split into smaller, more focused modules?**
  _Cohesion score 0.03221029248426509 - nodes in this community are weakly interconnected._
- **Why does `Result` connect `result` to `boot.rs`, `Value Object Extraction`, `new.rs`, `Inject`, `templating.rs`, `Destroy Command Logic`, `Wiring File Parsing`, `tests/sse.rs`, `[id]/route.rs`, `prelude.rs`, `CsrfToken`, `view.rs`, `src/sse.rs`, `post`, `health/route.rs`?**
  _High betweenness centrality (0.060) - this node is a cross-community bridge._
- **Should `Boot` be split into smaller, more focused modules?**
  _Cohesion score 0.11578947368421053 - nodes in this community are weakly interconnected._
- **Why does `context()` connect `path` to `auth_jwt.rs`, `Value Object Extraction`, `new.rs`, `http.rs`, `templating.rs`, `Destroy Command Logic`, `Wiring File Parsing`, `common/auth/mod.rs`, `auth_passkey.rs`, `auth_session.rs`, `auth_oauth.rs`, `dev.rs`, `.context`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._