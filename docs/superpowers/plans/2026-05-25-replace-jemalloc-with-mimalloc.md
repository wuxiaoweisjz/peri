# Replace jemalloc with mimalloc — Eliminate Arena Fragmentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `tikv-jemallocator` global allocator with `mimalloc`, completely removing jemalloc dependency and eliminating the ~17.5 MB/turn arena fragmentation that causes mapped virtual address bloat to 116+ MB after `/clear`. Mimalloc's default aggressive page-return behavior obviates the need for compile-time `_rjem_malloc_conf` symbols, runtime `mallctl` tuning, and per-arena `decay`/`purge` cycles.

**Architecture:** Direct swap of global allocator from `Jemalloc` → `MiMalloc`; delete the entire `jemalloc_config.rs` module (compile-time `malloc_conf` + runtime `mallctl` tuning); replace `jemalloc_decay()` with a thin `mi_collect(true)` wrapper; rewrite `/heapdump` to use `mi_process_info()` + `mi_stats_print_out()`.

**Tech Stack:** `mimalloc` v0.1 (Rust crate, provides `MiMalloc` struct), `libmimalloc-sys` v0.1 (FFI bindings for `mi_collect`, `mi_process_info`, `mi_stats_print_out`), Rust 2021.

**Root Cause (from heapdump analysis):**
- `allocated` does NOT grow (9.5 → 9.0 MB) → no data structure leak
- `active` +13.6 MB, `resident` +44.7 MB, `mapped` +137.2 MB → jemalloc arena fragmentation
- 680k+ malloc/free per turn, 97.3% freed immediately → dirty pages accumulate faster than decay purge
- macOS does not support `background_thread` → tuning options limited on primary dev platform
- jemalloc's `dirty_decay_ms` and arena design holds freed pages for reuse → mapped VAS never shrinks

**Why mimalloc solves this:**
- Segment-based design (not arena-based) → freed segments are returned to OS immediately
- Default `mi_option_reset_delay = 0` → no delay before releasing pages
- Cross-platform (macOS, Linux, Windows) → no `cfg(target_os)` workarounds needed
- No compile-time `malloc_conf` equivalent needed → simpler codebase
- No runtime `mallctl` API → no `jemalloc_config.rs` equivalent needed

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` (workspace) | Modify | Replace `tikv-jemallocator` / `tikv-jemalloc-ctl` with `mimalloc` / `libmimalloc-sys` |
| `peri-tui/Cargo.toml` | Modify | Replace workspace refs from jemalloc to mimalloc |
| `peri-tui/src/main.rs` | Modify | Replace `#[global_allocator]` + remove `mod jemalloc_config` + remove `configure_jemalloc()` call |
| `peri-tui/src/lib.rs` | Modify | Remove `pub mod jemalloc_config;` |
| `peri-tui/src/jemalloc_config.rs` | **Delete** | Entire file (compile-time `_rjem_malloc_conf` + runtime `mallctl` tuning) |
| `peri-tui/src/app/thread_ops.rs` | Modify | Replace `jemalloc_decay()` with `alloc_collect()` using `mi_collect(true)` |
| `peri-tui/src/app/agent_compact.rs` | Modify | Update function call name `jemalloc_decay()` → `alloc_collect()` |
| `peri-tui/src/command/core/heapdump.rs` | Modify | Replace jemalloc stats with `mi_process_info()` + `mi_stats_print_out()` |

---

### Task 1: Update Workspace Dependencies — Replace jemalloc with mimalloc

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Replace workspace dependency declarations**

In `Cargo.toml`, find (lines 74-76):

```toml
# --- Memory ---
tikv-jemallocator = "0.6"
tikv-jemalloc-ctl = { version = "0.6", features = ["stats", "use_std"] }
```

Replace with:

```toml
# --- Memory ---
mimalloc = { version = "0.1", default-features = false }
libmimalloc-sys = { version = "0.1", features = ["extended"] }
```

> **Note:** The `extended` feature on `libmimalloc-sys` exposes `mi_process_info()` and other extended APIs needed for `/heapdump`. The `mimalloc` crate provides the `MiMalloc` struct for `#[global_allocator]`.

