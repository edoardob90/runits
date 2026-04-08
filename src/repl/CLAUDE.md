# repl/ module

Interactive REPL mode with Fish-style completion, inline hints, and syntax highlighting.

## Architecture

- **`mod.rs`** — Event loop (`run()`), input dispatch (`handle_input()`), help handlers, banner/info display, `parse_repl_line()` (pub, also used by `main.rs` for batch mode). All tests.
- **`helper.rs`** — `UnitsHelper` struct implementing rustyline's `Completer`, `Hinter`, `Highlighter`, `Validator`, `Helper` traits. Infrastructure plumbing only.

## Key patterns

- `UnitsHelper` is `pub(super)` — only constructed in `mod.rs`'s `run()`
- Dispatch order in `handle_input()`: delimiter conversion -> `?` help -> bare quantity echo
- Theme is imported from `crate::theme`, not from format
- Highlighter creates its own `Theme::new(true)` (always colored; NO_COLOR checked before)
- Dimension-aware completion: after a conversion delimiter, only compatible units are suggested

## FUTURE markers

- `FUTURE(theme-config)` in `print_info()` — theme name should come from config
