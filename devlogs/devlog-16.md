# Devlog Entry 16 — Native Execution Migration and Verifier Canonicalization

**Date**: 2026-03-15

**Author**: Emile Avoscan

**Target Version**: 0.1.1

## Main Objective

The execution engine of `lvq` was overhauled to transition from raw shell string evaluations to native `std::process::Command` invocations. This architectural shift was deemed necessary to support future Kubernetes CSI and Ansible integrations, which require structured data and granular exit signals. Concurrently, the state-convergence verifier was hardened to resolve path mismatch anomalies that triggered false-positive "Dirty" states during large-scale provisioning transactions.

## Implementation

The implementation required a fundamental shift in how the provisioning intent was stored, passed, and executed. The goal was to eliminate reliance on the shell without sacrificing the perfect auditability that the user relies on before confirming a transaction.

### The Hybrid Instruction Model

Initially, a complete migration to a `Vec<std::process::Command>` was proposed. However, it was determined that this would break the "What You See Is What You Get" (WYSIWYG) contract with the user, as the actual command structure would be hidden behind a reconstructed display string. To resolve this, a hybrid `Instruction` struct was introduced. This struct encapsulates both the `shell_string` (the human-readable intent presented to the user and written to `/var/log/lvq`) and the `command_call` (the native `std::process::Command` object). This ensured that the compilation phase generated a unified, immutable source of truth for both human operators and machine orchestrators.

### Tri-Step Fstab Refactoring

The `/etc/fstab` modification logic previously relied heavily on shell-specific features (variable assignment, conditionals, and standard out redirection) condensed into a single string. To fit the new native execution model without prematurely building a pure-Rust `fstab` parser, the logic was segmented into three distinct `Instruction` blocks. The initial backup (`cp -p`) and the final atomic commit (`mv`) were migrated to native `Command` calls, while the `blkid` resolution and `echo` append logic were safely sandboxed within a constrained `sh -c` instruction.

### Execution Engine and Test Suite Alignment

The `apply_execution` function in `src/exec/mod.rs` was updated to iterate over the new `Vec<Instruction>`. The user confirmation prompt and the forensic log still print the `shell_string`, but the actual system changes are now invoked directly via `instruction.command_call.status()`. Following this, the `proptest` suites were refactored. Direct string comparisons on the execution list were mapped to the `.shell_string` properties, and dummy `Command::new("true")` objects were instantiated to satisfy the struct constraints during pure state testing.

### Canonicalizing the System Verifier

During high-volume testing, the verifier returned a `CRITICAL` state mismatch despite the execution succeeding. A "loud" verification loop was temporarily implemented to output per-step matching, which revealed that step 53 (`MkSwap`) was flagged as `MISSING`. It was discovered that `/proc/swaps` and `/proc/mounts` report physical device nodes (e.g., `/dev/dm-10`), whereas `lvq` tracks LVM symlinks (e.g., `/dev/mapper/tank_vg-swap_space`). To fix this, `std::fs::canonicalize()` was injected into all critical probes (`probe_swap_active`, `probe_mount_exists`, `probe_fs_exists`, `probe_block_device_size`, `probe_is_full_disk`, and `probe_fstab_exists`).

## Challenges & Resolutions

### WYSIWYG Contract vs. Native Execution

* **Challenge:** Moving entirely to `std::process::Command` meant the executed binary arguments might drift from the string presented to the user for confirmation, introducing a severe deception risk.
* **Resolution:** The `Instruction` struct was adopted. By generating the `shell_string` and the `command_call` concurrently from the same `Call` variant within the provisioner, synchronization between the user's confirmed intent and the kernel's execution was mathematically guaranteed.

### Verifier Math Drift and False Positives

* **Challenge:** A 77-step complex provisioning transaction was successfully applied, but `verify_done` rejected the final state. It was initially hypothesized that a decrementing logic bug (`total_calls -= 1`) for support calls like `Mkfs` and `Mkdir` was causing mathematical drift.
* **Resolution:** A highly verbose per-step verification audit was added to the engine. This telemetry proved the math was sound but isolated the failure to specific string-matching blind spots regarding block device symlinks, pivoting the debugging effort entirely toward probe resolution.

### Kernel Naming Conventions vs. LVM Logic

* **Challenge:** Probes reading from `/proc/swaps` and `/proc/mounts` failed to match expected LVM device paths because the kernel maps these to internal device-mapper nodes (`/dev/dm-X`).
* **Resolution:** Robust canonicalization was implemented across the entire `verifier` module. Target paths and queried paths are now rigorously resolved to their absolute physical node paths before any string comparison is attempted. In `probe_fstab_exists`, a multi-tiered fallback was added to check strings, canonical UUIDs, and canonical physical paths to ensure absolute certainty.

## Testing & Validation

Validation was conducted using a rigorous, two-pronged approach. First, the automated `proptest` harness (containing over 40 property tests) was executed to ensure the new `Instruction` struct maintained all previous atomicity and lifecycle invariants without panicking. Second, a massive 77-step simulated provisioning plan—carving 5 loopback devices into a heavily partitioned Volume Group with diverse filesystems (XFS, Ext4, Btrfs, Vfat, Swap)—was executed. The execution applied flawlessly, and with the newly canonicalized probes, the verifier successfully confirmed the exact convergence of the expected state against the actual machine state.

## Outcomes

The `lvq` execution layer is no longer a wrapper for shell commands; it is a true native execution engine. The codebase is now structurally prepared to return granular, serialized JSON error payloads for Kubernetes and Ansible integrations (Phase 10). Furthermore, the symlink resolution upgrades have eliminated intermittent "Dirty" state anomalies, ensuring the state-convergence verifier is as reliable and deterministic as the execution planner.

## Reflection

The debate between maintaining raw strings for transparency and adopting `Command` objects for security highlighted the core tension of building high-assurance system tools. The compromise found in the `Instruction` struct reinforces the project's pillar of "Boring Reliability." It was demonstrated that deep transparency and strict machine-level safety do not have to be mutually exclusive if the "compiler" step is architected correctly. The debugging phase also served as a stark reminder that in Linux storage, a path is merely a suggestion until it is canonicalized.

## Next Steps

With the `provision` engine stabilized and natively executed, development will proceed to Phase 2 of the roadmap. The immediate focus will shift to implementing the `decommission` module, establishing the inverse lifecycle of the current engine. Concurrently, Reflexive VM E2E (End-to-End) testing will be integrated into the CI pipeline to automate destructive testing across ephemeral environments.
