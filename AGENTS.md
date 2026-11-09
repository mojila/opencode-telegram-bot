# AGENTS.md

Instructions for AI agents working on this project.

## About the project

**opencode-telegram-bot** is a Telegram bot that acts as a mobile client for OpenCode.
It lets a user run and monitor coding tasks on a local machine through Telegram.

The implementation is now a **Rust workspace**: `otb-core` (pure logic, parity
gates, formatters, managers) plus `otb-bot` (thin binary wiring: CLI, Telegram,
health, store). The TypeScript `src/` tree is frozen and serves as parity source only.

Functional requirements, features, and development status are in [PRODUCT.md](./PRODUCT.md).
PRODUCT.md is still the source of truth for scope and milestones.

## Technology stack

- **Language:** Rust, edition 2021
- **Workspace:** `Cargo.toml` with `resolver = "2"`, members `crates/otb-core` + `crates/otb-bot`
- **Configuration:** environment variables (`.env` via `dotenvy`)
- **Logging:** `tracing` with levels (`debug`, `info`, `warn`, `error`)

### Core dependencies

- `tokio` (full) - async runtime
- `teloxide-core` 0.11 - thin Telegram Bot API client only (no dispatcher yet)
- `reqwest` (features `json` + `stream`) - OpenCode V1/V2 HTTP + SSE client
- `rusqlite` (bundled) - session/project directory cache on an isolated path only
- `axum` 0.7 - loopback health endpoint
- `clap` (derive) - CLI (`start` / `status`)
- `tracing` + `tracing-subscriber` - structured logging
- `dotenvy` - `.env` loading
- `serde` / `serde_json` - config and payload (de)serialization
- `anyhow` / `thiserror` - error handling (`anyhow` at boundaries, `thiserror` for library types)

### Test dependencies

- `cargo test` (unit tests inline + `crates/otb-core/tests/`)
- No external test runner; OpenCode HTTP interaction is covered with mocks/stubs

### Code quality

- `cargo fmt`
- `cargo clippy -- -D warnings`
- Rust edition 2021, strict warnings deny

### Build cache hygiene

- Workspace uses a single gitignored workspace-root `target/` (no per-crate targets).
- Safe clean (default, no confirmation needed): `rm -rf target/debug/incremental target/tmp` (~2+ GB freed, rebuilds automatically).
- Destructive cleans need explicit user confirmation via the `question` tool: full `cargo clean` (~5.5 GB: `debug` ~3.3 GB + `release` ~0.5 GB) forces a full rebuild, and `~/.cargo/registry` prune (~1.0 GB) forces re-download.
- Never delete `Cargo.lock`, `target/debug/deps`, or `target/release` without confirmation.
- Do not clean while `cargo build` / `cargo test` is running (incremental regrows and `rm` races).
- After cleaning, verify with `cargo fmt --check`; hand build/test runs to qa-automation.

## Architecture

### Main components

1. **otb-core (pure logic)** - `*_gates.rs` parity modules (slices 1-172), plus:
   `formatters/`, `render/`, `keyboards.rs`, `streaming.rs`, `interaction_state.rs`,
   `event_subscription.rs`, `prompt_queue_events.rs`, `scheduled_tasks.rs`,
   `types_config.rs`, `i18n_full.rs` + `locale_*_gates.rs`, `managers/`
2. **otb-bot (thin binary)** - `main.rs` + `cli.rs` / `commands.rs` (23) /
   `menus.rs` (19) / `config.rs` / `guard.rs` / `health.rs` / `locale.rs` /
   `logging.rs` / `media.rs` / `opencode.rs` (read-only) / `settings_current.rs` /
   `store.rs` / `telegram.rs` (polling gated)
3. **OpenCode Client Layer** - `reqwest` V1/V2 clients, `event_subscription` SSE
   stream, `v2_mappers` V2-to-core translation, `server_target` / `server_health` /
   `process` parity
4. **State Managers** - `otb-core/src/managers/` (session/project/settings/question/
   permission/model/agent/variant/keyboard/pinned parity)
5. **Summary Pipeline** - event aggregation and Telegram-friendly formatting
   (`formatters/`, `render/`, `streaming.rs`, `rich_blocks.rs`, `rich_message.rs`,
   `markdown_normalizer.rs`)
