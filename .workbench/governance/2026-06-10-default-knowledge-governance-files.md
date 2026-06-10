# Default Knowledge Governance Files

## Trigger

After removing the generic knowledge inbox, the default knowledge-base initialization still differed from the active `zhishiku-skill` workflow. It placed `使用说明.md` inside `00知识库治理`, while the current product wording should use `治理说明.md`.

## Decision

Default knowledge-base seeding now creates:

- `00知识库治理/治理说明.md`
- `00知识库治理/调用记录台账.md`
- `08大神蒸馏/大神索引.md`
- `08大神蒸馏/_安装记录.md`

It does not create root-level `知识库使用说明.md`.

## Constraints

- Existing knowledge roots are not overwritten or backfilled by this initializer.
- Users can still rename, add, or remove category folders; skill health checks should follow the real directory state.

## Verification

- Passed: `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml runtime`.
- Passed: `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml workspace`.
- Passed: `npm run build -- --mode development`.

## Rollback

Restore the prior `DEFAULT_KNOWLEDGE_CATEGORIES` readme tuple and seed `00知识库治理/使用说明.md` only.
