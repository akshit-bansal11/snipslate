This project inherits every rule in F:/.claude/CLAUDE.md. Rules below add to or override it. Nothing here restates it.

- Top-level project since 2026-11-08 (promoted out of `F:/projects/mini-projects`). The mini-projects rules no longer apply; its reference id `yfu0idauhvj2nm9nkp0lcgkd7-5taorn0rz0` is retired. Older commits carry it as a `Ref-ID:` trailer; new commits do not.
- Notion page (source of truth for non-code facts), now in the Projects database: https://app.notion.com/p/410c6445b19f814f8eacc81c2faba48d
- Two parts in one repo:
  - The app (repo root): native cross-platform tray app, Rust + egui (Windows, macOS, Linux). The tray process has no GPU context; snip, settings and message windows run as child processes (`--translate`, `--copy`, `--settings`, `--message`).
  - The website (`site/`): Next.js landing page and docs at https://snipslate.akshitbansal.dev, Vercel project `snipslate` (team `akshit-bansal11s-projects`), root directory `site`, deployed by Vercel's Git integration on push to `main`.
- App quality gate: `scripts/check.ps1` (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`) and `scripts/check.ps1 -Ci` (`fmt --check`). `ci.yml` runs `-Ci` on Windows, macOS and Linux and skips pushes that only touch the site.
- Site quality gate: `npm run check` (writes) and `npm run check:ci` (non-mutating: `biome ci` + `tsc --noEmit`), run by `site.yml` together with `next build`.
- Pin crate versions from `cargo search <crate> --limit 1` and npm versions from the registry (`https://registry.npmjs.org/<pkg>/latest`), never from memory. TypeScript stays on 6.x: 7.x is the Go port without the compiler API Next's type check uses.
- Nothing builds or runs locally on the owner's laptop, including `npm`, `node` and `vercel`. Push and read GitHub Actions and Vercel builds. `screenshots.yml` (manual) renders the real windows on CI; the site's product images come from it.
- Release: push a `v*` tag; `release.yml` runs the gate and publishes `Snipslate-<version>-windows-x64.exe`, `Snipslate-<version>-macos-universal.zip`, `Snipslate-<version>-linux-x64.tar.gz` and `SHA256SUMS.txt`.
- The repo is private because this environment cannot create public repos; making it public is the owner's call. Until it is public, the site's download links (GitHub Releases) do not work for visitors.
