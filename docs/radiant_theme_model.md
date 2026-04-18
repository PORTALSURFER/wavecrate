# Radiant Theme Boundary

Status: implemented token boundary for `OPT-37`

Purpose: define which styling tokens belong to reusable Radiant core and which
remain inside the Sempal-shell compatibility layer.

## Why this exists

Before `OPT-37`, `vendor/radiant/src/gui/native_shell/style/*` mixed two
different concerns:

- reusable color and interaction tokens that generic widgets can share
- shell-specific chrome tokens for the current Sempal sidebar/header treatment

That made the native shell look more like the long-term theme API than it
should.

## Current boundary

### Generic core theme surface

Reusable tokens now live in [`radiant::theme::ThemeTokens`] and may be consumed
by generic widgets, containers, and runtimes.

This surface includes:

- semantic background and surface colors
- border and grid colors
- generic accent and highlight colors
- primary and muted text colors
- disabled-control fill
- hover, selection, focus, and scrim blend values
- motion emphasis speeds and amplitudes

`radiant::widgets::resolve_widget_visual_tokens()` is the first generic helper
that depends on this surface without importing native-shell styling modules.

### Compatibility shell tokens

The current native shell keeps shell-only chrome values in
`vendor/radiant/src/gui/native_shell/style/chrome.rs`.

These tokens are intentionally compatibility-only because they describe the
current Sempal shell chrome rather than reusable widget/container semantics:

- source-section divider color
- folder recovery badge idle fill
- folder recovery badge active fill
- shell layout sizing in `style/sizing.rs`

## Practical rule

When adding new styling tokens:

- put reusable widget/container/runtime tokens in `radiant::theme`
- keep Sempal-shell-only chrome tokens in `gui/native_shell/style`
- do not make `radiant::theme` depend on sidebar, browser, waveform, or other
  shell region names

## Migration note

The shell still builds `StyleTokens` for compatibility, but that struct now
wraps:

- `theme: ThemeTokens`
- `chrome: ShellChromeTokens`
- `sizing: SizingTokens`

That keeps the current Sempal shell working while giving generic Radiant code a
stable place to depend on shared theming.
