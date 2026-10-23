const assert = require('node:assert/strict');
const fs = require('node:fs');

function checkReceipt(model, receipt, log = console.log) {
  const intended = [...model.matchAll(/^(check|run) (\w+) for (.+) expect ([01])$/gm)];
  assert.equal(intended.length, 21, 'nine assertions and twelve satisfiable witnesses required');
  assert.equal(intended.filter(([, type]) => type === 'check').length, 9);
  assert.equal(new Set(intended.map(([, , name]) => name)).size, 21, 'unique commands required');
  assert.ok(receipt && typeof receipt.commands === 'object' && !Array.isArray(receipt.commands), 'commands required');
  assert.equal(Object.keys(receipt.commands).length, intended.length, 'all commands must execute');
  assert.ok(Number.isSafeInteger(receipt.timestamp) && receipt.timestamp > 0, 'execution timestamp required');
  assert.equal(receipt.solver, 'sat4j', 'expected bundled solver');
  assert.equal(receipt.repeat, 1, 'one solution per command required');
  for (const [, type, name, scope, expectation] of intended) {
    assert.ok(Object.hasOwn(receipt.commands, name), `missing command ${name}`);
    const command = receipt.commands[name];
    assert.ok(command && typeof command === 'object', `${name}: command metadata required`);
    assert.equal(command.name, name, `${name}: name mismatch`);
    assert.equal(command.type, type, `${name}: type mismatch`);
    assert.equal(command.source, `${type} ${name} for ${scope}`, `${name}: source/scope mismatch`);
    // Alloy 6.2.0 omits zero/default fields, including expects=0 and UNSAT
    // solution. Its CLI writes a receipt after each command, so exact command
    // coverage rejects partial runs. There is no per-command completed/UNSAT
    // flag: an absent solution alone cannot prove execution independently of
    // the CLI's successful exit (which the shell runner also requires).
    assert.equal(command.expects ?? 0, Number(expectation), `${name}: expectation mismatch`);
    const solutions = command.solution;
    if (solutions !== undefined) {
      assert.ok(Array.isArray(solutions) && solutions.length === 1, `${name}: malformed solution list`);
      for (const solution of solutions) {
        assert.ok(solution && Array.isArray(solution.instances) && solution.instances.length > 0,
          `${name}: incomplete SAT solution`);
        for (const instance of solution.instances) {
          assert.ok(instance && typeof instance.values === 'object' && instance.values !== null,
            `${name}: instance values required`);
        }
      }
    }
    const sat = solutions !== undefined;
    assert.equal(sat, expectation === '1', `${name}: unexpected ${sat ? 'SAT' : 'UNSAT'}`);
    log(`${type} ${name}: ${sat ? 'SAT witness' : 'UNSAT, no counterexample'} (expect ${expectation})`);
  }
}

if (require.main === module) {
  checkReceipt(fs.readFileSync('crates/gluon/tests/specs/csrf.als', 'utf8'),
    JSON.parse(fs.readFileSync(process.argv[2], 'utf8')));
}
module.exports = { checkReceipt };
