# Knowledge Inbox Derived State

Status: Reverted by `.workbench/governance/2026-06-10-revert-knowledge-inbox.md`.

Reason: `zhishiku-skill` defines `01原始资料` as intentionally unprocessed material and `02拆解报告` as the place for analysis outputs and A/B/C candidates. A generic derived inbox based on missing frontmatter or wikilinks conflicts with that workflow.

## Scope

- Add a derived knowledge inbox to surface Markdown files that likely need organizing.
- Keep the inbox as UI state only; do not create a real folder, move files, or change knowledge root ownership.
- Reuse the existing file tree open/save path so candidate files remain ordinary knowledge files.

## Change

- `workspace_info` now returns `knowledgeInboxFiles`.
- Candidate rules are conservative:
  - Markdown under a top-level folder outside the default `00`-`09` knowledge categories is listed.
  - Markdown inside a default category is listed only when it has no YAML frontmatter block and no `[[wikilink]]`.
- The knowledge library rail shows a `候选箱` toggle with the current count and an empty state.

## Verification

- Passed: `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace`.
- Passed: `npm run build -- --mode development`.

## Rollback

- Remove `knowledgeInboxFiles` from the workspace response and frontend type.
- Remove the `候选箱` rail toggle and related CSS.
