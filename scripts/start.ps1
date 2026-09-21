$ErrorActionPreference = 'Stop'
$env:Path = "C:\msys64\ucrt64\bin;$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not (Get-Command dx -ErrorAction SilentlyContinue)) {
    cargo install dioxus-cli --version 0.7.10 --locked
}
dx serve --web --features server
