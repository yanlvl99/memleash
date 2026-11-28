# memleash

[Português (BR)](https://github.com/yanlvl99/memleash/blob/main/README.pt-BR.md) · [API Docs](https://yanlvl99.github.io/memleash/)

Control Windows [process working-set](https://learn.microsoft.com/en-us/windows/win32/procthread/process-working-set) limits from Rust.

`memleash` finds a running app's processes, classifies them by command-line flags or active [WASAPI](https://learn.microsoft.com/en-us/windows/win32/coreaudio/wasapi) audio sessions, checks window state (foreground, background, minimized, tray-hidden), and caps, trims, or releases their working sets after verifying each process's creation time against PID reuse.

> Windows 10/11 only. Requires Rust Edition 2024.

## Usage

Add `memleash` from git (not published to crates.io yet):

```toml
[dependencies]
memleash = { git = "https://github.com/yanlvl99/memleash" }
```

Cap `Chrome.exe` while it is not in the foreground, trim its working set, then release it back to Windows default soft limits (50 / 345 pages per [`SetProcessWorkingSetSizeEx`](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-setprocessworkingsetsizeex)):

```rust
use memleash::{AppProfile, WindowState, scan};

fn main() -> memleash::Result<()> {
    let profile = AppProfile::chromium("Chrome.exe");
    let snap = scan(&profile)?;

    if snap.window_state != WindowState::Foreground {
        let report = snap.cap_all(true);
        println!(
            "capped={}, skipped={}, failed={}",
            report.succeeded,
            report.skipped,
            report.failures.len()
        );

        // Restore Windows default soft limits
        let released = snap.release_all();
        println!(
            "released={}, failed={}",
            released.succeeded,
            released.failures.len()
        );
    }

    Ok(())
}
```

For non-Chromium apps, build a custom profile with [`AppProfile::builder`](https://yanlvl99.github.io/memleash/memleash/struct.AppProfile.html#method.builder) ([`MatchRule`](https://yanlvl99.github.io/memleash/memleash/enum.MatchRule.html), [`SubLabelRule`](https://yanlvl99.github.io/memleash/memleash/enum.SubLabelRule.html), [`RoleConfig`](https://yanlvl99.github.io/memleash/memleash/struct.RoleConfig.html)) or use [`AppProfile::single`](https://yanlvl99.github.io/memleash/memleash/struct.AppProfile.html#method.single) for single-process apps.

## Workspace crates

| Crate | What it does |
| --- | --- |
| [`memleash`](https://yanlvl99.github.io/memleash/memleash/index.html) | Re-exports `memleash-win32` and `memleash-profiles`; adds [`scan()`](https://yanlvl99.github.io/memleash/memleash/fn.scan.html), [`AppSnapshot`](https://yanlvl99.github.io/memleash/memleash/struct.AppSnapshot.html), and batch actions (`cap_all`, `empty_all`, `release_all`). |
| [`memleash-win32`](https://yanlvl99.github.io/memleash/memleash_win32/index.html) | Win32/WDK/COM calls: `cap`, `empty_working_set`, `release`, `inspect`, `command_line`, `find_by_name`, `window_state`, `active_audio_pids`, `system_memory`, [`MemoryWatcher`](https://yanlvl99.github.io/memleash/memleash/struct.MemoryWatcher.html). |
| [`memleash-profiles`](https://yanlvl99.github.io/memleash/memleash_profiles/index.html) | Role matching, flag-based sub-identities (`Renderer:#6`, `Utility:network`), and [`AppProfile`](https://yanlvl99.github.io/memleash/memleash/struct.AppProfile.html) presets. No Win32 dependency. |

## Examples

```powershell
cargo run --example inspect
cargo run --example inspect -- Chrome.exe
cargo run --example cap_cycle -- Chrome.exe
```

## License

Licensed under [GPL-3.0-or-later](https://github.com/yanlvl99/memleash/blob/main/LICENSE).

---

Made with love and care ♥
