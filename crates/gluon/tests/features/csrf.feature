# These are PROVISIONAL reconstructed requirements, not original audit/design evidence.
Feature: Production CSRF session lifecycle
  Background:
    Given an isolated production CSRF application

  # PROVISIONAL CSRF-01: issuance, persistence and reuse, including first unsafe contact.
  @CSRF-01
  Scenario: Safe contact issues a persistent reusable token
    When a fresh client requests GET
    Then a persisted token is issued and reused

  # PROVISIONAL CSRF-01: rejection still establishes a token for the fresh session.
  @CSRF-01
  Scenario: Fresh unsafe contact creates a session but cannot invoke the handler
    When a fresh client requests POST
    Then the fresh unsafe request is rejected and its token can be reused

  # PROVISIONAL CSRF-02: only GET, HEAD and OPTIONS bypass validation.
  @CSRF-02
  Scenario Outline: Safe methods require no submitted token
    When a fresh client requests <method>
    Then a persisted token is issued and reused
    Examples:
      | method  |
      | GET     |
      | HEAD    |
      | OPTIONS |

  # PROVISIONAL CSRF-02 / CSRF-05: safe requests bypass both validation and buffering limits.
  @CSRF-02 @CSRF-05 @CSRF-06
  Scenario: Safe oversized body and wrong token bypass unsafe validation
    Given a persisted session token
    When a safe GET sends 4194305 bytes with a wrong header token
    Then the safe request preserves its payload and session token

  # PROVISIONAL CSRF-03: every non-safe method follows the same validation branch.
  @CSRF-03
  Scenario Outline: Unsafe methods validate the session token
    Given a persisted session token
    When the client exercises missing wrong and valid tokens with <method>
    Then only the valid request reaches the handler
    Examples:
      | method  |
      | POST    |
      | PUT     |
      | PATCH   |
      | DELETE  |
      | TRACE   |
      | CONNECT |
      | CUSTOM  |

  # PROVISIONAL CSRF-04: header precedence, trim, fallback and first decoded form field.
  @CSRF-04
  Scenario Outline: Token source selection preserves the original form
    Given a persisted session token
    When the client submits the <case> token selection case
    Then the selection result is <status>
    Examples:
      | case                    | status |
      | trimmed-header          | 200    |
      | wrong-header-wins       | 403    |
      | valid-header-wins       | 200    |
      | empty-header-fallback   | 200    |
      | invalid-header-fallback | 200    |
      | encoded-form            | 200    |
      | first-valid-duplicate   | 200    |
      | first-wrong-duplicate   | 403    |
      | first-empty-duplicate   | 403    |
      | non-form                | 403    |
      | form-token-not-trimmed  | 403    |

  # PROVISIONAL CSRF-05: 4 MiB inclusive, body limit takes precedence over token rejection.
  @CSRF-05
  Scenario Outline: Unsafe body limit is enforced before token validation
    Given a persisted session token
    When a body of <size> bytes is sent with a <token> token
    Then the selection result is <status>
    Examples:
      | size    | token   | status |
      | 4194304 | valid   | 200    |
      | 4194304 | missing | 403    |
      | 4194305 | valid   | 413    |
      | 4194305 | wrong   | 413    |
      | 4194305 | missing | 413    |

  # PROVISIONAL CSRF-06: exact binary preservation and extension only on admission.
  @CSRF-06
  Scenario: Binary bytes survive admission and rejects have no handler effects
    Given a persisted session token
    When binary requests are admitted and rejected
    Then only the valid request reaches the handler

  # PROVISIONAL CSRF-07: session-bound tokens cannot be used by another client.
  @CSRF-07
  Scenario: Independent clients retain isolated session tokens
    Given a persisted session token
    When a second session attempts to use the first session token
    Then the clients remain isolated

  # PROVISIONAL CSRF-07: actual session middleware load/create/save failures return 500.
  # Persistence is deferred until after the handler for create/save failures. This
  # asserts fail-closed HTTP responses, not transactional rollback of handler effects.
  @CSRF-07
  Scenario Outline: Session backend failures fail closed
    Given a persisted session token
    When the session store fails during <operation>
    Then the backend failure returns an internal error
    Examples:
      | operation |
      | load      |
      | create    |
      | save      |

  # PROVISIONAL CSRF-07: Session::get<String> deserialization errors also fail closed.
  @CSRF-07
  Scenario: Malformed persisted token fails before the handler without rotation
    Given a persisted session token
    When the persisted CSRF token has a non-string value
