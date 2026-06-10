# Knowledge Graph, Skill Context, Context Compiler

## Scope

- Re-applied the non-Inbox part of the Tolaria boundary layer.
- Added frontmatter relation edges and knowledge node typing to the knowledge graph.
- Connected `/知识库运维` to a tool pill carrying the current knowledge root, zhishiku-skill discovery status, and a minimal local protocol.
- Reworked cocreation prompt assembly into fixed context slots with character budgets.

## Non-goals

- Did not restore the derived knowledge inbox.
- Did not add World Info, character cards, setting cards, foreshadowing cards, or trigger insertion rules.
- Did not execute external skill scripts or modify the user knowledge base.

## Validation

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml knowledge_graph --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml cocreation --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml creative_skill_sources --lib`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 check --manifest-path src-tauri\Cargo.toml`

## Rollback

- Revert the changes in `knowledge_graph.rs`, `KnowledgeGraphDrawer.tsx`, `creative_skills.rs`, `creativeSkills.ts`, `CopilotPromptEditor.tsx`, `App.tsx`, `CreativeSkillsDrawer.tsx`, `appTypes.ts`, and `cocreation.rs`.
- No user files or external systems are changed by this slice.
