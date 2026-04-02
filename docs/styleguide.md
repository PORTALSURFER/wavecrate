---
layout: default
title: Style Guide
permalink: /styleguide
description: Microchip brutalism rules that keep the Sempal UI coherent.
---

# Microchip Brutalism — GUI Style Guide
Inspired by a microchip die

---

## 1. Color Palette (Current App Palette)

### Core surfaces
- **#0A0A0A** — primary background for the app canvas and modal bases  
- **#121212** — secondary background for panels and cards  
- **#1C1C1C** — tertiary background for controls and list rows to add depth  
- **#3A3A3A** — panel outline for outer frames and strong dividers  
- **#4A4A4A** — grid strong for primary lines in displays and separators  
- **#2B2B2B** — grid soft for secondary lines and subtle row backing

### Text
- **#EAEAEA** — primary text for labels, buttons, and inputs  
- **#A9A9A9** — muted text for helper copy and secondary metadata  
- **#FFFFFF** — high-contrast text on tinted badges/overlays

### Accents and feedback
- **#FF7A1A** — `accent_mint`; primary active selection, keep-state emphasis, and focus accents under the existing token name  
- **#FFC247** — `accent_copper`; waveform/playhead emphasis and secondary active tooling contrast  
- **#E14B32** — `accent_trash`; destructive/error strokes and text  
- **#FFB347** — `accent_warning`; warning foregrounds and attention strokes  
- **#FF922B** — `highlight_orange`; bright ember highlight for prompts, inputs, and hot actions  
- **#FFD08A** — `highlight_orange_soft`; soft warm informational fills  
- **#C63C1E** — `highlight_blue`; deep ember/red contrast for edit overlays under the existing token name  
- **#FF9A5C** — `highlight_blue_soft`; softened orange-red text/overlay tint  
- **#FFC83D** — `highlight_cyan`; hot gold used for marked/active overlays and similarity emphasis  
- **#FFE08A** — `highlight_cyan_soft`; pale gold used for focus strokes and elevated highlights

### Badges and chips
- Idle **#303030**; Busy **#FFB347**; Info **#FF7A1A**; Warning **#FFD08A**; Error **#E14B32**

### Interaction overlays & triage
- Drag highlight **#FF7A1A** (outline alpha varies by state)  
- Duplicate hover: fill **#1E1E1E**, stroke **#FFC83D**  
- Triage: Trash **#E14B32** (subtle **#8F3B31**), Keep **#FF7A1A**  
- Missing marker **#E14B32**  
- Palette source of truth: `vendor/radiant/src/gui/native_shell/style/palette.rs` — keep these values in sync when the theme shifts.

---

## 2. Geometry / Shape Language

### Core Shape Rules
- Only **rectilinear** shapes.
- **No curves**, no rounded corners, for UI chrome.
- To “soften” corners, use a **45° diagonal bevel** instead.
- Dense layers of rectangular blocks.
- Strong separation lines and strict grid behavior.

### Structural Forms Derived From the Chip
- Long vertical stacks  
- Horizontal banding  
- Checkerboard micro-patterns  
- Modular “bays” and compartments  
- Highly partitioned containers

### Line Geometry
- 1–2px micro-lines  
- 4–8px structural divider lines  
- 45° diagonals for transitions or “bridges”

---

## 3. Layout Principles

### A. Dense Structure
- Minimal empty space  
- Tight spacing  
- Everything looks *purpose-built* and mechanical

### B. Module-within-Module Design
- Outer frame  
- Subdivided sections  
- Micro-complex nesting

### C. Repetition
- Repeated bars  
- Repeated squares  
- Repeated micro-lines for rhythm and structure

### D. Asymmetry With Balance
- Not perfectly symmetrical  
- Variations in density  
- Intentional irregularities

---

## 4. Surface Texture Simulation

### Microline Textures
- Fine horizontal/vertical streaks  
- Use for: buttons, sliders, small panels

### Scanline / Interference Patterns
- Good for headers, status bars, separators

### Gridfill Textures
- 2x2 or 4x4 subtle pixel grids  
- Used to differentiate UI groups

---

## 5. Components Design Language

### Buttons
- Rectangles only  
- No curved corners  
- Optional diagonal bevel (45°)  
- Thin 1px outline  
- Pressed state: inset 1px inner shadow

### Sliders
- Track: long, narrow, rectilinear, microtextured  
- Handle: square block with possible 45° cut  
- Tick marks: 1px micro-grids

### Panels
- Thick outer frames  
- Subsection grids  
- Repeated vertical dividers

### Tabs / Navigation
- Tall, thin rectangular tabs  
- Resemble microchip partitions  
- Depth indicated by offset shading

### Meters / Waveforms (UI chrome)
- Sharp, blocky containers  
- No curved shapes in the surrounding chrome  
- Monochrome grey stack visuals

---

## 6. Displays / Views (Data Visualizations)

> **Rule:** Curves are only allowed *inside* dedicated data displays (waveforms, spectrograms, analyzers). All surrounding chrome must still follow the hard, rectilinear style.

### 6.1 Display Frames

