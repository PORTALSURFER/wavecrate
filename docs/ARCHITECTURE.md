# Architecture and Ownership

This document is the durable architecture map for Sempal. It explains what the
system is optimizing for, where code should live, and how the Sempal and
Radiant boundaries are meant to stay clean over time.

## Product principles

Sempal is a realtime-oriented sample manager for exploratory listening,
selection, and curation of audio material. The architecture should preserve a
few non-negotiable product properties:

- realtime primacy
  - UI response should remain perceptually immediate
  - blocking work belongs off the UI thread
- flow preservation
  - reversible actions beat confirmation-heavy workflows
  - failures may surface, but the UI should not freeze
- audition-first design
  - playback, scrubbing, and navigation should stay fast enough to support
    listening as the primary interaction mode
- predictability over cleverness
  - state transitions should be observable and easy to reason about
- data trust
  - destructive flows need explicit recovery behavior
- performance as correctness
  - perceived slowness counts as a product defect, not a cosmetic issue

Use these principles when a change is ambiguous: prefer the calmer, more
predictable, and lower-latency design.

## Change routing rules

- domain workflows, persistence orchestration, and application state belong in
  `src/`
- new product-specific UI behavior should prefer `src/app_core/**` over the
  legacy `src/app/**` layer unless the task is explicitly legacy-runtime work
- reusable UI/runtime/layout work belongs in `vendor/radiant`
- shell-compatibility behavior should stay inside the compatibility surfaces
  rather than leaking back into generic Radiant modules

## Ownership map

### `src/app/controller/**`

Owns:

- UI intent handling
- controller orchestration
- library workflows and recovery actions

Should avoid:

- renderer-specific geometry logic
- low-level DB primitives

### `src/app_core/**`

Owns:

- host-facing application state projection
- native bridge projection/invalidation rules
- GUI action catalog and runtime test integration

Should avoid:

- direct filesystem mutation policy outside the persistence layer
- new coupling back into the legacy runtime boundary

### `src/sample_sources/**`

Owns:

- database schema and read/write APIs
- journal-backed file operations
- crash recovery behavior for file and folder mutations

Should avoid:

- UI policy and rendering behavior

### `src/issue_gateway/**`

Owns:

- issue reporting DTOs
- token storage and integration boundaries

### `vendor/radiant/**`

Owns:

- reusable layout primitives
- reusable widgets
- runtime/backend integration
- compatibility shell rendering for Sempal while migration continues

Should avoid:

- taking ownership of Sempal-specific controller or domain policy

## Radiant boundary

Radiant is moving toward a reusable GUI library with three preferred generic
areas:

- `radiant::layout`
- `radiant::widgets`
- `radiant::runtime`

The current native shell remains a compatibility surface for Sempal. Treat it
as `radiant::compat::sempal_shell` in spirit even where module names still
reflect the older structure.

Practical rule:

- new generic GUI abstractions belong in the public Radiant layers
- compatibility fixes may still touch the native shell
- new Sempal product behavior should compose generic Radiant surfaces where
  possible instead of expanding shell-only APIs

## UI design direction

The current UI direction is dense, structural, and utilitarian rather than
ornamental. Preserve these characteristics:

- strong layout structure over decorative flourish
- clear visual hierarchy and repeatable geometry
- interaction feedback that is obvious but not noisy
- stable surfaces that prioritize readability and manipulation over novelty

If a style decision would trade clarity or responsiveness for decoration, do
not take that trade.

## CODEOWNERS and guardrails

When ownership boundaries change, update both:

- `.github/CODEOWNERS`
- the relevant architecture notes in this file

Guardrail scripts in `scripts/check_*.{sh,ps1}` enforce several of these
boundaries. When one fires, fix the ownership violation before considering an
allowlist.
