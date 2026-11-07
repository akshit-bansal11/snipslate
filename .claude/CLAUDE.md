This project inherits every rule in F:/.claude/CLAUDE.md. Rules below add to or override it. Nothing here restates it.

- Notion page (source of truth for non-code facts): https://app.notion.com/p/410c6445b19f814f8eacc81c2faba48d
- Ref-ID trailer for every commit: `Ref-ID: yfu0idauhvj2nm9nkp0lcgkd7-5taorn0rz0`
- Type D (native cross-platform tray app: Rust + egui; Windows, macOS, Linux). No web deploy, no services.
- Process model: the tray process has no GPU context; snip, settings and message windows run as child processes (`--translate`, `--copy`, `--settings`, `--message`).
- Quality gate: `scripts/check.ps1` (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`) and `scripts/check.ps1 -Ci` (`fmt --check` instead of formatting). CI runs `-Ci` on Windows, macOS and Linux.
- Pin crate versions from `cargo search <crate> --limit 1`, never from memory.
- Nothing builds or runs locally on the owner's laptop. Push and read GitHub Actions.
- Release: push a `v*` tag; `release.yml` runs the gate and publishes `Snipslate-<version>-windows-x64.exe`, `Snipslate-<version>-macos-universal.zip`, `Snipslate-<version>-linux-x64.tar.gz` and `SHA256SUMS.txt`.
- The repo is private because this environment cannot create public repos; making it public is the owner's call.