- Display areas (e.g. waveform view, spectrogram, meters) sit inside:
  - A **rigid rectangular frame**
  - With 1–2 nested inner borders to mimic multi-layer chip regions
  - Optional 45° bevels on outer corners only if you need visual hierarchy
- Use a slightly lighter background than the main app:
  - **#121212 – #1C1C1C**

### 6.2 Waveform View Style

**Background**
- Dark panel: **#0A0A0A – #1C1C1C**  
- Overlay subtle vertical grid lines (beats/frames):
  - Primary grid: **#4A4A4A** (1px)  
  - Secondary grid: **#2B2B2B** (thinner or lower opacity)  
- Optional horizontal zero line: **#3A3A3A** (1px)

**Waveform Curve**
- Curved line is allowed here, but must feel “instrumental”:
  - 1–2px line
  - Primary color:
    - Default: **#FFC247** (`accent_copper`, repurposed as a gold waveform accent)
    - Alternative highlight: **#FF7A1A** (`accent_mint`) for selected/armed
  - No blur, no glow; if you need emphasis, use:
    - double-line effect (bright core, darker outline)
    - or stepped opacity segments

**Filling / Energy**
- Optional under-curve fill:
  - Very subtle, 5–15% opacity of the waveform color (gold by default)
  - Hard clipped at zero (no soft feathering)
- For selection regions:
  - Rectangular bands with sharp edges, using **#FFB347** for warning/attention states or **#C63C1E** for edit/active tooling states

**Additional Details**
- Peaks or markers depicted as:
  - Thin vertical bars (no rounded markers), **#FFE08A**
  - Small blocky ticks along the top or bottom
- Zoom/pan handles: small square grips aligned to frame edges

### 6.3 Spectrogram / Frequency Displays

**Background**
- Same base as waveform (**#0A0A0A – #1C1C1C**)  
- Primary grid:
  - Vertical lines for time (**#2B2B2B**)  
  - Horizontal lines for frequency (**#4A4A4A**)

**Color Mapping (Ember Industrial Theme)**
- Use a **warm, restrained palette** with minimal hues:
  - Low energy: **#0A0A0A – #1C1C1C**
  - Mid energy: **#303030**
  - High energy: **#FF7A1A**
  - Saturated peaks (very sparing): **#FFC247** or **#FFB347**
- Avoid rainbow spectrums; keep it within red-orange-gold for coherence.

**Rendering Style**
- Rectangular “pixels” or tiles:
  - Each time/frequency bin drawn as a small rectangular cell
  - Slightly hard, no blur on cell edges
- Optional horizontal banding noise to mimic sensor data

**Curves / Overlays**
- Overlays like EQ curves or analysis lines:
  - Thin 1px lines, **#FFC247** (secondary gold accent) or **#FF7A1A** (ember orange)
  - Allow smooth curves but:
    - No dot handles with circles — use small squares/diamonds
    - No glow; emphasize with line thickness or double-line effect

### 6.4 Other Data Views (Scopes, Vectors, Custom Displays)

**Oscilloscope / Lissajous**
- Frame same as waveform view  
- Curves allowed but:
  - Use crisp lines, no blur
  - Colors:
    - Main: **#FFC247** (`accent_copper`, gold waveform accent)
    - Secondary/ghost: **#FF7A1A** with low opacity
- Optional trail effect:
  - Simulated with alpha decay, not blur

**Bar / Column Meters**
- Use vertical or horizontal **rectangular segments**  
- Segment colors:
  - Low: **#303030**
  - Mid: **#4A4A4A**
  - High: **#FF7A1A** / **#FFC247** for peaks
- Peak hold indicator: small rectangular cap, no rounded shapes

### 6.5 Display Chrome & Labeling

**Borders**
- Outer border: **#3A3A3A** (1–2px)  
- Inner inset border: **#0A0A0A** or **#121212** to suggest depth

**Labels / Axis Text**
- Typeface: monospaced or technical-looking sans-serif  
- Color: **#A9A9A9** at 70–80% opacity  
- Alignment:
  - Frequency labels: left or right edge  
  - Time labels: bottom edge  
- Use blocky separators (short lines) instead of dots or circles

---

## 7. Lighting & Shading

### General Aesthetic
- Mostly flat shading  
- Subtle metallic reflections  
- Sharp edge highlights (1–2px)  
- Depth conveyed by layered geometry, not blur

### Avoid:
- Blur  
- Glow (except tiny LED accents)  
- Soft gradients  

Only micro-linear gradients allowed.

---

## 8. Interaction Feel

### Behavioral Personality
Interactions should feel:
- Mechanical  
- Precise  
- Hard-edged  
- Instant and snappy

### Allowed Interactions
- Sliding blocks  
- Hard toggles  
- Snap-open compartments

### Prohibited
- Bounce animations  
- Soft fades  
- Curved motion paths

---

## 9. High-Level Style Keywords

- **Microchip Brutalism**  
- Rectilinear Density  
- Industrial Metal  
- Machine-Logic Aesthetic  
- 45° Geometry  
- Partitioned Complexity  
- High-Frequency Patterns  
- Dark Circuit Board  
- Warm, Technical, Mechanical  

---
