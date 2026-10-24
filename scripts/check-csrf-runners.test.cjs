const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { test } = require('node:test');
const { checkReceipt } = require('./check-csrf-alloy-results.cjs');
const model = fs.readFileSync('crates/gluon/tests/specs/csrf.als', 'utf8');

// Minimal fixture follows the installed Alloy 6.2.0 DTO schema, including its
// omitted zero expectations and absent UNSAT solution fields.
function receipt() {
  const commands = {};
  for (const [, type, name, scope, expects] of model.matchAll(/^(check|run) (\w+) for (.+) expect ([01])$/gm)) {
    commands[name] = { type, name, source: `${type} ${name} for ${scope}` };
    if (expects === '1') Object.assign(commands[name], {
      expects: 1, solution: [{ instances: [{ values: { 'State$0': {} } }] }],
    });
  }
  return { commands, timestamp: 1792745704997, repeat: 1, solver: 'sat4j' };
}
const validate = r => checkReceipt(model, r, () => {});
test('complete receipt with omitted UNSAT defaults passes', () => validate(receipt()));
for (const [name, mutate, error] of [
  ['omitted check', r => delete r.commands.Issuance, /all commands/],
  ['omitted witness', r => delete r.commands.Issue, /all commands/],
  ['replacement command', r => { r.commands.Other = r.commands.Issuance; delete r.commands.Issuance; }, /missing command/],
  ['unexpected SAT', r => { r.commands.Issuance.solution = r.commands.Issue.solution; }, /unexpected SAT/],
  ['unexpected UNSAT', r => delete r.commands.Issue.solution, /unexpected UNSAT/],
  ['incorrect check expectation', r => { r.commands.Issuance.expects = 1; }, /expectation mismatch/],
  ['missing witness expectation', r => delete r.commands.Issue.expects, /expectation mismatch/],
  ['unexecuted witness metadata', r => { r.commands.Issue = { ...r.commands.Issue, solution: undefined }; }, /unexpected UNSAT/],
  ['empty solution', r => { r.commands.Issue.solution = []; }, /malformed solution/],
  ['null solution', r => { r.commands.Issuance.solution = null; }, /malformed solution/],
  ['missing instance', r => { r.commands.Issue.solution = [{}]; }, /incomplete SAT/],
  ['empty instances', r => { r.commands.Issue.solution = [{ instances: [] }]; }, /incomplete SAT/],
  ['missing execution metadata', r => delete r.timestamp, /timestamp/],
  ['incorrect scope', r => { r.commands.Issuance.source = 'check Issuance for 3'; }, /source\/scope/],
  ['incorrect command type', r => { r.commands.Issuance.type = 'run'; }, /type mismatch/],
]) {
  test(`rejects ${name}`, () => {
    const r = receipt();
    mutate(r);
    assert.throws(() => validate(r), error);
  });
}

