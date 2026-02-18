## Summary

- What:
- Why:

## Scope

- Single responsibility:
- Out of scope:

## Risk

- Behavior changes:
- Edge cases:
- Rollback plan:

## Validation

- Local checks: `bash scripts/ci_local.sh` / `powershell -ExecutionPolicy Bypass -File scripts/ci_local.ps1`
- Tests added/updated:
- Manual QA notes:
  - Prefer running the app via `scripts/run_sandbox.sh` / `scripts/run_sandbox.ps1` so tests and manual QA do not touch real user data.

## Docs

- User-facing docs updated (if needed): `manual/usage.md`
- Developer docs updated (if needed): `docs/`

## Throughput norms

- Keep PRs small and easy to review. If this is large, split into follow-up PRs.
- Prefer short-lived branches and merging quickly over long-running mega-branches.

## Diagnostics (when reporting bugs)

- Attach a minimal repro and (when useful) a bundle from `scripts/bug_bundle.sh` / `scripts/bug_bundle.ps1` (review before sharing; may contain local paths).
