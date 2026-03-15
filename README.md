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

1. **Ingesting** live state via `probes` defined in the `verifier` module.
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

### Advanced Example

`lvq` supports, in theory, infinite scalling for such scenarios, perfectly allowing you to write a massive `provision` like: 

```bash
sudo lvq provision \
  --pv /dev/loop0:/dev/loop1 \
  --pv /dev/loop2 \
  --pv /dev/loop3:/dev/loop4 \
  --vg tank_vg \
  --lv web_root:1G:xfs:/var/www/html \
  --lv db_data:2G:ext4:/var/lib/mysql \
  --lv app_logs:500M:xfs:/var/log/app \
  --lv redis_cache:500M:ext4:/var/lib/redis \
  --lv user_uploads:1G:btrfs:/srv/uploads \
  --lv backup_staged:1G:xfs:/mnt/backups \
  --lv scratch_pad:200M:vfat:/mnt/scratch \
  --lv media_assets:1G:ext4:/var/www/media \
  --lv docker_volumes:1.5G:xfs:/var/lib/docker \
  --lv swap_space:1G:swap
```
This generates a massive 77 step plan, fully showcasing the power of `lvq`. It allows the user to review every single step before execution: 

```bash
--- WARNINGS ---
Targeting full disk "/dev/loop0" (not a partition)...
Targeting full disk "/dev/loop1" (not a partition)...
Targeting full disk "/dev/loop2" (not a partition)...
Targeting full disk "/dev/loop3" (not a partition)...
Targeting full disk "/dev/loop4" (not a partition)...

--- PENDING SYSTEM CHANGES ---
 1. cp -p /etc/fstab /etc/fstab.bak
 2. pvcreate -y "/dev/loop0"
 3. pvcreate -y "/dev/loop1"
 4. pvcreate -y "/dev/loop2"
 5. pvcreate -y "/dev/loop3"
 6. pvcreate -y "/dev/loop4"
 7. vgcreate -s 4194304B tank_vg "/dev/loop0" "/dev/loop1" "/dev/loop2" "/dev/loop3" "/dev/loop4"
 8. lvcreate -y -n web_root -L 1073741824B tank_vg
 9. mkfs -t xfs "/dev/tank_vg/web_root"
10. mkdir -p "/var/www/html"
11. mount "/dev/tank_vg/web_root" "/var/www/html"
12. cp -p /etc/fstab /etc/fstab.xfs.tmp
13. ID=$(blkid -s UUID -o value /dev/tank_vg/web_root); if [ -z "$ID" ]; then ID=/dev/tank_vg/web_root; else ID="UUID=$ID"; fi; echo "$ID /var/www/html xfs defaults 0 2" >> /etc/fstab.xfs.tmp
14. mv /etc/fstab.xfs.tmp /etc/fstab
15. lvcreate -y -n db_data -L 2147483648B tank_vg
16. mkfs -t ext4 "/dev/tank_vg/db_data"
17. mkdir -p "/var/lib/mysql"
18. mount "/dev/tank_vg/db_data" "/var/lib/mysql"
19. cp -p /etc/fstab /etc/fstab.ext4.tmp
20. ID=$(blkid -s UUID -o value /dev/tank_vg/db_data); if [ -z "$ID" ]; then ID=/dev/tank_vg/db_data; else ID="UUID=$ID"; fi; echo "$ID /var/lib/mysql ext4 defaults 0 2" >> /etc/fstab.ext4.tmp
21. mv /etc/fstab.ext4.tmp /etc/fstab
22. lvcreate -y -n app_logs -L 524288000B tank_vg
23. mkfs -t xfs "/dev/tank_vg/app_logs"
24. mkdir -p "/var/log/app"
25. mount "/dev/tank_vg/app_logs" "/var/log/app"
26. cp -p /etc/fstab /etc/fstab.xfs.tmp
27. ID=$(blkid -s UUID -o value /dev/tank_vg/app_logs); if [ -z "$ID" ]; then ID=/dev/tank_vg/app_logs; else ID="UUID=$ID"; fi; echo "$ID /var/log/app xfs defaults 0 2" >> /etc/fstab.xfs.tmp
28. mv /etc/fstab.xfs.tmp /etc/fstab
29. lvcreate -y -n redis_cache -L 524288000B tank_vg
30. mkfs -t ext4 "/dev/tank_vg/redis_cache"
31. mkdir -p "/var/lib/redis"
32. mount "/dev/tank_vg/redis_cache" "/var/lib/redis"
33. cp -p /etc/fstab /etc/fstab.ext4.tmp
34. ID=$(blkid -s UUID -o value /dev/tank_vg/redis_cache); if [ -z "$ID" ]; then ID=/dev/tank_vg/redis_cache; else ID="UUID=$ID"; fi; echo "$ID /var/lib/redis ext4 defaults 0 2" >> /etc/fstab.ext4.tmp
35. mv /etc/fstab.ext4.tmp /etc/fstab
36. lvcreate -y -n user_uploads -L 1073741824B tank_vg
37. mkfs -t btrfs "/dev/tank_vg/user_uploads"
38. mkdir -p "/srv/uploads"
39. mount "/dev/tank_vg/user_uploads" "/srv/uploads"
40. cp -p /etc/fstab /etc/fstab.btrfs.tmp
41. ID=$(blkid -s UUID -o value /dev/tank_vg/user_uploads); if [ -z "$ID" ]; then ID=/dev/tank_vg/user_uploads; else ID="UUID=$ID"; fi; echo "$ID /srv/uploads btrfs defaults 0 2" >> /etc/fstab.btrfs.tmp
42. mv /etc/fstab.btrfs.tmp /etc/fstab
43. lvcreate -y -n backup_staged -L 1073741824B tank_vg
44. mkfs -t xfs "/dev/tank_vg/backup_staged"
45. mkdir -p "/mnt/backups"
46. mount "/dev/tank_vg/backup_staged" "/mnt/backups"
47. cp -p /etc/fstab /etc/fstab.xfs.tmp
48. ID=$(blkid -s UUID -o value /dev/tank_vg/backup_staged); if [ -z "$ID" ]; then ID=/dev/tank_vg/backup_staged; else ID="UUID=$ID"; fi; echo "$ID /mnt/backups xfs defaults 0 2" >> /etc/fstab.xfs.tmp
49. mv /etc/fstab.xfs.tmp /etc/fstab
50. lvcreate -y -n scratch_pad -L 209715200B tank_vg
51. mkfs -t vfat "/dev/tank_vg/scratch_pad"
52. mkdir -p "/mnt/scratch"
53. mount "/dev/tank_vg/scratch_pad" "/mnt/scratch"
54. cp -p /etc/fstab /etc/fstab.vfat.tmp
55. ID=$(blkid -s UUID -o value /dev/tank_vg/scratch_pad); if [ -z "$ID" ]; then ID=/dev/tank_vg/scratch_pad; else ID="UUID=$ID"; fi; echo "$ID /mnt/scratch vfat defaults 0 2" >> /etc/fstab.vfat.tmp
56. mv /etc/fstab.vfat.tmp /etc/fstab
57. lvcreate -y -n media_assets -L 1073741824B tank_vg
58. mkfs -t ext4 "/dev/tank_vg/media_assets"
59. mkdir -p "/var/www/media"
60. mount "/dev/tank_vg/media_assets" "/var/www/media"
61. cp -p /etc/fstab /etc/fstab.ext4.tmp
62. ID=$(blkid -s UUID -o value /dev/tank_vg/media_assets); if [ -z "$ID" ]; then ID=/dev/tank_vg/media_assets; else ID="UUID=$ID"; fi; echo "$ID /var/www/media ext4 defaults 0 2" >> /etc/fstab.ext4.tmp
63. mv /etc/fstab.ext4.tmp /etc/fstab
64. lvcreate -y -n docker_volumes -L 1073741824B tank_vg
65. mkfs -t xfs "/dev/tank_vg/docker_volumes"
66. mkdir -p "/var/lib/docker"
67. mount "/dev/tank_vg/docker_volumes" "/var/lib/docker"
68. cp -p /etc/fstab /etc/fstab.xfs.tmp
69. ID=$(blkid -s UUID -o value /dev/tank_vg/docker_volumes); if [ -z "$ID" ]; then ID=/dev/tank_vg/docker_volumes; else ID="UUID=$ID"; fi; echo "$ID /var/lib/docker xfs defaults 0 2" >> /etc/fstab.xfs.tmp
70. mv /etc/fstab.xfs.tmp /etc/fstab
71. lvcreate -y -n swap_space -L 1073741824B tank_vg
72. mkswap /dev/tank_vg/swap_space
73. swapon /dev/tank_vg/swap_space
74. cp -p /etc/fstab /etc/fstab.swap.tmp
75. ID=$(blkid -s UUID -o value /dev/tank_vg/swap_space); if [ -z "$ID" ]; then ID=/dev/tank_vg/swap_space; else ID="UUID=$ID"; fi; echo "$ID none swap sw 0 0" >> /etc/fstab.swap.tmp
76. mv /etc/fstab.swap.tmp /etc/fstab
77. systemctl daemon-reload
------------------------------

Execute these commands? [Y/n]: 
```

**What `lvq` does under the hood:**

1. **Probes** all targeted devices to build an in-memory `SystemState`.
2. **Verifies** all devices are free of existing filesystems and `fstab` collisions.
3. **Calculates** the aggregate capacity of the PV pool and verifies it can house the 518GB of requested LVs.
4. **Generates** a sequential plan: `pvcreate` → `vgcreate` → `lvcreate` → `mkfs/mkswap` → `mkdir` → `mount` → `fstab`.
5. **Prompts** for confirmation, then **Executes** and logs to `/var/log/lvq`.

**The Safety Gate:** Unless invoked with `-y`, which is only recommended for the brave, `lvq` displays the full plan and system warnings, requiring an explicit `Y` to proceed.

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

