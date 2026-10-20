# Known defects in core-rs

This document covers only clear defects confirmed in the core-rs code.
It does not cover the behavior of dependent libraries or the OS themselves.
Design decisions and future plans that span the workspace are covered by [DESIGN.md](./DESIGN.md#11-design-decisions).
Topics specific to the RocketPack compiler are covered by [RocketPack compiler design](./design/rocketpack-compiler.md#10-design-decisions).

When an item is filed in an external issue tracker, add its number to the Issue column of the list; when an item is fixed, delete its row from the list and its child document under `docs/issues/`.
Line numbers are a snapshot at the time of investigation, so recheck them when fixing.

| Item | Severity | Investigated on | Issue |
| --- | --- | --- | --- |

There are currently no items in the list.

## Not included here

| Topic | Treatment in the design |
| --- | --- |
| Remote dependencies and lockfiles | [Deferred items in the RocketPack compiler design](./design/rocketpack-compiler.md#102-deferred) |
| Multiple Rust module roots | [Deferred items in the RocketPack compiler design](./design/rocketpack-compiler.md#102-deferred) |
