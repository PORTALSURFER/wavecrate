# Radiant Status-Bar Pilot

This note records the OPT-36 pilot slice that migrates Sempal's footer status
bar onto generic Radiant containers and widgets while the rest of the app still
runs through the compatibility shell.

## What migrated

- The footer band now composes through a generic `UiSurface<()>` built from:
  - public layout containers from `radiant::layout`
  - `TextWidget` leaves for left, center, right, and counter copy
  - a `CanvasWidget` leaf for the compact progress track
- The native shell now resolves footer segment and widget rects from that
  generic surface instead of the bespoke status-bar layout partition helper.
- Footer paint stays in the compatibility renderer for now so the visual result
  remains materially equivalent.

## Gaps surfaced by the pilot

- The compatibility shell still needs a small adapter layer to paint public
  widgets. The pilot uses generic surface layout directly but still paints text
  and progress primitives through native-shell code.
- The current native window runtime cannot yet host a generic `UiSurface`
  subtree directly inside the legacy shell scene graph. This slice uses the
  generic surface for composition/layout only.
- Public widget theming is still thinner than the shell-specific style tokens,
  so matching the native chrome exactly still requires compatibility-side paint
  code.

## Why this slice is useful

- It proves Sempal can consume `radiant::layout`, `radiant::runtime`, and
  `radiant::widgets` directly on a production UI band.
- It keeps blast radius low because the status bar is compact, stable, and has
  minimal domain coupling.
- It gives follow-up migrations a concrete adapter pattern for embedding
  generic surfaces inside the compatibility shell while runtime work continues.