test('Quint runner checks both models and continues all bounded verifications after failure', () => {
  const root = fs.mkdtempSync(path.join(process.env.JCODE_SCRATCH_DIR || os.tmpdir(), 'csrf-runner-test-'));
  try {
    for (const dir of ['scripts', 'crates/gluon/tests/specs', 'crates/gluon/tests/specs/node_modules/.bin', 'target/formal-tools/quint/apalache-dist-0.51.1/apalache/bin']) {
      fs.mkdirSync(path.join(root, dir), { recursive: true });
    }
    fs.copyFileSync('scripts/check-csrf-quint.sh', path.join(root, 'scripts/check-csrf-quint.sh'));
    for (const name of ['csrf.qnt', 'csrf_concurrent.qnt']) {
      fs.copyFileSync(`crates/gluon/tests/specs/${name}`, path.join(root, 'crates/gluon/tests/specs', name));
    }
    fs.writeFileSync(path.join(root, 'target/formal-tools/apalache.tgz'), 'fixture');
    const hash = require('node:crypto').createHash('sha256').update('fixture').digest('hex');
    fs.writeFileSync(path.join(root, 'scripts/csrf-tools.env'), `QUINT_VERSION=0.29.1\nAPALACHE_VERSION=0.51.1\nAPALACHE_SHA256=${hash}\n`);
    fs.writeFileSync(path.join(root, 'target/formal-tools/quint/apalache-dist-0.51.1/apalache/bin/apalache-mc'), '', { mode: 0o755 });
    const calls = path.join(root, 'calls.jsonl');
    fs.writeFileSync(path.join(root, 'crates/gluon/tests/specs/node_modules/.bin/quint'), `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
if (args[0] === '--version') { console.log('0.29.1'); process.exit(0); }
fs.appendFileSync(${JSON.stringify(calls)}, JSON.stringify(args) + '\\n');
console.log('mock ' + args.join(' '));
if (process.env.MOCK_FAILURE && args.includes('--invariant=' + process.env.MOCK_FAILURE)) process.exit(7);
`, { mode: 0o755 });
    const sequential = 'csrf.qnt';
    const concurrent = 'csrf_concurrent.qnt';
    const properties = [
      ['issuance', 'reuse', 'isolation', 'safe_admission', 'unsafe_admission', 'precedence', 'body_limit', 'payload_preserved', 'store_failure', 'contract'],
      ['token_reuse', 'admission_from_snapshot', 'foreign_token_rejected', 'save_uses_snapshot', 'save_preserves_other_session', 'contract'],
    ];
    const assertCoverage = invocations => {
      const typechecks = invocations.filter(a => a[0] === 'typecheck');
      assert.deepEqual(typechecks.map(a => path.basename(a[1])), [sequential, concurrent]);
      const modelCommands = invocations.filter(a => ['test', 'run'].includes(a[0]));
      assert.deepEqual(modelCommands.map(a => path.basename(a[1])), [sequential, sequential, concurrent, concurrent]);
      for (const command of modelCommands) assert.ok(command.includes('--seed=0x43535246'));
      for (const command of modelCommands.filter(a => a[0] === 'run')) {
        assert.ok(command.includes('--max-steps=6'));
      }
      const verifies = invocations.filter(a => a[0] === 'verify');
      assert.deepEqual(
        [verifies.slice(0, 10), verifies.slice(10)].map(group =>
          group.map(a => a.find(v => v.startsWith('--invariant=')))),
        properties.map(group => group.map(p => `--invariant=${p}`)),
      );
      for (const args of verifies) {
        assert.ok(args.includes('--max-steps=6'));
        assert.ok(args.includes('--apalache-version=0.51.1'));
      }
      for (const args of verifies.slice(0, 10)) assert.ok(args.some(a => path.basename(a) === sequential));
      for (const args of verifies.slice(10)) assert.ok(args.some(a => path.basename(a) === concurrent));
    };
    const runGate = failure => {
      fs.writeFileSync(calls, '');
      const result = spawnSync('bash', ['scripts/check-csrf-quint.sh'], {
        cwd: root, encoding: 'utf8', env: { ...process.env, MOCK_FAILURE: failure },
      });
      assert.equal(result.status, 1, result.stderr);
      const artifactDir = result.stdout.match(/Quint artifacts: (.+)/);
      assert.ok(artifactDir, result.stdout);
      const invocations = fs.readFileSync(calls, 'utf8').trim().split('\n').map(JSON.parse);
      assertCoverage(invocations);
      return {
        invocations,
        run: artifactDir[1],
        status: fs.readFileSync(path.join(artifactDir[1], 'verification-status.log'), 'utf8'),
      };
    };

    // A failed sequential property must not suppress the concurrent model.
    const priorFailure = runGate('reuse');
    assert.match(priorFailure.status, /reuse FAIL/);
    assert.match(priorFailure.status, /contract PASS/);
    assert.match(priorFailure.status, /concurrent contract PASS/);
    for (const property of properties[0]) {
      assert.match(fs.readFileSync(path.join(priorFailure.run, `verification-${property}.log`), 'utf8'), /mock verify/);
    }
    for (const property of properties[1]) {
      assert.match(fs.readFileSync(path.join(priorFailure.run, `concurrent-verification-${property}.log`), 'utf8'), /mock verify/);
    }

    // A failure in a concurrent invariant itself reaches the consumer-visible gate.
    const concurrentFailure = runGate('save_uses_snapshot');
    assert.match(concurrentFailure.status, /concurrent save_uses_snapshot FAIL/);
    assert.match(concurrentFailure.status, /concurrent contract PASS/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
