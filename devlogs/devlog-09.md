# Devlog Entry [09] — Strengthening the LVM Planner via Formal Verification and Property-Based Fuzzing

**Date**: 2026-03-11

**Author**: Emile Avoscan

**Target Version**: 0.1.0

## Main Objective

The primary focus of this development cycle was the construction of a robust testing harness for the `planner` module. The objective was to ensure that high-level provisioning commands are correctly and safely translated into a sequential vector of LVM system calls. This involved verifying structural integrity, operational ordering, and the handling of specialized filesystem branches such as swap space.

## Implementation

The implementation was approached through a layered strategy, moving from core type definitions to complex transformation logic.

### Formal Verification via Kani (Attempted)

An attempt was made to utilize the Kani Rust Verifier to provide formal proofs for operational ordering. The goal was to prove that for every valid `Action` passing through `plan_provision`, the output `Vec<Call>` would maintain a strictly valid LVM dependency graph (e.g., Physical Volumes initialized before Volume Groups). While initial scaffolding was built, including the application of `kani::Arbitrary` to core enums like `SizeUnit` and `Filesystem`, the implementation was ultimately pivoted due to the complexity of symbolic execution involving heap-allocated types.

### Property-Based Testing Harness (Proptest)

A comprehensive fuzzer was implemented using the `proptest` crate. Custom strategies were defined to generate valid LVM inputs, including `SizeUnit`, `Filesystem`, and `LvRequest` structures. These strategies utilized regex-based string generation (e.g., `[a-z0-9_]{1,10}`) to ensure inputs conformed to the parser's expectations. Two primary property tests were established:

1. **Dependency Ordering Test**: Verified that `Call` sequences followed the lifecycle of `PvCreate` -> `VgCreate` -> `LvCreate` -> `Mkfs/MkSwap` -> `Mount`.
2. **Structural Invariant Test**: Confirmed that every input request was accounted for in the output without data loss or path mangling, specifically focusing on the consistency of the `/dev/{vg}/{lv}` device path generation.

### Specialized Logic Unit Testing

Targeted unit tests were implemented to verify branching logic that property tests might overlook. This included specific checks for the "Swap Exception," ensuring `MkSwap` is utilized instead of `Mkfs`, and verifying that `fstab` entries for swap correctly utilize `none` as a mount path. Additionally, logic for logical volumes without defined mount points was verified to ensure unnecessary directory and mount calls were suppressed.

### Wrapper Integrity Verification

The `plan` function in `mod.rs` was verified to ensure correct data propagation from the `Action` input to the `Draft` output. Tests were implemented to confirm that metadata such as `auto_confirm` flags and `DraftStatus::Pending` were correctly initialized during the transformation process.

## Challenges & Resolutions

### Kani State Space Explosion and Standard Library Constraints

* **Challenge**: The attempt to use Kani for formal verification resulted in significant compilation errors and eventual timeouts/path-abortions. This was triggered by Kani's inability to symbolically execute the `std::string` and `std::path` allocation logic, leading to an explosion of the state space.
* **Solution**: After attempting to bypass this using "Mock" structures and manual `Arbitrary` implementations, the decision was made to pivot to `proptest`. This allowed for high-coverage testing of the transformation logic without requiring the solver to reason about the internal memory layout of the Rust standard library.

### Naive Suffix Matching in Structural Invariants

* **Challenge**: A failure was observed in the structural invariant test at approximately 30 iterations. The error revealed a name collision where an LV named "x" was incorrectly matched against a device path for an LV named "0x" due to a naive `ends_with` string check.
* **Solution**: The invariant check was rewritten to utilize `PathBuf::file_name()` comparison. This ensured that the test logic correctly identified path components as distinct entities, resolving the false positive and allowing the suite to scale.

## Testing & Validation

Validation was performed by running the integrated `proptest` and unit test suite.

* **Scale**: The structural invariant tests were scaled to **100,000 cases** locally, with a release-target configuration of **10,000,000 cases**.
* **Observation**: All invariants held across the extended test run. Operational ordering was confirmed to be monotonic, and the "Swap" and "No-Mount" branches were verified to produce the exact intended system calls.

## Outcomes

The `planner` module was successfully transformed into a verified "black box." The project now possesses a high-fidelity testing harness that guarantees the structural and sequential correctness of LVM provisioning plans. The transition from high-level intent to discrete system calls is now resilient against naming collisions, ordering violations, and metadata loss.

## Reflection

This development cycle highlighted the inherent tension between formal verification and high-level abstractions. While Kani remains a powerful tool for arithmetic and bit-level logic, the overhead of verifying the Rust standard library's heap management often outweighs the benefits for architectural-level mapping functions. The pivot to property-based testing proved that for complex transformations, high-iteration fuzzing provides a more practical and equally robust path to confidence.

The discovery of the "x" vs "0x" naming collision was a pivotal moment. It underscored the necessity of "testing the tests"—even the most rigorous validation logic can fall prey to simple string-matching fallacies. By hardening the testing harness itself, the reliability of the entire `lvq` pipeline was significantly elevated.

## Next Steps

With the `planner` module's reliability established, focus will shift to the `verifier` and `exec` modules. The next objective is to implement harnesses for pre-flight system checks and to develop a mocking layer for actual LVM command execution. Finally, an End-to-End (EtoE) harness will be constructed to verify the entire pipeline from parsing to execution.