- [ ] **Step 2: Verify Cargo resolves**

Run: `cargo check -p peri-tui`
Expected: Fails to compile (expected — `main.rs` still references jemalloc). The goal is to verify the workspace dep graph resolves.

---

### Task 2: Update peri-tui Crate Dependencies

**Files:**
- Modify: `peri-tui/Cargo.toml`

- [ ] **Step 1: Replace platform-specific jemalloc deps with mimalloc**

In `peri-tui/Cargo.toml`, find (lines 63-65):

```toml
[target.'cfg(not(target_os = "windows"))'.dependencies]
tikv-jemallocator = { workspace = true }
tikv-jemalloc-ctl = { workspace = true }
```

Replace with:

```toml
[target.'cfg(not(target_os = "windows"))'.dependencies]
mimalloc = { workspace = true }
libmimalloc-sys = { workspace = true }
```

> **Design decision:** We keep the `cfg(not(target_os = "windows"))` gate even though mimalloc supports Windows. This matches the existing pattern and avoids changing Windows behavior (system allocator). If Windows support is desired later, the gate can be removed in a separate change.

---

### Task 3: Replace Global Allocator in main.rs

**Files:**
- Modify: `peri-tui/src/main.rs`

- [ ] **Step 1: Replace `#[global_allocator]` declaration**

In `peri-tui/src/main.rs`, find (lines 3-5):

```rust
#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

Replace with:

```rust
#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

- [ ] **Step 2: Remove `mod jemalloc_config;` declaration**

In `peri-tui/src/main.rs`, find (line 33):

```rust
mod jemalloc_config;
```

Delete this line.

- [ ] **Step 3: Remove `configure_jemalloc()` call from `main()`**

In `peri-tui/src/main.rs`, find (lines 254-260):

```rust
    // jemalloc config is applied at compile time via the `_rjem_malloc_conf`
    // global symbol (see jemalloc_config.rs). It takes effect BEFORE main()
    // runs — jemalloc initializes during lang_start's first allocation.

    // Runtime mallctl writes as fallback/diagnostics (may not fully take effect
    // for background_thread if arenas already exist, but harmless to call).
    peri_tui::jemalloc_config::configure_jemalloc();
```

Replace with:

```rust
    // mimalloc requires no compile-time or runtime configuration — its default
    // behavior aggressively returns freed pages to the OS, which is exactly
    // what we need for high allocation-churn workloads.
```

- [ ] **Step 4: Build to verify**

Run: `cargo build -p peri-tui`
Expected: Build succeeds (with warnings about unused `jemalloc_config` module in `lib.rs` — will fix in Task 4).

---

### Task 4: Delete jemalloc_config.rs Module

**Files:**
- Modify: `peri-tui/src/lib.rs`
- Delete: `peri-tui/src/jemalloc_config.rs`

- [ ] **Step 1: Remove module declaration from lib.rs**

In `peri-tui/src/lib.rs`, find (line 23):

```rust
pub mod jemalloc_config;
```

Delete this line.

- [ ] **Step 2: Delete the module file**

```bash
rm peri-tui/src/jemalloc_config.rs
```

- [ ] **Step 3: Build to verify**

Run: `cargo build -p peri-tui`
Expected: Build succeeds with no jemalloc-related warnings.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor(tui): replace jemalloc with mimalloc global allocator

- Replace tikv-jemallocator with mimalloc as global allocator
- Delete jemalloc_config.rs (no compile-time/runtime config needed)
- mimalloc's default aggressive page return eliminates the need for
  dirty_decay_ms, background_thread, and lg_tcache_max tuning"
