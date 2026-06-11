<!--
Template for clean-room sensor implementation PRs (#1635).
Rules: .github/instructions/clean-room-sensors.instructions.md
Process: docs/specs/sensors/README.md
-->

## Summary

<!-- What does this PR do and why? -->

## Related Issues

<!-- e.g., Relates to #1635 plus the phase child issue -->

## Type of Change

- [ ] Bug fix (`fix/` branch)
- [x] New feature (`feat/` branch)
- [ ] Refactoring (`refactor/` branch)
- [ ] Documentation (`docs/` branch)
- [ ] Dependencies update
- [ ] Other (`chore/` branch)

## Clean-room provenance

<!-- Pin EVERY spec document consulted. One line per document. -->

```text
Implemented from docs/specs/sensors/<doc>.md revision <N> (commit <sha>).
Implemented from docs/specs/sensors/<doc>.md revision <N> (commit <sha>).
No other external sensor documentation was used.
```

### Implementer attestation

- [ ] This implementation references only `docs/specs/sensors/**` (at
      the revisions pinned above) and this repository.
- [ ] Every spec document pinned above is implementation-ready (no
      unresolved `TODO(provenance)` markers; status is not
      `Draft — not implementation-ready`).
- [ ] I did not consult LibreHardwareMonitor, OpenHardwareMonitor,
      Linux kernel, lm-sensors, or any decompiled monitoring tool
      while writing this implementation (full prohibited-source list:
      `.github/instructions/clean-room-sensors.instructions.md`).
- [ ] Register access added by this PR is read-only; the only writes
      are those the pinned specs document as required for reads
      (e.g. Super I/O config keys, bank select), and the ecosystem
      mutex conventions are honored.

### Reviewer attestation

Reviewers: copy the checklist below into your approval review
comment, with both boxes checked. Do not approve without it.

```markdown
- [ ] I reviewed this implementation only against
      `docs/specs/sensors/**`, this repository, and the pinned spec
      revision.
- [ ] I did not consult LibreHardwareMonitor, OpenHardwareMonitor,
      Linux kernel, lm-sensors, or decompiled monitoring tools while
      reviewing this implementation.
```

## Screenshots / Videos

<!-- If applicable, add screenshots or videos to demonstrate the changes -->

## Test Plan

<!-- Pure-function decoders with dump fixtures per the repo testing policy -->

- [ ] Manual testing
- [ ] Unit tests

## Checklist

- [ ] Self-reviewed the code
- [ ] Linting and formatting pass (`npm run lint && npm run format` / `cargo tauri-lint && cargo tauri-fmt`)
- [ ] Tests pass (`npm test` / `cargo tauri-test`)
- [ ] No new warnings or errors
