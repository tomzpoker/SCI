#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

required=(
  Cargo.toml Cargo.lock README.md CHANGELOG.md VERSION_STATE.md PROJECT_STATE.md
  ARCHITECTURE_STATE.md DATABASE_STATE.md ROADMAP_STATE.md AUDIT_EXISTANT.md DATABASE_MAPPING.md
  engines/manifest.toml scripts/bootstrap.sh scripts/bootstrap.ps1 scripts/doctor.ps1 scripts/check-project.ps1
  versions/0.5.0.md PHASE_1_STATUS.md
)

missing=0
for f in "${required[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "MISSING: $f"
    missing=1
  fi
done

migration_count=$(find migrations -maxdepth 1 -type f -name '*.sql' | wc -l | tr -d ' ')
if [[ "$migration_count" -lt 4 ]]; then
  echo "MISSING: expected at least 4 migrations, found $migration_count"
  missing=1
fi

if grep -RniE 'DROP[[:space:]]+TABLE|TRUNCATE[[:space:]]+TABLE' migrations >/dev/null 2>&1; then
  echo 'FORBIDDEN destructive migration detected'
  missing=1
fi

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

echo 'PHASE 1 STATIC CONTRACTS: PASS'
