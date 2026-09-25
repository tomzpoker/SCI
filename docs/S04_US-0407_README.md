# US-0407 — Installation

Appliquer le delta sur `D:\SCI\DEV\SCI-family-rust` issu de S04-FINAL puis exécuter :

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-us0407-s05.ps1
```

Puis migration et tests :

```powershell
cd D:\SCI\DEV\SCI-family-rust;powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\migrate.ps1
cd D:\SCI\DEV\SCI-family-rust;cargo test --lib --features server;cargo check --features server
```
