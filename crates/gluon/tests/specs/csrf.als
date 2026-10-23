module csrf
open util/ordering[State]
// CSRF-01..07: provisional reconstructed audit IDs, matching csrf.qnt.
// Sequential logical Session state, two collision-free issued tokens and one
// wrong token. No entropy, timing, expiry, concurrent issuance or DB durability
// claim. Normalized absent/blank/invalid header = none; first decoded form field
// (absent/empty = none). Read failure includes lazy load or malformed stored data.
// As in Quint, String insert after get has no independent serialization failure.
// Outer SessionManager save failures are outside the atomic middleware step and
// tested separately in Gherkin. selected is a ghost normalized submission.
// Size Below/At/Above abstracts 4MiB, two opaque byte strings abstract payloads.
// Seven ordered states = initial plus six requests, same bound as Quint.
// Quint mapping: A/B=1/2, TA/TB/Wrong=1/2/3, none=0, prior=last.before.
// Below/At/Above=0/1/2, P0/P1=0/1, no output=-1. Result Initial/Admit/
// Forbidden/TooLarge/StoreError=0/1/2/3/4; Yes/No=true/false.
abstract sig Session {}
one sig A, B extends Session {}
abstract sig Token {}
one sig TA, TB, Wrong extends Token {}
abstract sig Bool {}
one sig Yes, No extends Bool {}
abstract sig Size {}
one sig Below, At, Above extends Size {}
abstract sig Payload {}
one sig P0, P1 extends Payload {}
abstract sig Result {}
one sig Initial, Admit, Forbidden, TooLarge, StoreError extends Result {}
sig State {
  tokens, prior: Session -> lone Token,
  active, safe, mime, readFail: one Bool,
  sid: one Session,
  header, form, selected, extension: lone Token,
  bodySize: one Size,
  payload: one Payload,
  result: one Result,
  output: lone Payload
}
fun issued[s: Session]: one Token { s = A => TA else TB }
pred failed[x: State] {
  x.readFail = Yes
}
pred init[x: State] {
  no x.tokens + x.prior
  x.active = No
  x.sid = A
  x.safe = Yes
  no x.header + x.form + x.selected + x.extension + x.output
  x.mime = No
  x.bodySize = Below
  x.payload = P0
  x.readFail = No
  x.result = Initial
}
pred request[x, y: State] {
  y.active = Yes
  y.prior = x.tokens
  y.selected = (some y.header => y.header else (y.mime = Yes => y.form else none))
  failed[y] => {
    y.tokens = x.tokens
    y.result = StoreError
  } else {
    y.tokens = x.tokens ++ y.sid -> (some x.tokens[y.sid] => x.tokens[y.sid] else issued[y.sid])
    y.safe = Yes => y.result = Admit else {
      y.bodySize = Above => y.result = TooLarge else {
        (some y.selected and y.selected = y.tokens[y.sid]) => y.result = Admit else y.result = Forbidden
      }
    }
  }
  y.result = Admit => {
    y.output = y.payload
    y.extension = y.tokens[y.sid]
  } else { no y.output + y.extension }
}
// Required trace constraint, not unconstrained snapshots.
fact Trace {
  init[first]
  all x: State - last | request[x, next[x]]
}
// CSRF-01 ensure_session_token issues even on a later rejection.
assert Issuance { all x: State | x.active = Yes and not failed[x] implies x.tokens[x.sid] = issued[x.sid] }
// CSRF-01 existing tokens are reused.
assert Reuse { all x: State | x.active = Yes and some x.prior[x.sid] implies x.tokens[x.sid] = x.prior[x.sid] }
// CSRF-07 other session state is unchanged.
assert Isolation { all x: State, s: Session - x.sid | x.active = Yes implies x.tokens[s] = x.prior[s] }
// CSRF-02 safe methods bypass token/body checks after successful ensure.
assert SafeAdmission { all x: State | x.active = Yes and not failed[x] and x.safe = Yes implies x.result = Admit }
// CSRF-03 unsafe bounded admission iff selected token matches.
assert UnsafeAdmission { all x: State | x.active = Yes and not failed[x] and x.safe = No and x.bodySize != Above implies
  ((some x.selected and x.selected = x.tokens[x.sid]) => x.result = Admit else x.result = Forbidden) }
