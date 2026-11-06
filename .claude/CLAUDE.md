This project inherits every rule in F:/.claude/CLAUDE.md. Rules below add to or override it. Nothing here restates it.

- Notion page (source of truth for non-code facts): https://app.notion.com/p/410c6445b19f814f8eacc81c2faba48d
- Ref-ID trailer for every commit: `Ref-ID: yfu0idauhvj2nm9nkp0lcgkd7-5taorn0rz0`
- Type D (native Windows tray app). No web deploy, no services.
- Quality gate: `scripts/check.ps1` (writes) and `scripts/check.ps1 -Ci` (CI). There is no `package.json`; this is a .NET project.
- Nothing builds or runs locally on the owner's laptop. Push and read GitHub Actions.
- Release: push a `v*` tag; `release.yml` runs the gate and publishes `Snipslate-<version>.exe` plus `SHA256SUMS.txt`.
- The repo is private because this environment cannot create public repos; making it public is the owner's call.
