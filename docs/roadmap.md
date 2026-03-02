# **lvquick Roadmap to v1.0**

## **Introduction**

This document outlines the official development roadmap for lvquick, a Rust-based transactional wrapper for LVM2. The project's mission is to eliminate high-risk storage mistakes by enforcing a deterministic **Plan → Verify → Confirm → Execute** lifecycle. Each phase below represents a major development milestone corresponding to one of the core commands, ensuring that every high-risk storage operation is fully journaled, validated, and recoverable.

The roadmap is structured from foundational operations (`provision`) to advanced workflows (`snap-back`), culminating in a fully operational, enterprise-ready `v1.0` release.

## **Phase 1: `provision`**

**High-Level Goal:** Implement the foundational workflow to safely provision storage from raw disks to mounted filesystems.

**Breadth and Depth of Tasks:**

* Ingest and validate system state (LVM, fstab, mounts, filesystem signatures)
* Generate immutable `Vec<LvmAction>` plans
* Require explicit confirmation before execution
* Sequential, journaled execution with rollback support
* Post-condition verification of mounted filesystems

**Success Metric:** A raw block device can be provisioned into a fully mounted, fstab-consistent filesystem in a single deterministic transaction.

## **Phase 2: `decommission`**

**High-Level Goal:** Safely remove storage without leaving dangling references or inconsistent fstab entries.

**Breadth and Depth of Tasks:**

* Unmount filesystems cleanly
* Remove LV, VG, and optionally PV in proper order
* Update `/etc/fstab` atomically
* Validate no dangling references remain
* Journal transactions and support rollback

**Success Metric:** Storage can be fully decommissioned deterministically, with no ghost mounts or lost fstab entries.

## **Phase 3: `shrink`**

**High-Level Goal:** Enable safe shrinking of logical volumes where supported, avoiding data loss.

**Breadth and Depth of Tasks:**

* Enforce strict order: filesystem shrink → LV shrink
* Detect minimum filesystem sizes
* Generate and execute immutable action plans
* Verify post-conditions and journal operations
* Support rollback on failure

**Success Metric:** LV shrinking can be executed safely, reproducibly, and with full verification of final sizes.

## **Phase 4: `evacuate`**

**High-Level Goal:** Safely remove a PV from a Volume Group without replacement.

**Breadth and Depth of Tasks:**

* Calculate required free extents on remaining PVs
* Perform deterministic `pvmove` operations
* Reduce and remove the PV
* Verify VG integrity and journal execution

**Success Metric:** PV can be evacuated safely, guaranteeing all data is migrated and VG remains consistent.

## **Phase 5: `replace-disk`**

**High-Level Goal:** Enable live disk replacement in a Volume Group.

**Breadth and Depth of Tasks:**

* Add new PV and extend VG
* Perform `pvmove` from old PV to new PV
* Remove old PV safely
* Journal all steps and enable resumable operations

**Success Metric:** A disk can be replaced live, with minimal downtime and full transaction recovery.

## **Phase 6: `shrink-xfs`**

**High-Level Goal:** Implement XFS shrink through canonical migration, as XFS cannot shrink in place.

**Breadth and Depth of Tasks:**

* Create new LV with correct size
* Format and copy data safely
* Swap mounts and update fstab atomically
* Remove original LV after verification
* Journal the full workflow

**Success Metric:** XFS shrink operations can be performed reliably with no data loss and full rollback capability.

## **Phase 7: `accelerate`**

**High-Level Goal:** Enable SSD caching safely for existing HDD-backed LVs.

**Breadth and Depth of Tasks:**

* Calculate correct cache-pool and metadata sizes
* Attach cache in writeback/writethrough mode
* Verify mode and ratio correctness
* Journal and rollback operations

**Success Metric:** SSD caching can be applied reliably without errors or misconfiguration.

## **Phase 8: `snap-back`**

**High-Level Goal:** Create application-consistent snapshots safely.

**Breadth and Depth of Tasks:**

* Detect filesystem type and freeze appropriately
* Create LVM snapshot
* Optionally mount snapshot read-only
* Verify snapshot consistency
* Journal long-running operations

**Success Metric:** Snapshots are consistent, verifiable, and safely mountable, supporting backups and testing.

## **Phase 9: CLI & Automation Enhancements**

**High-Level Goal:** Make lvquick fully automation-ready and machine-readable.

**Breadth and Depth of Tasks:**

* JSON plan output for pipelines
* Non-interactive mode (`-y --force`)
* Transaction inspection commands (`lvq history`, `lvq continue`, `lvq repair`)

**Success Metric:** All core commands can be used in automated workflows and inspected programmatically.

## **Phase 10: Full Operational Suite (v1.0)**

**High-Level Goal:** Deliver a complete, deterministic, journaled LVM safety layer.

**Breadth and Depth of Tasks:**

* Stabilize all eight core commands
* Verify transaction journaling and rollback across workflows
* Comprehensive unit and integration tests
* Enterprise and automation readiness
* Perform an architectural audit to refactor the project into a clear SoC structure

**Success Metric:** lvquick 1.0 provides deterministic, auditable, and production-ready transactional storage operations.

## **Summary Table**

| Phase | Focus Area                      | Target Version |
| :---- | :------------------------------ | :------------- |
| 1     | `provision`                     | 0.1            |
| 2     | `decommission`                  | 0.2            |
| 3     | `shrink`                        | 0.3            |
| 4     | `evacuate`                      | 0.4            |
| 5     | `replace-disk`                  | 0.5            |
| 6     | `shrink-xfs`                    | 0.6            |
| 7     | `accelerate`                    | 0.7            |
| 8     | `snap-back`                     | 0.8            |
| 9     | CLI & automation enhancements   | 0.9            |
| 10    | Full operational suite (`v1.0`) | 1.0            |
