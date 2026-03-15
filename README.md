# lvquick (`lvq`)

**The Transactional State-Convergence Engine for LVM2.**

Manual LVM management is a **"Russian Roulette"** of shell scripts, string parsing, and destructive commands. **`lvquick`** eliminates this risk by treating Linux storage management as a compiled, verified transaction. It transforms your declarative intent into a mathematically proven, journaled execution plan.

`lvq` is not a replacement for LVM2, a daemon, or a storage orchestrator. It is a **deterministic safety layer** for production systems, focused on operational correctness and transaction integrity. It does not attempt to be clever—only correct.

## Core Pillars

* **Transactional Integrity:** Every operation is modeled as an immutable plan before a single byte is written to disk.
* **Refusal Over Confusion:** If the system state is ambiguous or a partial configuration is detected, `lvq` stops. It refuses to "guess" user intent or proceed with "drifted" states.
* **Provable Correctness:** Critical arithmetic—specifically Physical Extent alignment and Exabyte-scale sizing—is formally proven using **Kani** symbolic execution.
* **Boring Reliability:** High-risk operations become predictable and repeatable. By enforcing a strict **Plan → Verify → Confirm → Execute** lifecycle, storage management becomes unremarkably safe.

## Why `lvq` Wraps LVM2

LVM2 is a powerful imperative system: commands execute immediately, and multi-step workflows (like resizing, migrating data, or updating fstab) are prone to human fatigue and extent miscalculations. `lvq` addresses these risks by:

1. **Ingesting** live state via `lvm fullreport --reportformat json`. We explicitly avoid fragile C bindings, relying on LVM2's stable CLI-to-JSON interface.
2. **Generating** an immutable action plan that ensures invariants (e.g., `LV_new_size ≥ FS_size`).
3. **Verifying** safety before execution, detecting "blunder risks" such as busy mounts or inconsistent fstab entries.
4. **Journaling** every step to `/var/log/lvq` for forensic auditing and deterministic recovery.
5. **Validating** final system state to ensure `Expected State == Actual State`.

## Quick Start

### Installation

`lvq` is a single, static binary with zero runtime dependencies beyond standard LVM2 userspace tools.

```bash
# Clone and build
cargo build --release
sudo cp target/release/lvq /usr/local/bin/

```
### Simple Provisioning

Provision a raw disk into a Volume Group, carve out a Logical Volume, format it with XFS, and mount it persistently in one atomic transaction:

```bash
sudo lvq provision \
  --pv /dev/sdb \
  --vg data_vg \
  --lv logs:10G:xfs:/var/log/app
``` 

### Advanced Provisioning

`lvq` supports multi-disk Physical Volume (PV) pools and multiple Logical Volume (LV) declarations in a single transaction. You can pass multiple `--pv` flags or colon-separated paths.

```bash
sudo lvq provision \
  --pv /dev/sdb1:/dev/sdc1 \
  --pv /dev/sdd1 \
  --vg enterprise_vg \
  --lv logs:10G:xfs:/var/log/app \
  --lv data:500G:ext4:/mnt/data \
  --lv swap:8G:swap
```

**What `lvq` does under the hood:**

1. **Probes** all targeted devices (`sdb1`, `sdc1`, `sdd1`) to build an in-memory `SystemState`.
2. **Verifies** all devices are free of existing filesystems and `fstab` collisions.
3. **Calculates** the aggregate capacity of the PV pool and verifies it can house the 518GB of requested LVs.
4. **Generates** a sequential plan: `pvcreate` (x3) → `vgcreate` → `lvcreate` (x3) → `mkfs/mkswap` (x3) → `mkdir` → `mount` → `fstab`.
5. **Prompts** for confirmation, then **Executes** and logs to `/var/log/lvq`.

**The Safety Gate:** Unless invoked with `-y`, `lvq` displays the full plan and system warnings, requiring an explicit `Y` to proceed.

## Hardening & Verification

Built with a "high-assurance" mindset, `lvq` is subjected to multiple layers of rigorous automated verification to guarantee panic-free, mathematically safe execution:

* **Formal Verification (`kani`):** Size calculations and unit conversions (`SizeUnit::to_bytes()`) are mathematically proven to be free of integer overflows and out-of-bounds panics at the bit-level.
* **Property-Based Testing (`proptest`):** Over **40 property tests** validate logic atomicity. We generate tens of thousands of randomized, valid edge cases to ensure the parser and planner hold true across all acceptable states.
* **Continuous Fuzzing (`cargo-fuzz`):** The ingestion pipeline has survived **3.1+ million** continuous mutations, validating **1,126 unique execution edges** with **0 panics** at throughputs of up to 15,000 executions per second.
* **State Simulation:** The verifier is tested against "Parallel Universes" of system states (Clean, Done, and Dirty) to ensure idempotency.

## Architectural Safeguards

* **Pass 1 (Idempotency):** No actions are generated if the system already matches your intent.
* **Pass 2 (Feasibility):** Hardware-level checks prevent formatting disks that are currently mounted or referenced in your boot sequence.
* **Safe `fstab` Management:** Updates use a **Temp → Sync → Atomic Rename** pattern. Entries are dynamically resolved to `UUIDs` via `blkid` to guarantee persistent mounting across reboots.

## Design Constraints

* **Single Static Binary:** Written in Rust; no runtime dependencies beyond LVM2.
* **No Daemon:** Zero background processes, distributed locking, or hidden retries.
* **Air-Gapped Ready:** Designed for secure, isolated production environments.
* **Root Enforced:** Immediately verifies UID 0 before parsing begins.
* **Explicit Behavior:** No "hidden" automation; behavior is transparent and predictable.

## Roadmap to v1.0

| Phase | Focus Area | Target Version |
| --- | --- | --- |
| **Phase 1** | **`provision`** | **v0.1.0 (Current)** |
| Phase 2 | `decommission` & Reflexive VM E2E | v0.2.0 |
| Phase 3 | `shrink` | v0.3.0 |
| Phase 4 | `evacuate` | v0.4.0 |
| Phase 5 | `replace-disk` | v0.5.0 |
| Phase 6 | `shrink-xfs` | v0.6.0 |
| Phase 7 | `accelerate` | v0.7.0 |
| Phase 8 | `snap-back` | v0.8.0 |
| Phase 9 | CLI Automation, `repair`, & `continue` | v0.9.0 |
| Phase 10 | **Full Operational Suite (`v1.0`)** | **1.0.0** |
| *Next* | *Ansible Collection & Kubernetes CSI Driver* | *Post-v1.0* |

## Documentation Deep Dives

* **[Architecture & Design](docs/architecture.md):** The internal State-Convergence engine logic.
* **[Testing & Verification](docs/testing.md):** Deep dive into formal proofs and fuzzing.
* **[Full Roadmap](docs/roadmap.md):** Our journey from v0.1 to v1.0.
* **[Development Logs](devlogs/):** The history of every major architectural decision.

### Disclaimer

**Root privileges are required.** `lvq` manages raw block devices. While we employ extreme defensive programming, you should always have a verified backup of your data. *`lvq` is provided "as is", without warranty of any kind.*

