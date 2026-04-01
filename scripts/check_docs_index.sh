#!/usr/bin/env bash

# Ensures `docs/README.md` remains a reliable system-of-record landing page.
#
# Checks:
# - Required docs are referenced by path in `docs/README.md`
# - Any `docs/*.md` path referenced in `docs/README.md` exists on disk
# - The improvement-audit plan entry stays phase-neutral and points readers to
#   `tmp/improvement_audit_plan.md` as the canonical audit-lane status source

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DOCS_README="docs/README.md"

required=(
  "docs/INDEX.md"
  "docs/FEATURE_CHECKLIST.md"
  "docs/ARCHITECTURE.md"
  "docs/ENV_VARS.md"
  "docs/TEST.md"
  "docs/design_principles.md"
  "docs/plans/index.md"
  "docs/plans/TEMPLATE_execution_plan.md"
  "docs/plans/TEMPLATE_investigation.md"
  "docs/run_contracts.md"
)
required_pointers=(
  "tmp/improvement_audit_plan.md"
)

if [[ ! -f "$DOCS_README" ]]; then
  echo "[docs_index] Missing $DOCS_README" >&2
  exit 1
fi

missing_refs=0
for path in "${required[@]}" "${required_pointers[@]}"; do
  if command -v rg >/dev/null 2>&1; then
    found_path=0
    rg -F -q "$path" "$DOCS_README" || found_path=1
  else
    found_path=0
    grep -F -q "$path" "$DOCS_README" || found_path=1
  fi
  if (( found_path != 0 )); then
    if (( missing_refs == 0 )); then
      echo "[docs_index] docs/README.md is missing required references:" >&2
    fi
    echo " - $path" >&2
    missing_refs=$((missing_refs + 1))
  fi
  if [[ ! -f "$path" ]]; then
    echo "[docs_index] Required file missing on disk: $path" >&2
    exit 1
  fi
done

if (( missing_refs > 0 )); then
  exit 1
fi

python3 - "$DOCS_README" <<'PY'
import re
import sys
from pathlib import Path

readme = Path(sys.argv[1])
text = readme.read_text(encoding="utf-8")
canonical_phrase = "canonical source for the current audit lane status and execution order"

paths = sorted(set(re.findall(r"\bdocs/[A-Za-z0-9._/-]+\.md\b", text)))
missing = [p for p in paths if not Path(p).is_file()]
if missing:
    print("[docs_index] docs/README.md references missing files:", file=sys.stderr)
    for p in missing:
        print(f" - {p}", file=sys.stderr)
    sys.exit(1)

match = re.search(
    r"- `tmp/improvement_audit_plan\.md`(?P<body>.*?)(?:\r?\n- |\r?\n## |\Z)",
    text,
    re.S,
)
if not match:
    print("[docs_index] docs/README.md is missing the improvement-audit plan bullet.", file=sys.stderr)
    sys.exit(1)

audit_bullet = match.group(0)
normalized_audit_bullet = re.sub(r"\s+", " ", audit_bullet).strip()
if canonical_phrase not in normalized_audit_bullet:
    print(
        "[docs_index] docs/README.md must describe tmp/improvement_audit_plan.md as the canonical audit-lane status source.",
        file=sys.stderr,
    )
    sys.exit(1)

if re.search(r"Phase\s+[0-9]+|item\s+[0-9]+|waiting\s+for\s+explicit\s+confirmation", normalized_audit_bullet):
    print(
        "[docs_index] docs/README.md should not duplicate mutable phase/item/waiting status for tmp/improvement_audit_plan.md.",
        file=sys.stderr,
    )
    sys.exit(1)

print(f"[docs_index] OK ({len(paths)} referenced docs files)")
PY
