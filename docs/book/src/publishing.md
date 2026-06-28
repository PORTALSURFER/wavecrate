# Publishing the Book

The intended public path is:

```text
https://portalsurfer.org/wavecrate/docs/
```

The mdBook source lives in the Wavecrate repository under:

```text
docs/book/src/
```

The local build output goes to:

```text
target/mdbook/wavecrate-docs/
```

That output is generated and should not be committed to the Wavecrate repository.

## Local Build

```bash
mdbook build
```

## Local Preview

```bash
mdbook serve --open
```

## Public Site Wiring

The public Wavecrate welcome page should link to `/wavecrate/docs/`.

Publishing should fit the existing PortalSurfer static site workflow. The static mdBook output can be copied into the site repository at `site/wavecrate/docs/` during the publish step, or generated there by a release/deploy script if the two repositories are coordinated.

The first implementation documents and previews the book locally, then wires the public link and static path without changing the release download or donation-gate routes.
