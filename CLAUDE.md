# Claude Code Instructions

Read and follow the shared project instructions first:

- `AGENTS.md` - always-on product, architecture, evidence, Git, and learning rules
- `docs/design-principles.md` - the maintainer's product and design decision lens
- the nearest scoped `AGENTS.md` for `core/`, `src-tauri/`, `src/`, or `docs/`

Then read the matching shared rule from `.agents/rules/README.md`:

- `.agents/rules/design.md` - design and evidence checklist
- `.agents/rules/rust.md` - Rust ownership and testing rules
- `.agents/rules/frontend.md` - frontend boundaries and visual verification
- `.agents/rules/settings.md` - Application Preference persistence rules
- `.agents/rules/documentation.md` - documentation placement and verification
- `.agents/rules/clean-room-sensors.md` - mandatory clean-room rules before touching `docs/specs/sensors/**` or native PawnIO CPU / Super I/O implementation

For design work, follow `.agents/skills/hardwarevisualizer-design-review/SKILL.md`.
After a durable correction, repeated failure, or surprising invariant, follow
`.agents/skills/capture-project-learning/SKILL.md`.
