# Contributor Orientation

This book is public-facing user and maintainer documentation. It should not replace the internal developer docs that already own engineering contracts.

## Where To Look

- `AGENTS.md` owns repo workflow, PR lifecycle, branch policy, and Linear routing.
- `docs/README.md` indexes canonical developer docs.
- `docs/TEST.md` owns validation lanes and test selection.
- `docs/TARGET.md` owns durable product, architecture, runtime, data, and UI contracts.
- `docs/TROUBLESHOOTING.md` owns developer troubleshooting and guardrail workflow.

## Documentation Boundaries

Use this mdBook for:

- user workflows
- public concepts
- contributor orientation
- safe links into internal docs
- publishing instructions

Keep these out of the public book unless deliberately reviewed:

- secrets or deployment credentials
- private local paths
- transient Linear planning details
- implementation notes that would confuse users
- historical execution plans

## Local Validation

Build the book before opening a PR:

```bash
mdbook build
```

Or run the repository check wrapper:

```bash
bash scripts/check.sh mdbook
```
