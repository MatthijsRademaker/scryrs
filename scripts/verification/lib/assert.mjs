/**
 * Minimal assertion library for scryrs cross-harness verification fixtures.
 *
 * Collects pass/fail results and provides a `summary()` function that prints
 * a final report and exits non-zero on failures.
 */

let PASSED = 0;
let FAILED = 0;

/**
 * Record a passing assertion.
 */
export function pass(name) {
	console.log(`  \x1b[32mPASS\x1b[0m ${name}`);
	PASSED++;
}

/**
 * Record a failing assertion with an optional reason.
 */
export function fail(name, reason) {
	console.log(`  \x1b[31mFAIL\x1b[0m ${name}`);
	if (reason) console.log(`        \x1b[31m${reason}\x1b[0m`);
	FAILED++;
}

/**
 * Assert a condition is truthy. Records pass/fail.
 */
export function assert(condition, name) {
	if (condition) {
		pass(name);
	} else {
		fail(name, "assertion failed");
	}
}

/**
 * Deep-equal assertion with a named test.
 */
export function assertDeepEqual(actual, expected, name) {
	try {
		const actualJson = JSON.stringify(actual);
		const expectedJson = JSON.stringify(expected);
		if (actualJson === expectedJson) {
			pass(name);
			return;
		}
		fail(name, `expected ${expectedJson}, got ${actualJson}`);
	} catch {
		fail(name, "JSON serialization error during deep equal check");
	}
}

/**
 * Print final pass/fail summary and exit non-zero if any failures.
 */
export function summary() {
	console.log("");
	console.log("============================================");
	console.log(
		`Results: \x1b[32m${PASSED} passed\x1b[0m, \x1b[31m${FAILED} failed\x1b[0m, ${PASSED + FAILED} total`,
	);
	console.log("============================================");

	if (FAILED > 0) process.exit(1);
}

/**
 * Return the current pass/fail counts (for composite entrypoints that want to
 * aggregate results across fixtures).
 */
export function counts() {
	return { passed: PASSED, failed: FAILED };
}

/**
 * Reset pass/fail counters.
 */
export function reset() {
	PASSED = 0;
	FAILED = 0;
}
