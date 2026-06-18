---
name: tdd
description: Drives development with tests. Use when implementing any logic, fixing any bug, or changing any behavior. Use when you need to prove that code works, when a bug report arrives, or when you're about to modify existing functionality.
---

# Test-Driven Development

## When to use

Always. Every implementation, every bug fix, every behavior change starts with a failing test. If you're writing production code without a test, stop and write the test first.

## Philosophy

**Core principle**: Tests should verify behavior through public interfaces, not implementation details. Code can change entirely; tests shouldn't.

**Good tests** are integration-style: they exercise real code paths through public APIs. They describe _what_ the system does, not _how_ it does it. A good test reads like a specification — "user can checkout with valid cart" tells you exactly what capability exists. These tests survive refactors because they don't care about internal structure.

**Bad tests** are coupled to implementation. They mock internal collaborators, test private methods, or verify through external means (like querying a database directly instead of using the interface). The warning sign: your test breaks when you refactor, but behavior hasn't changed. If you rename an internal function and tests fail, those tests were testing implementation, not behavior.

See [tests.md](tests.md) for examples and [mocking.md](mocking.md) for mocking guidelines.

## Anti-Pattern: Horizontal Slices

**DO NOT write all tests first, then all implementation.** This is "horizontal slicing" — treating RED as "write all tests" and GREEN as "write all code."

This produces **crap tests**:

- Tests written in bulk test _imagined_ behavior, not _actual_ behavior
- You end up testing the _shape_ of things (data structures, function signatures) rather than user-facing behavior
- Tests become insensitive to real changes — they pass when behavior breaks, fail when behavior is fine
- You outrun your headlights, committing to test structure before understanding the implementation

**Correct approach**: Vertical slices via tracer bullets. One test → one implementation → repeat. Each test responds to what you learned from the previous cycle. Because you just wrote the code, you know exactly what behavior matters and how to verify it.

```
WRONG (horizontal):
  RED:   test1, test2, test3, test4, test5
  GREEN: impl1, impl2, impl3, impl4, impl5

RIGHT (vertical):
  RED→GREEN: test1→impl1
  RED→GREEN: test2→impl2
  RED→GREEN: test3→impl3
  ...
```

## Workflow

### 1. Analyze

Study the task description, existing code, and the tests already in play. Identify:

- What public behavior must change or be added
- Whether existing public interfaces can be used as-is, need extension, or need new ones
- Which existing tests might break as collateral — so you know what to expect
- Opportunities for [deep modules](deep-modules.md) (small interface, deep implementation)
- [Interface design](interface-design.md) choices that affect testability

**Determine the test surface yourself.** Decide what the public interface should look like. Decide which behaviors are the most critical paths to cover. Limit the scope to the behaviors directly affected by the change; skip speculative edge-case coverage until the core path passes.

### 2. Tracer Bullet

Write ONE test that confirms ONE thing about the system:

```
RED:   Write test for the core behavior → test fails
GREEN: Write minimal code to pass → test passes
```

This is your tracer bullet — proves the path works end-to-end. Run just this test repeatedly during the loop:

```bash
scripts/test-go
```

### 3. Incremental Loop

For each remaining behavior:

```
RED:   Write next test → fails
GREEN: Minimal code to pass → passes
```

Rules:

- One test at a time
- Only enough code to pass current test
- Don't anticipate future tests
- Keep tests focused on observable behavior

### 4. Refactor

After all tests pass, look for [refactor candidates](refactoring.md):

- Extract duplication
- Deepen modules (move complexity behind simple interfaces)
- Apply SOLID principles where natural
- Consider what new code reveals about existing code
- Run tests after each refactor step

**Never refactor while RED.** Get to GREEN first.

### 5. Verify

Run the full test suite to catch regressions:

```bash
scripts/test-go
```

Fix any breaks. If other tests break, your change may need a broader interface update — loop back to Analyze.

### 6. Mutate

Prove the test constrains real behavior. Deliberately break the implementation (comment out key logic, flip a condition, remove a guard) and run the test. The test must fail. Then restore the implementation and confirm it passes again.

If the test stays green when you break the behavior, the test does not verify anything. Redo it. This step validates that your test actually catches bugs — it is mandatory for every behavioral change.

Do not mutate in bulk after all implementation is done. Mutate per vertical slice: after a test passes, break the behavior it tests, confirm RED, restore, move on.

## Checklist Per Cycle

```
[ ] Test describes behavior, not implementation
[ ] Test uses public interface only
[ ] Test would survive internal refactor
[ ] Code is minimal for this test
[ ] No speculative features added
[ ] Full suite passes (not just new tests)
[ ] Mutation check: broke behavior, test failed, restored, test passed
```