// CSRF-04 valid normalized header always wins over the form.
assert Precedence { all x: State | x.active = Yes implies
  x.selected = (some x.header => x.header else (x.mime = Yes => x.form else none)) }
// CSRF-05 body check precedes token rejection, inclusive limit.
assert BodyLimit { all x: State | x.active = Yes and not failed[x] and x.safe = No implies
  (x.result = TooLarge iff x.bodySize = Above) }
// CSRF-06 only admitted requests expose original bytes and token downstream.
assert PayloadPreserved { all x: State | x.active = Yes implies
  (x.result = Admit => (x.output = x.payload and x.extension = x.tokens[x.sid]) else no x.output + x.extension) }
// CSRF-07 load/read failures are closed with no logical state change.
assert StoreFailure { all x: State | x.active = Yes and failed[x] implies
  (x.result = StoreError and x.tokens = x.prior) }

check Issuance for 7 but exactly 7 State expect 0
check Reuse for 7 but exactly 7 State expect 0
check Isolation for 7 but exactly 7 State expect 0
check SafeAdmission for 7 but exactly 7 State expect 0
check UnsafeAdmission for 7 but exactly 7 State expect 0
check Precedence for 7 but exactly 7 State expect 0
check BodyLimit for 7 but exactly 7 State expect 0
check PayloadPreserved for 7 but exactly 7 State expect 0
check StoreFailure for 7 but exactly 7 State expect 0

// Same twelve witness meanings as Quint's *_test runs, all must be SAT.
pred event[x: State] { x.active = Yes and x.sid = A }
pred Issue { some x: State | event[x] and no x.prior[A] and x.result = Admit and x.extension = TA }
pred Reused { some x: State | event[x] and x.prior[A] = TA and x.tokens[A] = TA and x.result = Admit }
pred SafeLarge { some x: State | event[x] and x.safe = Yes and x.bodySize = Above and x.header = Wrong and x.result = Admit }
pred Header { some x: State | event[x] and x.safe = No and x.header = TA and x.mime = No and x.result = Admit and x.output = P1 }
pred Form { some x: State | event[x] and x.safe = No and no x.header and x.form = TA and x.mime = Yes and x.result = Admit }
pred HeaderWins { some x: State | event[x] and x.safe = No and x.header = Wrong and x.form = TA and x.mime = Yes and x.result = Forbidden }
pred Missing { some x: State | event[x] and x.safe = No and no x.header + x.form and x.result = Forbidden and x.tokens[A] = TA }
pred Mismatch { some x: State | event[x] and x.safe = No and no x.header and x.form = Wrong and x.mime = Yes and x.result = Forbidden }
pred Foreign { some x: State | x.active = Yes and x.sid = B and x.prior[A] = TA and x.header = TA and x.safe = No and x.result = Forbidden and x.tokens[B] = TB }
pred Boundary { some x: State | event[x] and x.safe = No and x.header = TA and x.bodySize = At and x.result = Admit }
pred Oversized { some x: State | event[x] and x.safe = No and no x.header + x.form and x.bodySize = Above and x.result = TooLarge }
pred FailedStore { some x: State | event[x] and x.safe = Yes and no x.prior[A] and x.readFail = Yes and x.result = StoreError and no x.tokens[A] }
run Issue for 7 but exactly 7 State expect 1
run Reused for 7 but exactly 7 State expect 1
run SafeLarge for 7 but exactly 7 State expect 1
run Header for 7 but exactly 7 State expect 1
run Form for 7 but exactly 7 State expect 1
run HeaderWins for 7 but exactly 7 State expect 1
run Missing for 7 but exactly 7 State expect 1
run Mismatch for 7 but exactly 7 State expect 1
run Foreign for 7 but exactly 7 State expect 1
run Boundary for 7 but exactly 7 State expect 1
run Oversized for 7 but exactly 7 State expect 1
run FailedStore for 7 but exactly 7 State expect 1
