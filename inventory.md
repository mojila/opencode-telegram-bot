# T0 Baseline Inventory (freeze)

Date: 2026-11-09 (UTC)
Branch: main tracking my-fork/main
HEAD: a3f4813 — merge(rust-rewrite): slices 1-172 parity + i18n codegen + docs order
Tree status at freeze: clean (`git status --short --branch` → only `## main...my-fork/main`, `git diff --stat` empty)
Live process: PID 8971 — `node /opt/homebrew/bin/opencode-telegram start` (verified `ps -p 8971 -o pid,command`)

## TypeScript baseline (src)
- `find src -name "*.ts" | wc -l` → 282
- `find src/bot -name "*.ts" | wc -l` → 145
- `ls src/bot/menus/ | wc -l` → 19
  - agent-selection-menu.ts, command-catalog-menu.ts, context-control-menu.ts,
    file-browser-menu.ts, inline-menu.ts, mcp-catalog-menu.ts, message-history-menu.ts,
    model-selection-menu.ts, permission-menu.ts, project-selection-menu.ts,
    question-menu.ts, recent-selection-menu.ts, rename-menu.ts, scheduled-task-menu.ts,
    session-selection-menu.ts, settings-menu.ts, skills-catalog-menu.ts,
    variant-selection-menu.ts, worktree-selection-menu.ts
- `ls src/i18n/` → ar, de, en, es, fr, id, index.ts, it, ko, pt, ru, tr, zh (12 locales + index)
- `grep -c "command:" src/bot/commands/definitions.ts` → 25 (2 interface fields + 23 command entries:
  status, new, abort, detach, sessions, recent, messages, settings, projects, worktree,
  task, tasklist, rename, commands, skills, mcps, opencode_start, opencode_stop,
  reload[v2], goal[v2], open, ls, help)
- `grep -r "@opencode" package.json` → `@opencode-ai/sdk ^1.18.8`, `@opencode/client ^2.0.16`
- `ls src/` top-level → app, bot, cli, cli.ts, config.ts, i18n, index.ts, opencode, runtime, utils

## Rust rewrite (crates)
- `ls crates/` → otb-core (only crate)
- `ls crates/otb-core/src/*.rs | wc -l` → 153 (top-level only; plus subdirs: formatters, i18n, managers, render)

## OpenCode V1/V2 files (`find src/opencode -type f`)
- V1: src/opencode/client.ts, src/opencode/events.ts
- V2: src/opencode/v2/client.ts, src/opencode/v2/events.ts, src/opencode/v2/mappers.ts,
  src/opencode/v2/goal-message.ts
- Shared: auto-restart.ts, ready-lifecycle.ts, ready-refresh.ts, server-target.ts,
  server-health.ts, process.ts, local-start.ts

## DB files (better-sqlite3)
- package.json: `better-sqlite3 ^13.0.1`, `@types/better-sqlite3 ^7.6.13`
- usage: src/app/services/session-cache-service.ts (imports better-sqlite3; sqlite fallback warmup)
- referenced: src/runtime/node-version.ts (SIGSEGV guard comment for better-sqlite3)

## Daemon/service files
- `grep -ril "daemon" src` → src/runtime/service/manager.ts, src/runtime/service/types.ts,
  src/cli.ts, src/cli/args.ts
- No `*daemon*` filenames (`find src -iname "*daemon*"` → only matches via content above)

## T0 acceptance notes
- No edits under `src/` (only new file: inventory.md at repo root).
- PID 8971 left untouched and alive at freeze time.
