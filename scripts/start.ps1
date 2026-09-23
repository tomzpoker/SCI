$ErrorActionPreference = 'Stop'
Remove-Item Env:RUSTFLAGS,Env:RUSTUP_TOOLCHAIN,Env:CARGO_BUILD_TARGET -ErrorAction SilentlyContinue
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) { throw 'rustup introuvable.' }
if (-not (rustup toolchain list | Select-String -SimpleMatch '1.98.1-x86_64-pc-windows-msvc')) { rustup toolchain install 1.98.1-x86_64-pc-windows-msvc --profile default --target wasm32-unknown-unknown }
if (-not (Get-Command dx -ErrorAction SilentlyContinue) -or ((dx --version) -notmatch 'dioxus 0\.7\.10')) { cargo install dioxus-cli --version 0.7.10 --locked --force }
dx serve --web --features server
