# Documentation instructions (HardwareVisualizer)

These instructions are used by automated documentation workflows.

## File selection

- Prefer updating `README.md` / `README.ja.md` first.
- Only touch `docs/**` and `.github/**` when the change clearly belongs there (e.g., contributor docs, templates, or structured docs pages).
- Maintain AI-facing docs as first-class docs: `.github/instructions/**`

## Scope

Update documentation ONLY within this repository.

Primary docs targets:

- `README.md`
- `README.ja.md` (if present)
- `docs/**` (if present)
- AI instructions: `.github/instructions/**`
- Contributor docs such as `CONTRIBUTING.md` and `.github/**` templates

Do NOT edit external websites or other repositories.

## Writing rules

- Be factual and technical. No marketing.
- Prefer short paragraphs and bullet points.
- Use concrete steps and exact menu paths.
- Include code blocks with proper language tags (`bash`, `powershell`, `json`, `toml`, `yaml`, etc.).
- If behavior is uncertain, verify by reading the relevant PR diff or code before documenting.

## Change selection

Document user-facing changes only:

- Installation / update / distribution changes
- Settings UI changes
- Dashboard / visualization behavior changes
- Sensor support changes (CPU/GPU/RAM/etc.)
- Platform-specific caveats (Windows/macOS/Linux)
- Troubleshooting / known limitations

Skip internal refactors unless they affect users.

## Bilingual consistency

If `README.ja.md` exists:

- Keep headings and feature lists consistent between English and Japanese.
- If you can’t confidently translate a new section, add it to `README.md` and leave a TODO note in the PR description (do not add bad Japanese).

## PR etiquette for automated docs updates

- Keep changes small and focused.
- Use neutral PR titles like: `[docs] Update documentation (weekly)`
- In the PR description, list:
  - What was updated
  - Which merged PRs triggered the update
  - Any uncertainty / follow-up needed
