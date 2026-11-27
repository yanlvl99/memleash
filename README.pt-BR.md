# memleash

[English](https://github.com/yanlvl99/memleash/blob/main/README.md)

Controle limites de [working set de processos](https://learn.microsoft.com/pt-br/windows/win32/procthread/process-working-set) no Windows usando Rust.

O `memleash` encontra os processos de um aplicativo em execução, classifica cada filho pelas flags da linha de comando ou por sessões de áudio [WASAPI](https://learn.microsoft.com/pt-br/windows/win32/coreaudio/wasapi) ativas, identifica o estado da janela (primeiro plano, segundo plano, minimizada ou na bandeja) e aplica, esvazia ou libera limites de *working set* validando o timestamp de criação contra reutilização de PID.

> Apenas Windows 10/11. Requer Rust com Edition 2024.

## Uso

Adicione o `memleash` via git (ainda não publicado no crates.io):

```toml
[dependencies]
memleash = { git = "https://github.com/yanlvl99/memleash" }
```

Exemplo com o `chrome.exe`: faz o scan, aplica limite + trim quando não estiver em primeiro plano e devolve aos limites *soft* padrão do Windows (50 / 345 páginas):

```rust
use memleash::{AppProfile, WindowState, scan};

fn main() -> memleash::Result<()> {
    let profile = AppProfile::chromium("chrome.exe");
    let snap = scan(&profile)?;

    if snap.window_state != WindowState::Foreground {
        let report = snap.cap_all(true);
        println!(
            "capped={}, skipped={}, failed={}",
            report.succeeded,
            report.skipped,
            report.failures.len()
        );

        // Restaura os limites soft padrão do Windows
        snap.release_all();
    }

    Ok(())
}
```

Para apps fora do padrão Chromium, monte um perfil próprio com `AppProfile::builder("app.exe")` (`MatchRule`, `SubLabelRule`, `RoleConfig`) ou use `AppProfile::single("app.exe", config)` para apps de processo único.

## Crates do workspace

| Crate | O que faz |
| --- | --- |
| `memleash` | Re-exporta `memleash-win32` e `memleash-profiles`; adiciona `scan()`, `AppSnapshot` e ações em lote (`cap_all`, `empty_all`, `release_all`). |
| `memleash-win32` | Chamadas Win32/WDK/COM: `cap`, `empty_working_set`, `release`, `inspect`, `command_line`, `find_by_name`, `window_state`, `active_audio_pids`, `system_memory`, `MemoryWatcher`. |
| `memleash-profiles` | Classificação por role, sub-identidades por flag (`Renderer:#6`, `Utility:network`) e presets de `AppProfile`. Sem dependência de Win32. |

## Exemplos

```powershell
cargo run --example inspect
cargo run --example inspect -- chrome.exe
cargo run --example cap_cycle -- chrome.exe
```

## Licença

Licenciado sob [GPL-3.0-or-later](https://github.com/yanlvl99/memleash/blob/main/LICENSE).

---

Feito com amor e carinho ♥
