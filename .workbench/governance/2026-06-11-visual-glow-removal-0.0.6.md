# Visual Glow Removal 0.0.6

## Scope

- Remove the large radial glow from the knowledge graph canvas background.
- Remove the text glow from the empty editor Wridian brand mark.
- Remove outer glow shadows from the memory tree root, trunk, sense, and branch labels while keeping borders and subtle inset highlights.
- Bump release version from 0.0.5 to 0.0.6 for package, Tauri, and Rust manifests.

## Verification

- `npm run build`
- `powershell -ExecutionPolicy Bypass -File scripts\cargo-msvc.ps1 test --manifest-path src-tauri\Cargo.toml --lib`
- `git diff --check`
- Version sources no longer contain `0.0.5`: package manifest, package lock, Tauri config, Cargo manifest, and Cargo lock all resolve to `0.0.6`.
- `npm run tauri -- build` produced `Wridian_0.0.6_x64-setup.exe`.
- Release artifacts copied to `release/Wridian-0.0.6-test.exe` and `release/Wridian-0.0.6-x64-setup.exe`; only the formal installer is tracked.

## Rollback

Revert `src/App.css` and version manifest changes, then rebuild the previous release artifacts.
