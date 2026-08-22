---
name: verify-identity-contracts
description: Verify the id/key contract between backend producers and frontend consumers before implementing any change that joins, selects, persists, or attributes entities keyed by backend-produced ids (GPUs, storage devices, sensors, processes). Use when adding a selector, attribution UI, persistence of a selected id, a join across two data sources, or an e2e fixture for multi-source data.
---

# Verify Identity Contracts

## When This Fires

The change consumes an entity that exists in more than one data source — a
one-shot inventory fetch, the live monitor stream, the archive, or a persisted
store key — or persists and restores an id across sessions.

The motivating failure: the `getHardwareInfo` inventory and the monitor stream
key GPUs in different id namespaces on every platform (ADR 0016). Building a
selector on an assumed shared namespace produced eleven review rounds of
consequence bugs, all discoverable up front by reading the producers.

## 1. Write The Contract Table Before Any UI Code

For every id the change consumes, read the *producing* Rust code and record in
the PR description or ADR: the source, the per-platform id shape, and whether
any two sources share a namespace. When they do not, name the join key the
sources actually share (typically the reported name) and state where the join
must refuse (ambiguity: the key matches more than one entry on either side).

Do not infer the contract from fixtures, tests, or frontend types — those can
encode the same wrong assumption the change is about to build on.

## 2. One Resolution Rule, One Owner

Grep every consumer of the shared selection atom or key. The question "which
entity is effective" must be answered in exactly one place — a hook or derived
atom — that every surface consumes. If resolution logic already exists in two
components, centralize it in this change before adding a third. Duplicated
resolvers are how one surface labels an entity while another surface renders a
different entity's values.

Subscriptions to per-sample atoms belong in the component that renders the
value, never in a screen parent (ADR 0010 rendering-cost rule).

## 3. Fixtures Must Reproduce The Namespace Split

An e2e or unit fixture may share one id across two sources only if production
does. A fixture that flattens a real namespace difference certifies broken
joins: the classic GPU selector shipped non-functional on real hardware while
its e2e passed, because `GPU_FIXTURES` used one id for both sources.

## 4. Persistence And Migration

State: which namespace is stored today, which namespace shipped versions
stored, and what translates one to the other. The migration must run at an
always-mounted boundary (an app-level hook), never inside a screen — some
navigation layouts never mount that screen. It must fetch its own inputs,
because a restart can land on a view that fetches nothing.

## 5. Surface Parity For Resolution States

List every surface that renders the value. Every state the resolution can
produce — unavailable, ambiguous, not-yet-measured — must render on each of
them; a blank on one surface reads as idle, not as missing.

## Exit Checklist

- [ ] Contract table recorded, sourced from producer code
- [ ] Exactly one resolution owner; other consumers import it
- [ ] Fixture ids differ across sources wherever production's do
- [ ] Migration at an always-mounted boundary, tested in both directions
- [ ] Each resolution state asserted on each surface
