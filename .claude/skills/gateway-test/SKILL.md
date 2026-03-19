---
name: gateway-test
description: Run gateway test suite, optionally filtering by test name pattern. Use when user says "run tests", "test the gateway", or specifies a test name.
---

# Gateway Test Runner

Run the gateway test suite and summarize results concisely.

## Behavior

1. Run: `cd gateway && cargo test $ARGUMENTS 2>&1`
   - If the user provides arguments (e.g., `fallback`, `scoring`), pass them as the test filter
   - If no arguments, run all tests
2. Parse the output and present:
   - Total pass/fail count per test binary
   - If all pass: one-line summary with total count
   - If failures: show only failing test names and their assertion error messages
3. Do NOT show compiler warnings unless they are errors
4. Do NOT show the full cargo output — summarize it