6. **Process Manager** - local OpenCode server process start, stop, and status
   (`local_process_gates`, `server_command_gates`, `server_target_gates` parity)
7. **Runtime/CLI Layer** - `cli.rs`, `config.rs` bootstrap, `start`/`status` commands
8. **I18n Layer** - `i18n_full.rs` + `locale_*_gates.rs` (en + 11 locales parity)

### Data flow

```text
Telegram User
  -> Telegram Bot (teloxide-core, thin client)
  -> Managers + OpenCode reqwest client
  -> OpenCode Server

OpenCode Server
  -> SSE Events
  -> event_subscription / v2_mappers
  -> Summary Aggregator / Tool Managers
  -> Telegram Bot
  -> Telegram User
```

### State management

- Persistent selected-project/session context is read through a `settings.json`
  `current*` view (`settings_current.rs`, read-only).
- Session/project directory cache lives in `rusqlite` on an isolated path only
  (`store.rs`); not wired into live flows yet.
- Active runtime state is kept in dedicated in-memory managers.
- Session/project/model/agent context is synchronized through OpenCode API calls.
- The app is currently single-user by design.
- `OTB_ENABLE_POLLING=1` gates real Telegram polling; default stays scaffold-only.

## AI agent behavior rules

### Communication

- **Response language:** Reply in the same language the user uses in their questions.
- **Clarifications:** If plan confirmation is needed, use the `question` tool. Do not make major decisions (architecture changes, mass deletion, risky changes) without explicit confirmation.

### Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

### Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

### Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

### Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

### Git

- **Commits:** Never create commits automatically. Commit only when the user explicitly asks.

### Working on Windows

If your shell runs on Windows:

- Avoid fragile one-liners that can break in PowerShell.
- Use absolute paths when working with file tools (`read`, `write`, `edit`).

## Coding rules

### Language

- Code, identifiers, comments, and in-code documentation must be in English.
- User-facing Telegram messages should be localized through i18n (`i18n_full` + locales).

### Code style

- Rust edition 2021; run `cargo fmt` before finishing.
- Keep warnings clean: `cargo clippy -- -D warnings`.
- Keep functions small and focused.
- Use clear names and avoid unnecessary abbreviations.
- Prefer `async/await` with `tokio`; no blocking calls on the async runtime.

### Error handling

- Use `anyhow` at binary boundaries and `thiserror` for library error types in `otb-core`.
- Add context with `.context(...)` (session ID, operation type, etc.).
- Use `tracing` for diagnostics; send understandable error messages to users.
- Never expose stack traces or internals to users.
- No `unwrap()` / `expect()` in library code paths; handle `Result`/`Option` explicitly.

### Cross-platform

- The bot runs on Linux, macOS, and Windows; CI runs tests on Linux.
- Code must work on all three regardless of the OS you develop on: passing checks locally does not prove it works elsewhere.
- Code that touches paths, processes, shells, or the filesystem must work on all three: no hardcoded `\` or `/` separators (use `std::path`), no assumptions about line endings or path case.
- Windows-only logic runs behind `cfg(windows)` gating. A test for it either passes on every OS or is skipped outside Windows.

### Bot commands

The command registry lives in `crates/otb-bot/src/commands.rs` (23 commands) with
menus in `crates/otb-bot/src/menus.rs` (19 menus). Both are structure-only: no
Telegram API calls yet.

Important:

- When adding a command, update `commands.rs` only (and `menus.rs` for menus).
- The same source is used for Telegram `setMyCommands` and help/docs.
- Do not duplicate command lists elsewhere.

### Logging

The project uses `tracing`, initialized via `init_logging(log_level)` in `otb-bot`.

Log files (conventions carried over from TS):

- In source mode logs are stored `<project root>/logs` by default.
- Each source-mode bot run writes to a separate file named `bot-YYYY-MM-DD_HH-MM-SS_<pid>.log`.
- The `logs/` directory is gitignored, so search inside it directly: use `path: "logs"` with `pattern: "*.log"`.
- Installed mode writes under the installed app home `logs` directory and uses daily files named `bot-YYYY-MM-DD.log`.

Levels:

- **DEBUG** - detailed diagnostics (callbacks, keyboard build, SSE internals, polling flow)
- **INFO** - key lifecycle events (session/task start/finish, status changes)
- **WARN** - recoverable issues (timeouts, retries, unauthorized attempts)
- **ERROR** - critical failures requiring attention

Use:

```rust
tracing::debug!("[Component] Detailed operation {}", details);
tracing::info!("[Component] Important event occurred");
tracing::warn!("[Component] Recoverable problem: {}", err);
tracing::error!("[Component] Critical failure: {}", err);
```

Important:

- Do not use raw `println!` / `eprintln!` in library code; use `tracing`.
  (`println!` only in the `cli` status scaffold.)
- Put internal diagnostics under `debug`.
- Keep important operational events under `info`.
- Default level is `info`.

## Testing

### What to test

- Unit tests for parity gates, formatters, managers, render, runtime helpers
- Integration-style tests around OpenCode HTTP interaction using mocks/stubs
- Focus on parity gates + critical paths; avoid over-testing trivial code

### Test structure

- `cargo test` runs `otb-core/tests/` plus inline `#[cfg(test)]` modules
- Use `cargo test -p otb-core -p otb-bot` to cover both crates
- Use descriptive test names
- Follow Arrange-Act-Assert
- For bugs: reproduce as a failing test first, then make it pass

