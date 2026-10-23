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

test('Quint runner invokes all ten bounded verifications even after failure', () => {
  const root = fs.mkdtempSync(path.join(process.env.JCODE_SCRATCH_DIR || os.tmpdir(), 'csrf-runner-test-'));
  try {
    for (const dir of ['scripts', 'crates/gluon/tests/specs/node_modules/.bin', 'target/formal-tools/quint/apalache-dist-0.51.1/apalache/bin']) {
      fs.mkdirSync(path.join(root, dir), { recursive: true });
    }
    fs.copyFileSync('scripts/check-csrf-quint.sh', path.join(root, 'scripts/check-csrf-quint.sh'));
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
if (args.includes('--invariant=reuse')) process.exit(7);
`, { mode: 0o755 });
    const result = spawnSync('bash', ['scripts/check-csrf-quint.sh'], { cwd: root, encoding: 'utf8' });
    assert.equal(result.status, 1, result.stderr);
    const invocations = fs.readFileSync(calls, 'utf8').trim().split('\n').map(JSON.parse);
    assert.deepEqual(invocations.slice(0, 3).map(a => a[0]), ['typecheck', 'test', 'run']);
    for (const command of invocations.slice(1, 3)) assert.ok(command.includes('--seed=0x43535246'));
    const verifies = invocations.filter(a => a[0] === 'verify');
    const properties = ['issuance', 'reuse', 'isolation', 'safe_admission', 'unsafe_admission', 'precedence', 'body_limit', 'payload_preserved', 'store_failure', 'contract'];
    assert.deepEqual(verifies.map(a => a.find(v => v.startsWith('--invariant='))), properties.map(p => `--invariant=${p}`));
    for (const args of verifies) {
      assert.ok(args.includes('--max-steps=6'));
      assert.ok(args.includes('--apalache-version=0.51.1'));
    }
    const output = path.join(root, 'target/csrf-spec/quint');
    const run = path.join(output, fs.readdirSync(output)[0]);
    for (const property of properties) assert.match(fs.readFileSync(path.join(run, `verification-${property}.log`), 'utf8'), /mock verify/);
    const status = fs.readFileSync(path.join(run, 'verification-status.log'), 'utf8');
    assert.match(status, /reuse FAIL/);
    assert.match(status, /contract PASS/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