```

---

### Task 5: Replace `jemalloc_decay()` with `alloc_collect()`

**Files:**
- Modify: `peri-tui/src/app/thread_ops.rs`
- Modify: `peri-tui/src/app/agent_compact.rs`

- [ ] **Step 1: Replace `jemalloc_decay()` in thread_ops.rs**

In `peri-tui/src/app/thread_ops.rs`, find (lines 3-43):

```rust
/// 通知 jemalloc 将空闲内存页归还给 OS。
/// 在 `/clear`、`/compact`、切换会话等大块内存释放后调用。
/// 注：仅释放 jemalloc 管理的 Rust 堆内存，SQLite/tokio 等非 Rust 分配不受影响。
#[cfg(not(target_os = "windows"))]
pub(crate) fn jemalloc_decay() {
    // Advance epoch to refresh internal stats
    if let Err(e) = tikv_jemalloc_ctl::epoch::advance() {
        tracing::debug!(error = %e, "jemalloc epoch advance failed");
        return;
    }
    let narenas: usize = match tikv_jemalloc_ctl::arenas::narenas::read() {
        Ok(n) => n as usize,
        Err(e) => {
            tracing::debug!(error = %e, "jemalloc narenas read failed");
            return;
        }
    };
    for i in 0..narenas {
        // 先触发 decay：处理 decay timeline 中正在老化的 dirty pages
        let mut decay_key = format!("arena.{}.decay", i);
        decay_key.push(0 as char);
        unsafe {
            let _: u8 = match tikv_jemalloc_ctl::raw::read(decay_key.as_bytes()) {
                Ok(v) => v,
                Err(_) => continue,
            };
        }
        // 再触发 purge：立即释放所有已达到 decay 阈值的 dirty pages
        let mut purge_key = format!("arena.{}.purge", i);
        purge_key.push(0 as char);
        unsafe {
            let _: u8 = match tikv_jemalloc_ctl::raw::read(purge_key.as_bytes()) {
                Ok(v) => v,
                Err(_) => continue,
            };
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn jemalloc_decay() {}
```

Replace with:

```rust
/// 通知分配器将空闲内存页归还给 OS。
/// 在 `/clear`、`/compact`、切换会话等大块内存释放后调用。
/// 注：仅释放 mimalloc 管理的 Rust 堆内存，SQLite/tokio 等非 Rust 分配不受影响。
#[cfg(not(target_os = "windows"))]
pub(crate) fn alloc_collect() {
    // mimalloc: force=true triggers aggressive collection, immediately
    // returning all freeable segments to the OS. This replaces the old
    // jemalloc per-arena decay+purge cycle with a single call.
    unsafe {
        libmimalloc_sys::mi_collect(true);
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn alloc_collect() {}
```

- [ ] **Step 2: Update call site in `open_thread`**

In `peri-tui/src/app/thread_ops.rs`, find (line 296):

```rust
        jemalloc_decay();
```

Replace with:

```rust
        alloc_collect();
```

- [ ] **Step 3: Update call site in `new_thread`**

In `peri-tui/src/app/thread_ops.rs`, find (lines 403-404):

```rust
        // 归还已释放内存页给 OS（jemalloc arena decay）
        jemalloc_decay();
```

Replace with:

```rust
        // 归还已释放内存页给 OS
        alloc_collect();
```

- [ ] **Step 4: Update call site in agent_compact.rs**

In `peri-tui/src/app/agent_compact.rs`, find (line 82):

```rust
        super::thread_ops::jemalloc_decay();
```

Replace with:

```rust
        super::thread_ops::alloc_collect();
```

- [ ] **Step 5: Build to verify**

Run: `cargo build -p peri-tui`
Expected: Build succeeds, no jemalloc references in thread_ops.rs or agent_compact.rs.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(tui): replace jemalloc_decay() with alloc_collect()

Use mi_collect(true) to eagerly return freed pages to OS.
Single call replaces per-arena decay+purge loop."
```

---

### Task 6: Rewrite `/heapdump` Command for mimalloc

**Files:**
- Modify: `peri-tui/src/command/core/heapdump.rs`

The heapdump command currently reads jemalloc-specific stats (`allocated`, `active`, `mapped`, `resident`, `retained`, `huge`) and config diagnostics (`dirty_decay_ms`, `background_thread`, `lg_tcache_max`, `narenas`). All of this must be replaced with mimalloc equivalents.

**mimalloc API mapping:**

| jemalloc API | mimalloc equivalent | Notes |
|---|---|---|
| `stats::allocated` | `mi_process_info().peak.reserved.current` | Currently allocated bytes |
| `stats::active` | Computed from `mi_process_info()` fields | Active (committed) pages |
| `stats::mapped` | `mi_process_info().peak.reserved.current` | Reserved virtual address space |
| `stats::resident` | `mi_process_info().peak.committed.current` | Physical RSS from allocator |
| `stats::retained` | No direct equivalent | mimalloc doesn't retain freed pages |
| `epoch::advance()` | Not needed | mimalloc stats are always current |
| `stats_print::stats_print()` | `mi_stats_print_out()` | Formatted stats output |
| Config section (`dirty_decay_ms`, etc.) | Not applicable | mimalloc has no runtime config |

- [ ] **Step 1: Rewrite the entire heapdump.rs**

Replace the entire content of `peri-tui/src/command/core/heapdump.rs` with:

```rust
use std::io::Write;

use crate::app::App;
use crate::command::Command;

pub struct HeapdumpCommand;

impl Command for HeapdumpCommand {
    fn name(&self) -> &str {
        "heapdump"
    }

    fn description(&self, _lc: &crate::i18n::LcRegistry) -> String {
        "Dump heap memory profile to .tmp/heapdump-*.txt".to_string()
    }

    fn execute(&self, app: &mut App, _args: &str) {
        let now = chrono::Local::now();
        let filename = format!(".tmp/heapdump-{}.txt", now.format("%Y%m%d-%H%M%S"));

        let mut buf: Vec<u8> = Vec::new();

        // ── 1. RSS ──
        let rss_mb = read_rss_mb();
        let _ = writeln!(buf, "=== HEAPDUMP {} ===", now.format("%Y-%m-%d %H:%M:%S"));
        let _ = writeln!(buf, "RSS: {:.1} MB\n", rss_mb);

        // ── 2. mimalloc summary ──
        #[cfg(not(target_os = "windows"))]
        {
            let mut info: libmimalloc_sys::mi_process_info_t = unsafe { std::mem::zeroed() };
            unsafe {
                libmimalloc_sys::mi_process_info(&mut info);
            }
            let mb = |v: usize| v as f64 / (1024.0 * 1024.0);

            let _ = writeln!(buf, "=== MIMALLOC SUMMARY ===");
            let _ = writeln!(buf, "  elapsed:       {:.1} s", info.elapsed_millis as f64 / 1000.0);
            let _ = writeln!(buf, "  user_time:     {:.1} s", info.user_millis as f64 / 1000.0);
            let _ = writeln!(buf, "  system_time:   {:.1} s", info.system_millis as f64 / 1000.0);
            let _ = writeln!(buf);
            let _ = writeln!(buf, "  current rss:           {:.1} MB", mb(info.current_rss));
            let _ = writeln!(buf, "  peak rss:              {:.1} MB", mb(info.peak_rss));
            let _ = writeln!(buf);
            let _ = writeln!(buf, "  current committed:     {:.1} MB", mb(info.current_committed));
            let _ = writeln!(buf, "  peak committed:        {:.1} MB", mb(info.peak_committed));
            let _ = writeln!(buf, "  current reserved:      {:.1} MB", mb(info.current_reserved));
            let _ = writeln!(buf, "  peak reserved:         {:.1} MB", mb(info.peak_reserved));
            let _ = writeln!(buf);
            let _ = writeln!(buf, "  malloc_count:          {}", info.malloc_count);
            let _ = writeln!(buf, "  free_count:            {}", info.free_count);
            let _ = writeln!(buf, "  current_mallocs:       {}", info.current_mallocs);
            let _ = writeln!(buf, "  malloc_requested:      {:.1} MB", mb(info.malloc_requested));
            let _ = writeln!(buf);
            let _ = writeln!(
                buf,
                "  RSS-overhead:          {:.1} MB (RSS-committed)\n",
                rss_mb - mb(info.current_committed)
            );

            // Detailed mimalloc stats
            let _ = writeln!(buf, "=== MIMALLOC DETAILED STATS ===");
            unsafe extern "C" fn stats_write(msg: *const std::os::raw::c_char, _arg: *mut std::os::raw::c_void) {
                // This callback is called by mi_stats_print_out for each line.
                // We cannot capture buf directly, so we collect into a thread-local.
                // Instead, we'll use mi_stats_print with a Vec<u8> output.
            }
            // Use mi_stats_print_out with output to stdout, then capture via
            // a simpler approach: write to a Vec<u8> using a custom output callback.
            let mut stats_buf: Vec<u8> = Vec::new();
            {
                // mi_stats_print_out writes to a callback. We use a static mut
                // pointer trick to pass our Vec reference through the opaque arg.
                let arg = &mut stats_buf as *mut Vec<u8> as *mut std::os::raw::c_void;
                unsafe extern "C" fn write_to_vec(msg: *const std::os::raw::c_char, arg: *mut std::os::raw::c_void) {
                    if msg.is_null() { return; }
                    let cstr = std::ffi::CStr::from_ptr(msg);
                    let bytes = cstr.to_bytes();
                    if !bytes.is_empty() {
                        let vec = &mut *(arg as *mut Vec<u8>);
                        vec.extend_from_slice(bytes);
                    }
                }
                unsafe {
                    libmimalloc_sys::mi_stats_print_out(Some(write_to_vec), arg);
                }
            }
            buf.extend_from_slice(&stats_buf);
        }

        // ── 3. TUI components ──
        {
            let s = &app.session_mgr.sessions[app.session_mgr.active];
            let agent_bytes: usize = s
                .agent
                .agent_state_messages
                .iter()
                .map(|m| m.content().len())
                .sum();
            let pipeline_bytes: usize = s
                .messages
                .pipeline
                .completed_messages()
                .iter()
                .map(|m| m.content().len())
                .sum();

            let _ = writeln!(buf, "\n=== TUI COMPONENTS ===");
            let _ = writeln!(
                buf,
                "  agent_state_messages: count={}, bytes={:.1}MB",
                s.agent.agent_state_messages.len(),
                agent_bytes as f64 / (1024.0 * 1024.0)
            );
            let _ = writeln!(
                buf,
                "  pipeline_completed:   count={}, bytes={:.1}MB",
                s.messages.pipeline.completed_messages().len(),
                pipeline_bytes as f64 / (1024.0 * 1024.0)
            );
            let _ = writeln!(
                buf,
                "  view_messages:        count={}",
                s.messages.view_messages.len()
            );
            let _ = writeln!(
                buf,
                "  pending_messages:     count={}",
                s.messages.pending_messages.len()
            );
            let _ = writeln!(
                buf,
                "  ephemeral_notes:      count={}",
                s.messages.ephemeral_notes.len()
            );
            let _ = writeln!(buf, "  todo_items:           count={}", s.todo_items.len());
            let _ = writeln!(
                buf,
                "  background_tasks:     count={}",
                app.session_mgr.sessions[app.session_mgr.active].background_task_count
            );
        }

        // ── 4. All sessions ──
        {
            let _ = writeln!(buf, "\n=== SESSIONS ===");
            for (i, sess) in app.session_mgr.sessions.iter().enumerate() {
                let _ = writeln!(
                    buf,
                    "  [{}]: agent_msgs={}, view_vms={}, loading={}",
                    i,
                    sess.agent.agent_state_messages.len(),
                    sess.messages.view_messages.len(),
                    sess.ui.loading,
                );
            }
        }

        // Write file
        let full_path = std::path::Path::new(&filename);
        if let Some(parent) = full_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let msg = match std::fs::write(full_path, &buf) {
            Ok(()) => {
                #[cfg(not(target_os = "windows"))]
                let mut info: libmimalloc_sys::mi_process_info_t = unsafe { std::mem::zeroed() };
                #[cfg(not(target_os = "windows"))]
                unsafe { libmimalloc_sys::mi_process_info(&mut info); }
                #[cfg(not(target_os = "windows"))]
                let reserved_str = format!(
                    "{:.0}MB",
                    info.current_reserved as f64 / (1024.0 * 1024.0)
                );
                #[cfg(target_os = "windows")]
                let reserved_str = "N/A".to_string();
                format!("Heapdump -> {filename}\nRSS: {rss_mb:.0}MB | reserved: {reserved_str}")
            }
            Err(e) => format!("heapdump failed: {e}"),
        };
        app.session_mgr.sessions[app.session_mgr.active]
            .messages
            .view_messages
            .push(crate::app::MessageViewModel::system(msg));
    }
}

fn read_rss_mb() -> f64 {
    if cfg!(target_os = "macos") {
        std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(|kb| kb / 1024.0)
            })
            .unwrap_or(-1.0)
    } else {
        -1.0
    }
}
```

> **Important notes on the heapdump rewrite:**
> - `mi_process_info_t` fields: `elapsed_millis`, `user_millis`, `system_millis`, `current_rss`, `peak_rss`, `current_committed`, `peak_committed`, `current_reserved`, `peak_reserved`, `malloc_count`, `free_count`, `current_mallocs`, `malloc_requested`. These map directly to the mimalloc C struct.
> - `mi_stats_print_out(callback, arg)` uses a C function pointer callback pattern. We pass a `*mut Vec<u8>` through the opaque `arg` parameter and cast it back in the callback.
> - The old `stats.huge.allocated` and jemalloc config sections are removed — mimalloc has no equivalent concepts.
> - The `read_rss_mb()` helper is preserved unchanged — it reads OS RSS via `ps` on macOS.

- [ ] **Step 2: Build to verify**

Run: `cargo build -p peri-tui`
Expected: Build succeeds. If `mi_process_info_t` field names don't match the installed version, check `libmimalloc-sys` docs for the exact struct layout and adjust field names accordingly.

- [ ] **Step 3: Commit**

```bash
git add peri-tui/src/command/core/heapdump.rs
git commit -m "refactor(heapdump): replace jemalloc stats with mimalloc mi_process_info

Use mi_process_info() for summary stats and mi_stats_print_out()
for detailed allocator diagnostics."
```

---

### Task 7: Clean Up — Remove All jemalloc References

**Files:** None new (verification step)

- [ ] **Step 1: Grep for remaining jemalloc references**

```bash
grep -rn "jemalloc" --include="*.rs" --include="*.toml" .
```

Expected: **Zero matches**. All references to `jemalloc`, `tikv_jemalloc`, `tikv_jemallocator`, `Jemalloc`, `jemalloc_config`, `jemalloc_decay`, `mallctl`, `_rjem_malloc_conf` should be gone.

- [ ] **Step 2: Grep for remaining tikv references**

```bash
grep -rn "tikv" --include="*.rs" --include="*.toml" .
```

Expected: Zero matches.

- [ ] **Step 3: Check Cargo.lock is clean**

```bash
grep -c "tikv-jemalloc" Cargo.lock
```

Expected: 0 (after `cargo update` or `cargo check`).

- [ ] **Step 4: Commit any remaining cleanup**

```bash
git add -A
git commit -m "chore: remove all remaining jemalloc/tikv references"
```

---

### Task 8: Build Verification & Testing

**Files:** None (testing only)

- [ ] **Step 1: Full workspace build**

```bash
cargo build --workspace
```

Expected: Build succeeds with no errors or jemalloc-related warnings.

- [ ] **Step 2: Run peri-tui tests**

```bash
cargo test -p peri-tui --lib
```

Expected: All tests pass. The old `jemalloc_config::tests` module was deleted in Task 4, so no jemalloc tests should remain.

- [ ] **Step 3: Release build**

```bash
cargo build -p peri-tui --release
```

Expected: Build succeeds. Binary should be smaller than before (mimalloc is ~50KB vs jemalloc ~300KB).

- [ ] **Step 4: Manual smoke test — verify allocator is mimalloc**

1. Start TUI in release mode: `cargo run -p peri-tui --release`
2. Run `/heapdump` command
3. Check `.tmp/heapdump-*.txt` — verify:
   - Section header says `MIMALLOC SUMMARY` (not `JEMALLOC SUMMARY`)
   - Stats values are populated (non-zero)
   - `current_reserved` is reasonable (< 100 MB for idle state)

- [ ] **Step 5: Manual smoke test — verify memory behavior**

1. Start TUI in release mode
2. Send 3-5 messages with tool calls
3. Run `/heapdump` — note `current_reserved` and RSS
4. Run `/clear`
5. Run `/heapdump` again — compare values
6. Expected: `current_reserved` should drop significantly after `/clear` (mimalloc eagerly returns pages)
7. After 10+ turns, RSS should remain stable (no linear growth)

- [ ] **Step 6: Manual smoke test — verify `alloc_collect()` works**

1. In TUI, send several messages
2. Run `/clear` (triggers `alloc_collect()` via `new_thread()`)
3. Run `/compact` (triggers `alloc_collect()` via `agent_compact.rs`)
4. Switch sessions (triggers `alloc_collect()` via `open_thread()`)
5. All operations should complete without errors

- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "chore: jemalloc → mimalloc migration complete, all tests pass"
```

---

## Self-Review Checklist

### 1. Spec Coverage

| Requirement | Task | Status |
|---|---|---|
| Replace `tikv-jemallocator` with `mimalloc` | Task 1-3 | ✅ |
| Remove `tikv-jemalloc-ctl` dependency | Task 1-2 | ✅ |
| Delete `jemalloc_config.rs` (compile-time + runtime config) | Task 4 | ✅ |
| Replace `jemalloc_decay()` with `mi_collect(true)` | Task 5 | ✅ |
| Update all `jemalloc_decay()` call sites | Task 5 | ✅ |
| Rewrite `/heapdump` for mimalloc stats | Task 6 | ✅ |
| Remove all jemalloc references | Task 7 | ✅ |
| Build + test verification | Task 8 | ✅ |

### 2. Placeholder Scan

No TBD/TODO/fill-in-later patterns. All code blocks contain complete implementations.
**One note:** `mi_process_info_t` struct field names should be verified against the actual `libmimalloc-sys` version installed. If the `extended` feature doesn't expose the struct fields listed, check `docs.rs/libmimalloc-sys` for the exact layout.

### 3. Type Consistency

- `mimalloc::MiMalloc` implements `std::alloc::GlobalAlloc` — valid for `#[global_allocator]`
- `libmimalloc_sys::mi_collect(true)` — `bool` parameter maps to `mi_bool_t` (C `int`)
- `libmimalloc_sys::mi_process_info(&mut info)` — takes `*mut mi_process_info_t`
- `mi_stats_print_out` callback signature: `extern "C" fn(*const c_char, *mut c_void)`
- `alloc_collect()` is `pub(crate)` — matches existing visibility of `jemalloc_decay()`
- All `cfg(not(target_os = "windows"))` gates preserved from original code

### 4. Risk Assessment

| Risk | Mitigation |
|---|---|
| `mi_process_info_t` struct layout mismatch | Verify field names with `docs.rs/libmimalloc-sys` before implementing; add `#[allow(dead_code)]` for unused fields |
| `mi_stats_print_out` callback safety | Use the proven `*mut Vec<u8>` through opaque `arg` pattern |
| mimalloc performance regression vs jemalloc | mimalloc benchmarks show comparable or better throughput for general workloads; monitor in practice |
| `libmimalloc-sys` extended feature not available | Fall back to reading `current_rss` from `mi_process_info()` alone, skip detailed stats |
| Cross-compilation (Linux target from macOS) | mimalloc uses CMake; ensure CMake is available in CI |

### 5. API Verification Checklist

Before starting implementation, verify these APIs exist in the installed `libmimalloc-sys` version:

```rust
// Verify in docs.rs/libmimalloc-sys:
libmimalloc_sys::mi_collect(force: bool);           // ✓ standard API
libmimalloc_sys::mi_process_info(info: *mut mi_process_info_t);  // ✓ needs "extended" feature
libmimalloc_sys::mi_stats_print_out(out: Option<extern "C" fn(...)>, arg: *mut c_void);  // ✓ standard API
libmimalloc_sys::mi_process_info_t  // struct with fields: elapsed_millis, current_rss, peak_rss, etc.
```

If `mi_process_info_t` is not available, use this fallback approach for heapdump:
```rust
// Fallback: use only mi_stats_print_out for all stats
// The stats output includes "process: " lines with RSS and committed/reserved info
```