## OpenCode client quick reference

Use V2 OpenCode API as authoritative: https://opencode.ai/docs/api (all routes
under `/api/*`). Default `OPENCODE_SERVER_VERSION=v2`, fallback to V1 bare
routes only when the server does not answer V2 (`GET /api/info` -> `GET
/global/health`).

V1/V2 go through `reqwest` (no TS SDK as primary). SSE streams through
`otb-core::event_subscription`, V2 shapes are normalized by `v2_mappers`, and
server targeting/health/process follow `server_target_gates` / `session_health_gates` /
`local_process_gates` parity.

```rust
// Health + project/session calls (V2 shape authoritative; V1 fallback in comments).
let http = reqwest::Client::new();
let base = "http://127.0.0.1:49374"; // V2 service default; V1 fallback http://127.0.0.1:4096

// GET /api/info (V1: /global/health)
let health: serde_json::Value =
    http.get(format!("{base}/api/info")).send().await?.json().await?;

// GET /api/project, POST /api/session, POST /api/session/{id}/prompt,
// POST /api/session/{id}/interrupt (V1: /project, /session, /session/:id/message, /session/:id/abort)

// SSE: GET /api/event -> event_subscription stream (V1: /event)
let stream = otb_core::event_subscription::subscribe(&http, base).await?;
while let Some(event) = stream.next().await {
    let core_event = otb_core::v2_mappers::map_event(&event);
    // handle core event (aggregator / tool managers)
}
```

Full behavior: `src/opencode/*` TS parity source + `otb-core` `server_target_gates`,
`v2_client_gates`, `v2_event_translator_gates`, `event_transport_gates`.

## Workflow

1. Read [PRODUCT.md](./PRODUCT.md) to understand scope and status.
2. Inspect existing crates (`crates/otb-core`, `crates/otb-bot`) before adding or changing components.
3. Align major architecture changes (including new dependencies) and milestones with the user first.
4. Add or update tests for new functionality.
5. After code changes, run quality checks: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build`, and `cargo test`.
6. Update checkboxes in `PRODUCT.md` when relevant tasks are completed.
7. Keep code clean, consistent, and maintainable.

## Legacy TypeScript reference (frozen)

- `src/` holds 282 TS files (145 under `src/bot/`, 19 menus, 12 locales + index).
- Stack: `grammy`, `@grammyjs/menu`, `@opencode-ai/sdk` (V1) + `@opencode/client` (V2), Vitest + `vi.mock()`, ESLint + Prettier.
- Command list was centralized in `src/bot/commands/definitions.ts` (23 commands).
- OpenCode layer: `src/opencode/{client,events}.ts` + `src/opencode/v2/*` (client, events, mappers, goal-message); shared `server-target/health/process/local-start/auto-restart/ready-*`.
- Session cache used `better-sqlite3` via `src/app/services/session-cache-service.ts`.
- Frozen: do not extend. Parity source only for the Rust rewrite.
