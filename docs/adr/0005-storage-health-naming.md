# Storage Health Naming

Status: accepted

Storage Health is the product concept for retained storage device health records, while SMART is one input/source used to collect those signals. Before releasing Storage Health, we decided to use Storage Health names for persisted settings, IPC commands, DTOs, workers, and database tables, and to reserve SMART naming for provider-level collection details where the underlying protocol is actually being discussed.

Existing `storageSmart` settings are read only for compatibility and are rewritten as `storageHealth` settings on save. The existing v6 database migration keeps its historical `storage_smart_daily_snapshots` table name, and a later forward migration renames that table to the Storage Health name so already-created development databases keep their data.

Storage Health Record names should describe the retained daily device-health record, not a specific UI placement. Dashboard and future historical views can share the same record-shaped DTO instead of introducing Dashboard-specific storage health data names.

User-facing labels and settings descriptions should use Storage Health language. SMART may still appear in provider names, low-level diagnostic messages, or raw health-signal explanations where it identifies the actual collection source.
