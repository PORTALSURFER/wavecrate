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

- Smoke local checks: `bash scripts/ci.sh smoke` / `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 smoke`
- Fast local checks: `bash scripts/ci.sh quick` / `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 quick`
- Full CI parity (when relevant): `bash scripts/ci.sh local` / `powershell -ExecutionPolicy Bypass -File scripts/ci.ps1 local`
- Tests added/updated:
- Manual QA notes:
  - Prefer running the app via `scripts/run.sh sandbox` / `scripts/run.ps1 sandbox` so tests and manual QA do not touch real user data.

## Docs

- User-facing docs updated (if needed): `manual/usage.md`
- Developer docs updated (if needed): `docs/`

## Throughput norms

- Keep PRs small and easy to review. If this is large, split into follow-up PRs.
- Prefer short-lived branches and merging quickly over long-running mega-branches.

## Diagnostics (when reporting bugs)

- Attach a minimal repro and (when useful) a bundle from `scripts/run.sh bug-bundle` / `scripts/run.ps1 bug-bundle` (review before sharing; may contain local paths).
