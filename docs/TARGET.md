# Wavecrate Project Target: A Fast, Reliable Sample Exploration Workstation

## Vision

Wavecrate should become a focused desktop application for browsing, auditioning,
marking, organizing, and curating large audio sample libraries without breaking
the user's listening flow.

Wavecrate is not a generic GUI library, a DAW, a plugin host, or a file-manager
skin. It is an audition-first sample workstation: the user should be able to
move quickly through folders and files, hear material immediately, see useful
waveform context, mark ranges, rename or organize material, and trust that the
library state remains recoverable.

The target is a dense, responsive, predictable creative tool that can handle
large local sample collections while keeping playback, selection, and browsing
fast enough to feel direct.

## Scope and Interpretation

This document is a product and architecture target for Wavecrate, not a one-shot
implementation task.

Use it when reviewing features, refactors, UI changes, background work, and the
Wavecrate/Radiant boundary. Prefer incremental changes that move the product closer
to this target while preserving working validation lanes.

## Core Product Goals

Wavecrate should provide:

1. Fast source and folder browsing for large sample libraries.
2. Immediate sample auditioning from keyboard, mouse, and selection changes.
3. Clear waveform visualization with precise playhead, cursor, and mark ranges.
4. Reliable file and folder mutation workflows with recovery behavior.
5. Dense, low-friction UI that supports repeated professional use.
6. Background scanning, decoding, analysis, and metadata work that never blocks
   the GUI thread.
7. Predictable status and progress feedback for long-running work.
8. Stable metadata, tagging, filtering, and organization workflows.
9. Clean separation between Wavecrate product logic and Radiant GUI-library logic.
10. Tests and diagnostics that protect real user workflows and performance.

## Product Principles

Wavecrate should optimize for:

- audition-first workflows
- realtime-feeling navigation and playback
- dense, structural layouts over decorative surfaces
- reversible operations where practical
- explicit status when work is running
- correctness under large libraries and slow disks
- recoverability for destructive file operations
- predictable keyboard and pointer behavior

Perceived stalls are product bugs. If a source scan, decode, rename, analysis
job, or metadata update can take noticeable time, it belongs off the GUI thread
with clear state handoff back to the UI.

## Non-Goals

Wavecrate should not become:

- a general-purpose GUI framework
- a DAW or multitrack arrangement tool
- a plugin host
- a generic file manager
- a visual effects playground
- a database administration tool
- a collection of one-off UI experiments

Wavecrate may need DAW-like visual primitives such as waveforms, ranges, meters, or
transport controls, but those should serve sample exploration rather than expand
the product into full music production.

## Radiant Boundary

Radiant owns reusable GUI capability. Wavecrate owns sample-manager product logic.

Move functionality into Radiant when it is generic enough to support other
applications:

- layout panels and split panes
- virtualized trees, lists, and detail tables
- text input, editable rows, keyboard focus, and shortcuts
- GPU surface overlays and waveform/timeline rendering primitives
- icon buttons, SVG/image resource caching, progress widgets, and status
  surfaces
- repaint, invalidation, subscriptions, and background-resource ergonomics

Keep functionality in Wavecrate when it is product or domain behavior:

- source configuration and sample-library policy
- audio-file discovery and supported media rules
- playback and audition semantics
- sample metadata, tags, and filters
- rename, move, trash, restore, and recovery workflows
- Wavecrate-specific status wording and command behavior

If Wavecrate has to build a custom UI primitive only because Radiant lacks the
right general API, prefer improving Radiant and then migrating Wavecrate back to
the generic primitive.

## UI Target

The Wavecrate UI should be compact, stable, and optimized for scanning:

- tight margins and predictable panel geometry
- resizable sidebars and durable split positions
- folder tree, sample list, waveform, and status surfaces visible together
- clear selected, playing, loading, and failed states
- keyboard navigation for common browse and rename actions
- pointer interactions that show exact positions and ranges
- no marketing-style hero layout, decorative cards, or ornamental whitespace

Status bars should stay concise. Long-running operations should report what is
happening without monopolizing the interface.

## Performance Target

Wavecrate should handle large sources without freezing or rebuilding unnecessary UI
work.

Important performance rules:

- source scanning must stream discoveries to the UI incrementally
- folder and sample views should be virtualized or windowed for large datasets
- sample decode and waveform preparation must run in background work
- stale background completions must not overwrite newer selection state
- playback should reuse already-loaded bytes where possible
- repaint requests should match actual visual changes
- GPU waveform overlays should be composited by Radiant, not faked with extra
  application surfaces

Performance-sensitive paths should have focused tests, diagnostics, examples,
or manual validation notes when practical.

## Reliability Target

Wavecrate should treat the filesystem and source database as user-trust surfaces.

File and folder operations should:

- preserve metadata where possible
- use clear recovery paths for partial failure
- avoid silent data loss
- keep UI projection and persisted state aligned
- report failures in user-actionable terms
- avoid blocking the GUI while work is planned or executed

Background workers should be cancellable or stale-result-safe when the user
changes selection, sources, or folders before work completes.

## Documentation and Validation

Durable product and architecture contracts belong in `docs/`. Planning and
backlog state belong in Linear.

Meaningful changes should usually include:

- focused tests for behavior that can regress
- a smoke or agent validation pass before commit/push
- updated docs when a durable contract changes
- Radiant example updates when a new generic GUI API is introduced

## Completion Criteria

Wavecrate is moving toward the target when:

- browsing remains responsive on large sample sources
- sample selection starts playback quickly and reliably
- waveform marks, playhead, cursor, and selections are precise and stable
- long-running work is visible but non-blocking
- file operations preserve trust and recovery behavior
- Wavecrate code owns sample-domain decisions
- Radiant code owns reusable GUI/runtime primitives
- the current Radiant GUI can replace deprecated legacy UI paths without losing
  core workflows

