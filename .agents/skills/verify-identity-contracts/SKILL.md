---
name: verify-identity-contracts
description: Verify the id/key contract between backend producers and frontend consumers before implementing any change that joins, selects, persists, or attributes entities keyed by backend-produced ids (GPUs, storage devices, sensors, processes). Use when adding a selector, attribution UI, persistence of a selected id, a join across two data sources, or an e2e fixture for multi-source data.
---

# Verify Identity Contracts

## When This Fires

Any of these, whether or not the change is UI work:

- an entity is read from more than one data source — a one-shot inventory
  fetch, the live monitor stream, the archive, or a persisted store key;
- a surface selects, attributes, or names an entity keyed by a
  backend-produced id;
- an id is persisted and restored across sessions;
- a fixture, factory, or seed supplies ids for any of the above.

Each step names the boundary it guards, and applies exactly when the change
crosses it — no more:

| Step | Applies when |
| --- | --- |
| 1. Contract | always, in any layer |
| 2. Fixture realism | the change supplies fixture, factory, or seed ids, in any layer |
| 3. Resolution owner | `src/` derives which entity is effective |
| 4. Persistence and migration | an id is stored, restored, or changes namespace |
| 5. Surface parity | a surface renders a resolution state |

So a Core change that only produces ids owes the contract and, if it ships a
fixture, fixture realism — not a React hook. Attribution UI that persists
nothing owes the contract, the resolution owner, and surface parity — not a
migration.

Within its own boundary no trigger is exempt. Attribution UI and fixture work
are covered rather than incidental: the contract is what makes an attribution
honest, and a fixture is where a wrong contract gets certified as correct.

The motivating failure: the `getHardwareInfo` inventory and the monitor stream
key GPUs in different id namespaces on every platform (ADR 0016). Building a
selector on an assumed shared namespace produced eleven review rounds of
consequence bugs, all discoverable up front by reading the producers.

## 1. Write The Contract Table Before Any Consuming Code

*Applies to every trigger, in every layer.*

For every id the change consumes or produces — including one that only a
fixture supplies — read the producing code (or the change itself, when it is
the producer) and record:
the source, the per-platform id shape, and whether any two sources share a
namespace. When they do not, name the join key the sources actually share
(typically the reported name) and state where the join must refuse
(ambiguity: the key matches more than one entry on either side).

Record it at the smallest durable owner that fits the change, per the
decision-preservation rule in `AGENTS.md`: a focused test or a code comment
next to the join for an implementation-level fact, commit or PR context for a
change-local why, and an ADR only when the contract itself is an
architecturally significant decision. A local change that ships no PR still
needs the reading — it does not need paperwork.

Do not infer the contract from fixtures, tests, or frontend types — those can
encode the same wrong assumption the change is about to build on.

## 2. Fixtures Must Reproduce The Namespace Split

*Applies wherever the change supplies ids — a Rust test fixture can flatten a
namespace exactly as an e2e one can.*

A fixture may share one id across two sources only if production does, and
its ids take their shapes from the contract table — `nvapi:12345` beside
`12345` — not two arbitrary strings that merely differ. One that flattens a
real namespace difference certifies broken joins: the classic GPU selector
shipped non-functional on real hardware while its e2e passed, because
`GPU_FIXTURES` used one id for both sources.

## Frontend Consumption

The remaining steps describe how `src/` consumes a contract. Each applies only
when the change crosses that step's boundary; a change that produces or
persists ids in `core/` or `src-tauri/` keeps the fact where its layer owns it
rather than moving it into the frontend.

## 3. One Resolution Rule, One Owner

*Applies when the change derives which entity is effective.*

Grep every consumer of the shared selection atom or key. The question "which
entity is effective" must be answered in exactly one place — a hook or derived
atom — that every surface consumes. If resolution logic already exists in two
components, centralize it in this change before adding a third. Duplicated
resolvers are how one surface labels an entity while another surface renders a
different entity's values.

Subscriptions to per-sample atoms belong in the component that renders the
value, never in a screen parent (ADR 0010 rendering-cost rule).

## 4. Persistence And Migration

*Applies when the change stores, restores, or re-namespaces an id; skip it
only when it does none of those.*

State: which namespace is stored today, which namespace shipped versions
stored, and what translates one to the other. An id persisted for the first
time has no shipped value — state that, and the translation work is done. The migration must run at an
always-mounted boundary (an app-level hook), never inside a screen — some
navigation layouts never mount that screen. It must fetch its own inputs,
because a restart can land on a view that fetches nothing.

Migration is one-way. Cover both cases that exist: a legacy value translates
to the current namespace, and a value already in the current namespace is left
unchanged — including one that is simply absent this session, which is intent
to preserve rather than a value to rewrite. Do not build or test a reverse
translation; writing the selection back into the obsolete namespace is the
failure, not the fallback.

## 5. Surface Parity For Resolution States

*Applies when the change renders a resolution state.*

List every surface that renders the value. Every state the resolution can
produce — unavailable, ambiguous, not-yet-measured — must render on each of
them; a blank on one surface reads as idle, not as missing.

## Exit Checklist

Check the boxes whose boundary the change crosses; the first is always one of
them.

- [ ] Contract recorded at its smallest durable owner, sourced from
      producer code
- [ ] Fixture ids differ across sources wherever production's do
      *(supplies ids)*
- [ ] Exactly one resolution owner; other consumers import it
      *(derives an effective entity)*
- [ ] Migration at an always-mounted boundary; legacy values translate and
      current values stay unchanged *(persists an id)*
- [ ] Each resolution state asserted on each surface
      *(renders a resolution state)*
