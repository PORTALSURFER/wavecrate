# Runbook: Changing invariants (checks + allowlists + docs)

Sempal uses small, mechanical invariants to keep changes agent-friendly and prevent new architectural drift.
Most invariants are enforced by scripts under `scripts/check_*.{sh,ps1}` and run via `scripts/ci_local.*` and CI.

## Principles

- Prefer diff-aware checks (scan only added lines / changed files) so existing legacy debt does not block progress.
- Prefer fixing the root issue over expanding allowlists.
- Keep error messages actionable: say what to do and where to move code.

## Checklist (make a safe invariant change)

1. Update the check implementation:
   - Add/update `scripts/check_<name>.sh`
   - Add/update `scripts/check_<name>.ps1` (keep behavior equivalent)
2. If the invariant needs exceptions, add/update an allowlist:
   - `docs/<name>_allowlist.txt` (or `docs/<name>_allowlist.txt` style used by existing checks)
   - Add a short justification comment for any new entry.
3. Wire it into the golden path:
   - `scripts/ci_local.sh`
   - `scripts/ci_local.ps1`
4. Wire it into CI:
   - `.github/workflows/ci.yml` (prefer Linux-only if the check depends on bash/rg)
5. Keep the agent index in sync:
   - Add/update the entry in `docs/INDEX.md`:
     - which script(s) enforce it
     - whether it is diff-aware vs full-scan
     - where to look for the allowlist (if any)
6. If the change affects docs topology, run/verify:
   - `scripts/knowledge_lint.*` (docs index + link checks)
7. If the invariant is intended as long-term hygiene, consider adding or updating scheduled “entropy” workflows:
   - `.github/workflows/entropy-*.yml` (issues/PRs that keep allowlists and docs tidy)

