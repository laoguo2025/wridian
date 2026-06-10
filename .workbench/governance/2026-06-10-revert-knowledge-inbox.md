# Revert Knowledge Inbox

## Trigger

The derived `候选箱` treated unstructured Markdown as a candidate knowledge card. This conflicted with `zhishiku-skill`, where `01原始资料` intentionally stores unprocessed source material and `02拆解报告` stores analysis outputs plus A/B/C candidates.

## Decision

Remove the generic derived inbox from Wridian.

Knowledge-card promotion is not a UI toggle:

- `01原始资料`: raw material, allowed to be unedited.
- `02拆解报告`: analysis reports, notes, source splits, and A/B/C candidates.
- `03-07`: only S-level reusable knowledge cards produced or accepted through `zhishiku-skill` / `tilian-skill` quality gates.

## Change

- Removed `knowledgeInboxFiles` from workspace API and frontend types.
- Removed the `候选箱` rail button and empty state.
- Removed backend scanning rules based on missing frontmatter or wikilinks.
- Updated the project map with the skill-driven promotion rule.

## Verification

- Passed: `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace`.
- Passed: `npm run build -- --mode development`.

## Rollback

Do not restore the generic inbox. If a future UI is needed, expose `zhishiku-skill` health-check or `02拆解报告` A-level candidates explicitly, using the skill's own report format and gates.
