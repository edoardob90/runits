# repl/ module

Interactive REPL mode with Fish-style completion, inline hints, and syntax highlighting.

## Architecture

- **`mod.rs`** — Event loop (`run()`), input dispatch (`handle_input()`), help handlers, banner/info display, `parse_repl_line()` (pub, also used by `main.rs` for batch mode). All tests.
- **`helper.rs`** — `UnitsHelper` struct implementing rustyline's `Completer`, `Hinter`, `Highlighter`, `Validator`, `Helper` traits. Infrastructure plumbing only.

## Key patterns

- `UnitsHelper` is `pub(super)` — only constructed in `mod.rs`'s `run()`
- Dispatch order in `handle_input()`: delimiter conversion → `?` help → bare expression eval (returns `HandleOutcome` enum: `Conversion`/`Quantity`/`None`)
- `last_quantity` tracks the most recent successful eval (for `_` previous-result variable), separate from `last_conversion`
- Theme is imported from `crate::theme`, not from format
- Highlighter creates its own `Theme::new(true)` (always colored; NO_COLOR checked before)
- Dimension-aware completion: after a conversion delimiter, only compatible units are suggested

## TDD patterns

Follow red/green/refactor — write the test before adding dispatch logic or new commands.

- **New REPL commands** — test `parse_repl_line()` (it's `pub`) directly in a unit test: feed the input string, assert the returned `HandleOutcome` variant and its contents. Confirm the test fails (wrong variant or panic) before adding the dispatch arm to `handle_input()`.
- **New completion behavior** — test `UnitsHelper`'s `Completer` impl with known prefixes (e.g., after a `→` delimiter); assert the candidate list contains expected unit names before wiring the dimension filter.
- **New help output** — unit test the relevant `?`/`info` handler's string output; prefer testing individual handler functions in isolation rather than the full REPL loop.
- **Integration path** — REPL commands that produce observable stdout can be tested via `assert_cmd` in `tests/cli_tests.rs` using `--batch` mode (pipe input lines); write the integration test first, then implement.

Concretely: run `cargo test -- repl::tests::<test_name>` after writing the test to confirm red, then implement.

## FUTURE markers

- `FUTURE(theme-config)` in `print_info()` — theme name should come from config
