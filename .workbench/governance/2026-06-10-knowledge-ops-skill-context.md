# Knowledge Ops Skill Context

## Scope

Connect the `知识库运维` skill entry to a real `zhishiku-skill`-aligned context without executing external scripts.

## Change

- Added a read-only backend command that reports whether local `zhishiku-skill/SKILL.md` is present.
- Skill management shows the knowledge-ops source as either the local skill path or an embedded minimal protocol.
- Selecting `/知识库运维` creates a `TOOL` pill instead of plain prompt text.
- The pill carries the current knowledge root, skill source state, and the minimum governance rules:
  - no generic candidate inbox,
  - `01原始资料` may remain raw,
  - `02拆解报告` holds reports and A/B/C candidates,
  - only S-level cards enter `03-07`,
  - cleanup defaults to archive suggestions under `09文件归档`.

## Non-goals

- Do not run `zhishiku-skill` scripts from the desktop app.
- Do not modify knowledge-base files automatically.
- Do not implement full health-check output parsing yet.

## Verification

- Passed: `npm run build -- --mode development`.
- Passed: `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml creative_skill_sources`.

## Rollback

Remove `creative_skills.rs`, unregister its command, and let slash commands return plain prompt text again.
