module csrf {
  // CSRF-01..07 are provisional reconstructed audit IDs (original audit unavailable).
  // Mapping: csrf.rs ensure_session_token, is_safe_method, extract_* and csrf_middleware.
  // Equivalent to csrf.als: sequential requests, two sessions, two collision-free
  // issued tokens plus one wrong token. No expiry/concurrent issuance, timing or
  // entropy claim. Store means Session's logical state, not durable DB commit.
  // Header 0 abstracts absent/empty/non-UTF8 after normalization; form is the
  // first decoded _csrf field, 0 absent/empty. Gherkin tests concrete parsing.
  // Read failure includes lazy backend loading or malformed session data. String
  // insert after get cannot independently fail serialization, so it is not a
  // separate nondeterministic failure. Outer SessionManager save failures are
  // outside the atomic middleware step (and tested separately in Gherkin).
  // selected is a ghost normalized submission, even when safe/store-failed.
  // Sizes 0/1/2 = below/at/above 4MiB. Payloads 0/1 are distinct byte strings.
  // Bounded checks: 6 transitions (Alloy 7 states). No unbounded proof claimed.
  var tokens: int -> int
  var last: { active: bool, before: int -> int, sid: int, safe: bool,
    header: int, form: int, mime: bool, size: int, payload: int,
    readFail: bool, selected: int, result: int, output: int, extension: int }

  action init = all {
    tokens' = Map(1 -> 0, 2 -> 0),
    last' = { active: false, before: Map(1 -> 0, 2 -> 0), sid: 1, safe: true,
      header: 0, form: 0, mime: false, size: 0, payload: 0,
      readFail: false, selected: 0, result: 0, output: -1, extension: 0 },
  }

  action request(s: int, safe: bool, h: int, f: int, mime: bool,
      bodySize: int, payload: int, readFail: bool): bool = {
    val old = tokens.get(s)
    val failed = readFail
    val expected = if (old == 0) s else old
    val selected = if (h != 0) h else if (mime) f else 0
    // Results: 1 admitted, 2 forbidden, 3 too large, 4 store error.
    val result = if (failed) 4 else if (safe) 1 else if (bodySize == 2) 3
      else if (selected != 0 and selected == expected) 1 else 2
    all {
      tokens' = if (failed) tokens else tokens.set(s, expected),
      last' = { active: true, before: tokens, sid: s, safe: safe,
        header: h, form: f, mime: mime, size: bodySize, payload: payload,
        readFail: readFail, selected: selected, result: result,
        output: if (result == 1) payload else -1,
        extension: if (result == 1) expected else 0 },
    }
  }

  action step = {
    nondet s = Set(1, 2).oneOf()
    nondet safe = Set(false, true).oneOf()
    nondet h = Set(0, 1, 2, 3).oneOf()
    nondet f = Set(0, 1, 2, 3).oneOf()
    nondet mime = Set(false, true).oneOf()
    nondet bodySize = Set(0, 1, 2).oneOf()
    nondet payload = Set(0, 1).oneOf()
    nondet readFail = Set(false, true).oneOf()
    request(s, safe, h, f, mime, bodySize, payload, readFail)
  }

  val failed = last.readFail
  // CSRF-01: every successful ensure, even before a 403/413, issues a token.
  val issuance = not(last.active) or failed or tokens.get(last.sid) == last.sid
  // CSRF-01: existing session tokens are never rotated by a request.
  val reuse = not(last.active) or last.before.get(last.sid) == 0
    or tokens.get(last.sid) == last.before.get(last.sid)
  // CSRF-07: processing one session cannot mutate the other.
  val isolation = not(last.active) or Set(1, 2).forall(s =>
    s == last.sid or tokens.get(s) == last.before.get(s))
  // CSRF-02: safe methods bypass both submission and body-size checks.
  val safe_admission = not(last.active) or failed or not(last.safe) or last.result == 1
  // CSRF-03: all unsafe, bounded requests admit iff a nonempty token matches.
  val unsafe_admission = not(last.active) or failed or last.safe or last.size == 2
    or (if (last.selected != 0 and last.selected == tokens.get(last.sid)) last.result == 1 else last.result == 2)
  // CSRF-04: nonempty normalized header wins, including a mismatching header.
  val precedence = not(last.active) or last.selected ==
    (if (last.header != 0) last.header else if (last.mime) last.form else 0)
  // CSRF-05: body failure precedes token validation, but follows store access.
  val body_limit = not(last.active) or failed or last.safe or ((last.result == 3) == (last.size == 2))
  // CSRF-06: admission preserves bytes and exposes token; rejection has no handler effects.
  val payload_preserved = not(last.active) or
    (if (last.result == 1) last.output == last.payload and last.extension == tokens.get(last.sid)
      else last.output == -1 and last.extension == 0)
  // CSRF-07: session load/read failures fail closed without changing logical state.
  val store_failure = not(last.active) or not(failed) or
    (last.result == 4 and tokens == last.before)
  val contract = issuance and reuse and isolation and safe_admission and unsafe_admission
    and precedence and body_limit and payload_preserved and store_failure

  // Twelve reachable witnesses, each executed as a deterministic test, not just counted.
  run issue_test = init.then(request(1, true, 0, 0, false, 0, 0, false))
    .expect(tokens.get(1) == 1 and last.extension == 1 and contract)
  run reuse_test = init.then(request(1, true, 0, 0, false, 0, 0, false))
    .then(request(1, true, 0, 0, false, 0, 1, false))
    .expect(last.before.get(1) == 1 and tokens.get(1) == 1 and contract)
  run safe_large_test = init.then(request(1, true, 3, 0, false, 2, 1, false))
    .expect(last.result == 1 and contract)
  run header_test = init.then(request(1, false, 1, 0, false, 0, 1, false))
    .expect(last.result == 1 and last.output == 1 and contract)
  run form_test = init.then(request(1, false, 0, 1, true, 0, 0, false))
    .expect(last.result == 1 and contract)
  run precedence_test = init.then(request(1, false, 3, 1, true, 0, 0, false))
    .expect(last.result == 2 and last.selected == 3 and contract)
  run missing_test = init.then(request(1, false, 0, 0, true, 0, 0, false))
    .expect(last.result == 2 and tokens.get(1) == 1 and contract)
  run wrong_test = init.then(request(1, false, 0, 3, true, 0, 0, false))
    .expect(last.result == 2 and contract)
  run foreign_test = init.then(request(1, true, 0, 0, false, 0, 0, false))
    .then(request(2, false, 1, 0, false, 0, 0, false))
    .expect(last.result == 2 and tokens.get(1) == 1 and tokens.get(2) == 2 and contract)
  run boundary_test = init.then(request(1, false, 1, 0, false, 1, 1, false))
    .expect(last.result == 1 and contract)
  run oversized_test = init.then(request(1, false, 0, 0, false, 2, 1, false))
    .expect(last.result == 3 and contract)
  run store_failure_test = init.then(request(1, true, 0, 0, false, 0, 0, true))
    .expect(last.result == 4 and tokens.get(1) == 0 and contract)
}
