---
name: add_test
description: |
  Finds untested code in either the happy path or an error case and writes a
  test for it using existing codebase conventions. Use when the user wants to
  improve test coverage, fill a gap, or add a regression test.
---

# add_test

Find something untested — a happy-path flow, an error case, or an edge
condition — and write a focused test for it that follows the conventions
already established in the codebase.

## When to Use

- The user asks to "add a test", "cover this", "write a test for X"
- A recent change lacks a test
- The user points at a file, function, or handler and wants a test
- Coverage work: the user wants to find and fill test gaps

## How It Works

1. **Survey the codebase** — read the target file, its module, and the
   surrounding test modules to understand the conventions in use
   (unit vs integration, test helpers available, `#[sqlx::test]` vs
   `#[tokio::test]`, assertion style, naming conventions)

2. **Identify the gap** — choose a specific function, handler, error variant,
   branch, or conversion that is not exercised by any existing test. Prefer
   gaps that:
   - Cover a real risk (panic paths, error handling, edge cases)
   - Follow the natural boundary of the target function's contract
   - Are small enough to be one clear test

3. **Write the test** — place it in the existing `#[cfg(test)] mod tests`
   block at the bottom of the same file, using the same helpers, imports,
   and patterns the other tests use

## Codebase Conventions

This project uses Rust with axum, sqlx (SQLite), tokio, and minijinja.
Tests live inline in `#[cfg(test)] mod tests { ... }` at the bottom of each
source file.

### Test types

| Type | Macro | Helpers |
|------|-------|---------|
| Unit test (pure logic, no DB) | `#[test]` | Direct function calls |
| DB integration test | `#[sqlx::test]` | `SqlitePool` parameter, raw SQL |
| HTTP integration test | `#[sqlx::test]` | `crate::test::start_app(pool).await`, reqwest |

### Common imports inside `mod tests`

- `use super::*;` for the module under test
- `use crate::test::*;` for `start_app`, `WEB_USERNAME`, `WEB_PASSWORD`,
  `BEARER_TOKEN`, `SENTRY_DSN`, `HTTP_PORT`
- `use sqlx::SqlitePool;` for DB tests
- `use reqwest::header;` for auth headers on HTTP tests

### Naming

- `snake_case` describing the scenario: `action_context_outcome`
- Example: `create_user_and_verify_exists`, `get_missing_user_returns_not_found_with_error_body`

### Patterns to follow

- **Arrange, Act, Assert** — set up state, exercise the code, verify results
- **One scenario per test** — test one branch/outcome clearly
- **Assert on full response shape** — check status, headers, and body content
- **Use `expect("...")` on fallible calls** — descriptive messages on .unwrap()
- **Clean up via `SqlitePool`** — `#[sqlx::test]` gives a fresh database per test

## Examples

### Unit test (pure function)

```rust
#[test]
fn valid_input_returns_expected_result() {
    let result = function_under_test("valid input");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().field, expected_value);
}

#[test]
fn invalid_input_returns_error_variant() {
    let result = function_under_test("bad");
    assert!(matches!(result, Err(AppError::BadRequest(_))));
}
```

### Integration test (HTTP + DB)

```rust
#[sqlx::test]
async fn endpoint_does_the_right_thing(pool: SqlitePool) {
    let addr = start_app(pool).await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{addr}/path"))
        .send()
        .await
        .expect("request should complete");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response
        .json()
        .await
        .expect("response should be valid JSON");

    assert_eq!(body["key"], "expected_value");
}
```

## Constraints

- Do **not** introduce `dbg!`, `TODO`, `FIXME`, `DEBUG:`, or `FIXTURE:` strings
- Use `.expect("description")` instead of bare `.unwrap()`
- Keep the test inside the existing `#[cfg(test)] mod tests` block
- Follow the file's existing import style; add only what is needed
