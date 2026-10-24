module csrf_concurrent {
  // Two pre-existing sessions have distinct CSRF tokens. Two request state
  // machines can interleave load, check, and save for either session.
  // The session's opaque value stands for other persisted data: each load takes
  // a full snapshot, check records token admission and the request-local value
  // change, and save replaces the record with that snapshot (last-save-wins).
  // This models no atomic increment/merge, first-token issuance, database
  // durable commit, expiry, unbounded behavior, or arbitrary I/O failure.
  // Bounded SMT checks cover at most six transitions, enough for two full
  // load/check/save sequences; no unbounded proof is claimed.

  var sessions: int -> { token: int, value: int }
  var phase: int -> int
  var sid: int -> int
  var submitted: int -> int
  var replacement: int -> int
  var snapshotToken: int -> int
  var loadedValue: int -> int
  var snapshotValue: int -> int
  var admitted: int -> bool
  var lastSave: { active: bool, sid: int, token: int, value: int,
    otherSid: int, otherToken: int, otherValue: int }

  action init = all {
    sessions' = Map(1 -> { token: 11, value: 0 }, 2 -> { token: 22, value: 0 }),
    phase' = Map(1 -> 0, 2 -> 0),
    sid' = Map(1 -> 1, 2 -> 1),
    submitted' = Map(1 -> 0, 2 -> 0),
    replacement' = Map(1 -> 0, 2 -> 0),
    snapshotToken' = Map(1 -> 0, 2 -> 0),
    loadedValue' = Map(1 -> 0, 2 -> 0),
    snapshotValue' = Map(1 -> 0, 2 -> 0),
    admitted' = Map(1 -> false, 2 -> false),
    lastSave' = { active: false, sid: 1, token: 0, value: 0,
      otherSid: 2, otherToken: 22, otherValue: 0 },
  }

  action load(r: int, s: int, presented: int, nextValue: int): bool = all {
    phase.get(r) == 0,
    sessions' = sessions,
    phase' = phase.set(r, 1),
    sid' = sid.set(r, s),
    submitted' = submitted.set(r, presented),
    replacement' = replacement.set(r, nextValue),
    snapshotToken' = snapshotToken.set(r, sessions.get(s).token),
    loadedValue' = loadedValue.set(r, sessions.get(s).value),
    snapshotValue' = snapshotValue.set(r, sessions.get(s).value),
    admitted' = admitted.set(r, false),
    lastSave' = lastSave,
  }

  // This action also abstracts an admitted request's local update to unrelated
  // session data; saving remains a separate transition.
  action check(r: int): bool = {
    val matches = submitted.get(r) == snapshotToken.get(r)
    all {
      phase.get(r) == 1,
      sessions' = sessions,
      phase' = phase.set(r, 2),
      sid' = sid,
      submitted' = submitted,
      replacement' = replacement,
      snapshotToken' = snapshotToken,
      loadedValue' = loadedValue,
      snapshotValue' = snapshotValue.set(r,
        if (matches) replacement.get(r) else snapshotValue.get(r)),
      admitted' = admitted.set(r, matches),
      lastSave' = lastSave,
    }
  }

  action save(r: int): bool = {
    val other = if (sid.get(r) == 1) 2 else 1
    all {
      phase.get(r) == 2,
      sessions' = sessions.set(sid.get(r), {
        token: snapshotToken.get(r), value: snapshotValue.get(r),
      }),
      phase' = phase.set(r, 3),
      sid' = sid,
      submitted' = submitted,
      replacement' = replacement,
      snapshotToken' = snapshotToken,
      loadedValue' = loadedValue,
      snapshotValue' = snapshotValue,
      admitted' = admitted,
      lastSave' = { active: true, sid: sid.get(r),
        token: snapshotToken.get(r), value: snapshotValue.get(r),
        otherSid: other, otherToken: sessions.get(other).token,
        otherValue: sessions.get(other).value },
    }
  }

  action step = {
    nondet op = Set(0, 1, 2).oneOf()
    nondet r = Set(1, 2).oneOf()
    if (op == 0) {
      nondet s = Set(1, 2).oneOf()
      nondet presented = Set(0, 11, 22).oneOf()
      nondet nextValue = Set(0, 1, 2).oneOf()
      load(r, s, presented, nextValue)
    } else if (op == 1) {
      check(r)
    } else {
      save(r)
    }
  }

  // Requests use exactly their loaded token; neither requests nor a later
  // whole-record save can rotate the already-present CSRF tokens.
  val token_reuse = sessions.get(1).token == 11 and sessions.get(2).token == 22
    and Set(1, 2).forall(r => phase.get(r) == 0 or snapshotToken.get(r) ==
      (if (sid.get(r) == 1) 11 else 22))
  val admission_from_snapshot = Set(1, 2).forall(r => phase.get(r) < 2 or
    admitted.get(r) == (submitted.get(r) == snapshotToken.get(r)))
  val foreign_token_rejected = Set(1, 2).forall(r => phase.get(r) < 2 or
    (submitted.get(r) != (if (sid.get(r) == 1) 22 else 11) or not(admitted.get(r))))
  val save_uses_snapshot = not(lastSave.active) or
    (sessions.get(lastSave.sid).token == lastSave.token and
      sessions.get(lastSave.sid).value == lastSave.value)
  val save_preserves_other_session = not(lastSave.active) or
    (sessions.get(lastSave.otherSid).token == lastSave.otherToken and
      sessions.get(lastSave.otherSid).value == lastSave.otherValue)
  val contract = token_reuse and admission_from_snapshot and foreign_token_rejected
    and save_uses_snapshot and save_preserves_other_session

  // Both requests load the existing token before either one saves. The
  // interleaving demonstrates reuse and accepted admission without rotation.
  run concurrent_token_reuse_test = init
    .then(load(1, 1, 11, 1))
    .then(load(2, 1, 11, 2))
    .then(check(1)).then(check(2))
    .then(save(1)).then(save(2))
    .expect(admitted.get(1) and admitted.get(2) and sessions.get(1).token == 11
      and contract)

  run foreign_token_test = init
    .then(load(1, 1, 22, 1))
    .then(check(1))
    .expect(not(admitted.get(1)) and foreign_token_rejected and contract)

  run other_session_isolation_test = init
    .then(load(1, 1, 11, 1))
    .then(check(1)).then(save(1))
    .expect(sessions.get(2).token == 22 and sessions.get(2).value == 0 and contract)

  // Both requests loaded value 0; the later save replaces the earlier saved
  // value with its own stale snapshot. This is an allowed lost update, not an
  // assertion that session updates are atomic increments or merges.
  run lost_update_test = init
    .then(load(1, 1, 11, 1))
    .then(load(2, 1, 11, 2))
    .then(check(1)).then(check(2))
    .then(save(1)).then(save(2))
    .expect(loadedValue.get(1) == 0 and loadedValue.get(2) == 0
      and snapshotValue.get(1) == 1 and snapshotValue.get(2) == 2
      and lastSave.value == 2 and sessions.get(1).value == 2
      and lastSave.value != snapshotValue.get(1) and contract)
}
