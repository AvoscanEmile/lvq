# Devlog Entry 03 — Implementation of the Type-Safe LVM Provisioning Parser

**Date**: 2026-03-03

**Author**: Emile Avoscan

**Target Version**: 0.1.0

## Main Objective

The primary objective of this development cycle was to architect a robust, zero-dependency command-line interface (CLI) for `lvquick` that maps raw user input directly to validated domain models. The goal was to replace generic parsing frameworks with a custom, two-pass tokenizer capable of handling LVM-specific complexities—such as multi-path device strings and percentage-based sizing—while enforcing strict data invariants at the application boundary.

## Implementation

### Hardening the Core Domain Model (`skel.rs`)

The existing data structures were refactored to prioritize type safety over primitive types. A `ValidPercentage` tuple struct was introduced to encapsulate the invariant that a percentage must reside within the $1..100$ range. The `SizeUnit` enum was expanded to support LVM-native targets (`%FREE`, `%VG`, `%PVS`), and the `LvRequest` struct was updated to include a nested `fsMount` struct, facilitating a cleaner mapping for logical volume creation and subsequent mounting logic.

### Distillation via `FromStr` Traits

To decouple string parsing from CLI logic, comprehensive `FromStr` implementations were written for `SizeUnit`, `Filesystem`, and `LvRequest`. This allowed the parser to delegate the complexity of string splitting and unit conversion to the types themselves. For instance, the `LvRequest` parser was designed to handle complex colon-delimited strings (e.g., `data:10G:ext4:/mnt/data`), ensuring that each segment is distilled into its corresponding domain variant or rejected early with a descriptive error.

### Orchestrator Pass: Global State and Subcommand Discovery

The `parse_cli` function was implemented as a high-level orchestrator. A linear scan of `env::args()` was developed to identify global toggles (such as `-y` or `--auto-confirm`) and the primary "Naked Token" representing the subcommand. By utilizing a lookbehind guard (`previous.starts_with('-')`), the orchestrator was made capable of distinguishing between a subcommand verb and a value associated with a flag, allowing for flexible command ordering.

### Worker Pass: Command-Specific Tokenization

A delegated worker function, `parse_provision`, was created to handle the specific requirements of the provisioning command. This function implements a stateful `while` loop that consumes tokens based on the identified flags (`--pv`, `--vg`, `--lv`). This separation ensures that the worker only concerns itself with domain-specific data, while skipping global modifiers that were already processed by the orchestrator pass.

### Input Sanitization and Boundary Guards

Specific guards were integrated into the tokenizer to prevent common CLI failures. Multi-path inputs for physical volumes were sanitized using `.filter(|s| !s.is_empty())` to prevent the creation of empty `PathBuf` objects. Additionally, lookahead guards were implemented to ensure that flags do not "swallow" other flags if a value is missing, maintaining the integrity of the token stream.

## Challenges & Resolutions

**Dynamic Subcommand Overwriting**
* **Challenge**: Because the orchestrator loop continued scanning after finding the subcommand to look for global flags, subsequent non-flag arguments (typos) would overwrite the `subcommand` variable.
* **Resolution**: A `subcommand.is_empty()` check was added to the discovery conditional, effectively locking the primary verb upon its first valid encounter.


**Variable Binding in Match Patterns**
* **Challenge**: Attempting to use the `subcommand` variable directly in a `match` arm within the worker function resulted in a new variable binding rather than a value comparison, causing the arm to act as a catch-all.
* **Resolution**: The "Verb" was hardcoded as a literal string in the skip arm of the worker loop, ensuring the parser strictly bypasses the known subcommand token without accidentally shadowing other patterns.


**Global State Isolation**
* **Challenge**: Initial designs attempted to parse the `-y` toggle inside the subcommand worker, leading to a loss of state when returning the `Command` enum, which lacked an `auto_confirm` field.
* **Resolution**: The `auto_confirm` variable was moved to the orchestrator level, where it is used to wrap the returned `Command` into a final `Action` struct, preserving the user's intent across the entire execution scope.


**Stable Rust Compatibility**
* **Challenge**: The use of "let-chains" (e.g., `if let Some(x) = y && condition`) was identified as an unstable feature that would prevent compilation on standard Rust toolchains.
* **Resolution**: Nested `if` blocks were used to replicate the logic, ensuring the parser remains compatible with the stable compiler while maintaining the same level of safety.

## Outcomes

The resulting architecture is a deterministic, zero-dependency parser that guarantees that any `Action` passed to the execution engine is logically and physically valid. By moving validation to the `FromStr` implementations of the core types, the CLI logic remains concise and easily extensible for future subcommands.

## Reflection

This development cycle reinforces the philosophy that the CLI is not merely a string-processing layer but an extension of the domain model. By refusing to use a generic parser, it was possible to implement LVM-specific grammar rules—like colon-delimited volume requests and percentage targets—that would be cumbersome to define in a standard framework. This approach ensures that the "Distillation Pipeline" from raw input to domain model is both transparent and highly resilient.

The transition from a single-pass loop to a two-pass orchestrator/worker pattern was a pivotal moment in the cycle. It demonstrated that architectural complexity at the input layer is often necessary to achieve simplicity and safety in the execution layer. By resolving the "binding traps" and "ghost variables" early, a foundation was laid that treats user input as a potentially hostile stream that must be distilled into "known-good" types before any system-level actions are considered.

## Next Steps

The next cycle will focus on the implementation of the execution engine within `main.rs`. This will involve translating the validated `Action` and `Call` enums into actual system commands via `std::process::Command`, including the integration of a confirmation prompt for non-automated tasks. 
