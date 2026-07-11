# Shared Agent Rules

This directory owns path-scoped rules shared by the repository's coding
agents. `scope` is a selection index for agents; it is not tool-specific
frontmatter or an automatic glob mechanism.

Read the matching rule before editing files in its scope. The root and scoped
`AGENTS.md` files point to the rules that are mandatory for their areas.

| Rule | Scope |
| --- | --- |
| [design.md](design.md) | Product, architecture, persistence, monitoring, and UX decisions |
| [rust.md](rust.md) | `core/**`, `src-tauri/**`, and Rust workspace manifests |
| [frontend.md](frontend.md) | Frontend code and language files under `src/` |
| [settings.md](settings.md) | Settings UI, settings services, and persisted settings ownership |
| [documentation.md](documentation.md) | Root documentation, `docs/**`, and GitHub Markdown |
| [clean-room-sensors.md](clean-room-sensors.md) | Sensor specs and PawnIO CPU / Super I/O implementation |

GitHub-specific configuration remains under `.github/`. Do not add a
tool-specific copy of these rules unless that tool is actively used and needs a
documented adapter.
