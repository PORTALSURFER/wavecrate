---
layout: default
title: Wavecrate
permalink: /
description: Sample manager with fast waveform review and destructive in-place edits.
---

# Wavecrate

Wavecrate is a focused sample-library workstation for browsing large folders, auditioning sounds quickly, carving useful ranges out of longer recordings, tagging the keepers, and handing material to a DAW without breaking the creative flow.

> This manual (`manual/`) is user-facing documentation only.
> Developer documentation lives in `docs/` in the repository.

[![Build release assets](https://github.com/PORTALSURFER/wavecrate/actions/workflows/release-build.yml/badge.svg)](https://github.com/PORTALSURFER/wavecrate/actions/workflows/release-build.yml)

<div class="download-hero">
  <div class="download-copy">Download the latest portable bundle.</div>
  <a class="download-link" href="https://github.com/portalsurfer/wavecrate/releases/latest" id="release-link">
    View latest releases
  </a>
</div>

<div class="support-callout">
  <div class="support-copy">
    It costs me a lot of time and effort to build this thing. You can use this link to show some appreciation if you enjoy the app.
      </div>
  <a class="support-link" href="https://buymeacoffee.com/portalsurfer">
    <span class="support-icon" aria-hidden="true">
      <svg viewBox="0 0 24 24" role="img" aria-hidden="true">
        <path d="M6 3h11a3 3 0 0 1 3 3v4a4 4 0 0 1-4 4h-1.3A7 7 0 0 1 8 19H7a5 5 0 0 1-5-5V9a6 6 0 0 1 6-6Zm1.3 4A4 4 0 0 0 4 11v3a3 3 0 0 0 3 3h1a5 5 0 0 0 5-5V7H7.3Zm7.7 0v4a7 7 0 0 1-1 3h2a2 2 0 0 0 2-2V6a1 1 0 0 0-1-1h-3Z" />
        <path d="M6 21h12v2H6z" />
      </svg>
    </span>
    Buy me a coffee
  </a>
</div>

<figure class="appshot">
  <img src="{{ '/assets/screenshot.png' | relative_url }}" alt="Wavecrate showing a source folder, collections, a waveform preview, and a sortable sample list" />
  <figcaption>Wavecrate browsing a Freesound folder with waveform preview, collections, metadata filters, and sample rows visible.</figcaption>
</figure>

## Quick usage guide

<div class="quick-guide">
  <section>
    <h3>Add sources</h3>
    <p>Click the plus button in Sources or drop a folder into the sidebar. Wavecrate indexes supported audio, keeps folder navigation local, and remembers source state between launches.</p>
  </section>
  <section>
    <h3>Browse and audition</h3>
    <p>Select a row to load the waveform and audition the sample. Use the folder tree, name and tag filters, collection marks, rating columns, and playback-age filters to narrow the list.</p>
  </section>
  <section>
    <h3>Mark useful ranges</h3>
    <p>Drag with the primary mouse button to create a play mark selection, or drag with the secondary mouse button to create an edit mark selection for destructive in-place edits. Resize or move handles when you need tighter timing.</p>
  </section>
  <section>
    <h3>Create and hand off clips</h3>
    <p>Extract selections into new WAV files, copy or drag selected ranges into a DAW, and move chosen samples into folders or collections for later production work.</p>
  </section>
  <section>
    <h3>Edit in place, carefully</h3>
    <p>Use trim, crop, fade, mute, gain, normalize, and delete when you want to work fast on the current file. These are destructive in-place edits; Wavecrate adds confirmations and undo-oriented recovery where it owns the operation, but you should still keep backups and choose deliberately.</p>
  </section>
  <section>
    <h3>Find related sounds</h3>
    <p>Prepare similarity data for a source, then use Find Similar or the map view to jump through related samples inside the active browser scope.</p>
  </section>
</div>

## Quick links
- [Usage guide](/wavecrate/usage)
- [Changelog](https://github.com/portalsurfer/wavecrate/blob/main/CHANGELOG.md)
- [Latest downloads](https://github.com/portalsurfer/wavecrate/releases)
- [Source on GitHub](https://github.com/portalsurfer/wavecrate)

## Hotkeys

<p class="hotkey-note">Press <kbd>Command-/</kbd> in Wavecrate to open the context-aware shortcut help overlay.</p>

<div class="hotkey-list">
  <section class="hotkey-card">
    <h3>Samples</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>Space</kbd></dt><dd>Play the selected sample. When sticky random is on, play a random sample section.</dd></div>
      <div class="hotkey-row"><dt><kbd>Shift-Space</kbd></dt><dd>Play from the current play start.</dd></div>
      <div class="hotkey-row"><dt><kbd>Option-Space</kbd></dt><dd>Play a random sample section.</dd></div>
      <div class="hotkey-row"><dt><kbd>X</kbd></dt><dd>Mark the sample and advance.</dd></div>
      <div class="hotkey-row"><dt><kbd>Command-A</kbd></dt><dd>Select all listed samples.</dd></div>
      <div class="hotkey-row"><dt><kbd>Command-C</kbd></dt><dd>Copy the play selection or selected file.</dd></div>
      <div class="hotkey-row"><dt><kbd>N</kbd></dt><dd>Normalize selected samples, or create a subfolder when no sample is selected.</dd></div>
      <div class="hotkey-row"><dt><kbd>F2</kbd> / <kbd>Command-R</kbd></dt><dd>Rename the selected item.</dd></div>
      <div class="hotkey-row"><dt><kbd>Delete</kbd> / <kbd>Backspace</kbd></dt><dd>Delete the selected item.</dd></div>
    </dl>
  </section>
  <section class="hotkey-card">
    <h3>Waveform</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>E</kbd></dt><dd>Extract the play selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>Command-E</kbd></dt><dd>Extract and trim the selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>C</kbd></dt><dd>Crop the selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>D</kbd></dt><dd>Trim the selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>L</kbd></dt><dd>Toggle loop playback.</dd></div>
    </dl>
  </section>
  <section class="hotkey-card">
    <h3>Navigation</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>Up</kbd> / <kbd>Down</kbd></dt><dd>Move the browser selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>Shift-Up</kbd> / <kbd>Shift-Down</kbd></dt><dd>Extend the sample selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>Command-Up</kbd> / <kbd>Command-Down</kbd></dt><dd>Move focus without changing marks.</dd></div>
      <div class="hotkey-row"><dt><kbd>Left</kbd> / <kbd>Right</kbd></dt><dd>Collapse or expand the selected folder.</dd></div>
    </dl>
  </section>
  <section class="hotkey-card">
    <h3>Ratings &amp; Collections</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>[</kbd></dt><dd>Lower the selected rating.</dd></div>
      <div class="hotkey-row"><dt><kbd>]</kbd></dt><dd>Raise the selected rating.</dd></div>
      <div class="hotkey-row"><dt><kbd>1</kbd>-<kbd>6</kbd></dt><dd>Toggle the selected sample in a collection.</dd></div>
    </dl>
  </section>
  <section class="hotkey-card">
    <h3>Metadata</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>`</kbd></dt><dd>Focus the tag input.</dd></div>
      <div class="hotkey-row"><dt><kbd>Up</kbd> / <kbd>Down</kbd></dt><dd>Move the tag-completion selection.</dd></div>
      <div class="hotkey-row"><dt><kbd>Esc</kbd></dt><dd>Cancel tag entry.</dd></div>
      <div class="hotkey-row"><dt><kbd>Delete</kbd> / <kbd>Backspace</kbd></dt><dd>Delete the selected tag.</dd></div>
    </dl>
  </section>
  <section class="hotkey-card">
    <h3>Transactions</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>Command-Z</kbd></dt><dd>Undo.</dd></div>
      <div class="hotkey-row"><dt><kbd>Command-Shift-Z</kbd></dt><dd>Redo.</dd></div>
      <div class="hotkey-row"><dt><kbd>Command-Y</kbd></dt><dd>Redo.</dd></div>
      <div class="hotkey-row"><dt><kbd>Shift-U</kbd></dt><dd>Toggle the transaction list.</dd></div>
    </dl>
  </section>
  <section class="hotkey-card">
    <h3>Help &amp; Modals</h3>
    <dl>
      <div class="hotkey-row"><dt><kbd>Command-/</kbd></dt><dd>Toggle shortcut help.</dd></div>
      <div class="hotkey-row"><dt><kbd>Esc</kbd></dt><dd>Close shortcut help, menus, edit prompts, dropdowns, and job or transaction panels before falling back to stop playback.</dd></div>
    </dl>
  </section>
</div>

<script>
  (function () {
    var link = document.getElementById("release-link");
    if (!link) {
      return;
    }
    fetch("https://api.github.com/repos/PORTALSURFER/wavecrate/releases?per_page=10")
      .then(function (response) {
        if (!response.ok) {
          throw new Error("release fetch failed");
        }
        return response.json();
      })
      .then(function (releases) {
        if (!Array.isArray(releases)) {
          return;
        }
        var match = null;
        var assetPattern =
          /^wavecrate-v\d+\.\d+\.\d+-(windows|linux|macos)-(x86_64|aarch64)\.zip$/i;
        for (var i = 0; i < releases.length; i += 1) {
          var release = releases[i];
          if (!release || release.draft) {
            continue;
          }
          var assets = Array.isArray(release.assets) ? release.assets : [];
          var hasBundle = assets.some(function (asset) {
            return asset && asset.name && assetPattern.test(asset.name);
          });
          if (hasBundle) {
            match = release;
            break;
          }
        }
        if (match && match.html_url) {
          link.href = match.html_url;
        }
      })
      .catch(function () {});
  })();
</script>
