# Hardware Archive Investigation Issues

The [Design Doc](../design/hardware-archive-duckdb.md) owns the recommended
structure and its trade-offs. [#2052](https://github.com/shm11C3/HardwareVisualizer/issues/2052)
tracks delivery and its existing requirements. Issue descriptions own concrete
work, validation and progress; this index does not duplicate them as a spec.

| Issue | Investigation |
| --- | --- |
| [#2083](https://github.com/shm11C3/HardwareVisualizer/issues/2083) | Exact SQLite values, native representations and query compatibility |
| [#2084](https://github.com/shm11C3/HardwareVisualizer/issues/2084) | Minute writes, concurrent reads, interruption/restart and ongoing retention/reclamation |
| [#2085](https://github.com/shm11C3/HardwareVisualizer/issues/2085) | Rust integration, complete database coverage and migration/topology |

These investigations can start in parallel. Full database conversion depends
on the selected value representation and observed native lifecycle behavior.
Results update the design and guide the next implementation Issues through
ordinary implementation and PR review. Further work is split as its scope
becomes concrete.
