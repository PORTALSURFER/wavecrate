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
- **#0D0E10** — primary background for the app canvas and modal bases  
- **#121417** — secondary background for panels and cards  
- **#191C20** — tertiary background for controls and list rows to add depth  
- **#3E444C** — panel outline for outer frames and strong dividers  
- **#4A5058** — grid strong for primary lines in displays and separators  
- **#2D3238** — grid soft for secondary lines and subtle row backing

### Text
- **#E7ECF2** — primary text for labels, buttons, and inputs  
- **#A1AAB5** — muted text for helper copy and secondary metadata  
- **#FFFFFF** — high-contrast text on tinted badges/overlays

### Accents and feedback
- **#66C2FF** — accent mint; active selection fills, positive focus, and cool-state emphasis  
- **#9CE7FF** — accent ice; focus strokes, hyperlinks, and high-visibility cool highlights  
- **#5C9DFF** — accent copper; secondary cool accent for waveform/playhead and tool-state contrast  
- **#54D6FF** — cyan highlight used for marked/active overlays and similarity emphasis  
- **#F2B65C** — warning foregrounds and attention strokes  
- **#FFCC7D** — soft warning/informational fills  
- **#E86565** — destructive/error strokes and text

### Badges and chips
- Idle **#2F343A**; Busy **#F2B65C**; Info **#66C2FF**; Warning **#FFCC7D**; Error **#E86565**

### Interaction overlays & triage
- Drag highlight **#66C2FF** (outline alpha varies by state)  
- Duplicate hover: fill **#1E2227**, stroke **#9CE7FF**  
- Triage: Trash **#E86565** (subtle **#A94D4D**), Keep **#66C2FF**  
- Missing marker **#E86565**  
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
  - **#121417 – #191C20**

### 6.2 Waveform View Style

**Background**
- Dark panel: **#0D0E10 – #191C20**  
- Overlay subtle vertical grid lines (beats/frames):
  - Primary grid: **#4A5058** (1px)  
  - Secondary grid: **#2D3238** (thinner or lower opacity)  
- Optional horizontal zero line: **#3E444C** (1px)

**Waveform Curve**
- Curved line is allowed here, but must feel “instrumental”:
  - 1–2px line
  - Primary color:
    - Default: **#5C9DFF** (accent copper, repurposed as a cool waveform accent)
    - Alternative highlight: **#66C2FF** (accent mint) for selected/armed
  - No blur, no glow; if you need emphasis, use:
    - double-line effect (bright core, darker outline)
    - or stepped opacity segments

**Filling / Energy**
- Optional under-curve fill:
  - Very subtle, 5–15% opacity of the waveform color (cool blue by default)
  - Hard clipped at zero (no soft feathering)
- For selection regions:
  - Rectangular bands with sharp edges, using **#F2B65C** for warning/attention states or **#5C9DFF** for edit/active tooling states

**Additional Details**
- Peaks or markers depicted as:
  - Thin vertical bars (no rounded markers), **#9CE7FF**
  - Small blocky ticks along the top or bottom
- Zoom/pan handles: small square grips aligned to frame edges

### 6.3 Spectrogram / Frequency Displays

**Background**
- Same base as waveform (**#0D0E10 – #191C20**)  
- Primary grid:
  - Vertical lines for time (**#2D3238**)  
  - Horizontal lines for frequency (**#4A5058**)

**Color Mapping (Sci-Fi Hard Theme)**
- Use a **cold, high-tech palette** with minimal hues:
  - Low energy: **#0D0E10 – #191C20**
  - Mid energy: **#30343A**
  - High energy: **#66C2FF**
  - Saturated peaks (very sparing): **#5C9DFF** or **#F2B65C**
- Avoid rainbow spectrums; keep it within blue–cyan range for coherence.

**Rendering Style**
- Rectangular “pixels” or tiles:
  - Each time/frequency bin drawn as a small rectangular cell
  - Slightly hard, no blur on cell edges
- Optional horizontal banding noise to mimic sensor data

**Curves / Overlays**
- Overlays like EQ curves or analysis lines:
  - Thin 1px lines, **#5C9DFF** (secondary cool accent) or **#66C2FF** (mint)
  - Allow smooth curves but:
    - No dot handles with circles — use small squares/diamonds
    - No glow; emphasize with line thickness or double-line effect

### 6.4 Other Data Views (Scopes, Vectors, Custom Displays)

**Oscilloscope / Lissajous**
- Frame same as waveform view  
- Curves allowed but:
  - Use crisp lines, no blur
  - Colors:
    - Main: **#5C9DFF** (accent copper, cool waveform accent)
    - Secondary/ghost: **#66C2FF** with low opacity
- Optional trail effect:
  - Simulated with alpha decay, not blur

**Bar / Column Meters**
- Use vertical or horizontal **rectangular segments**  
- Segment colors:
  - Low: **#30343A**
  - Mid: **#4A5058**
  - High: **#66C2FF** / **#5C9DFF** for peaks
- Peak hold indicator: small rectangular cap, no rounded shapes

### 6.5 Display Chrome & Labeling

**Borders**
- Outer border: **#3E444C** (1–2px)  
- Inner inset border: **#0D0E10** or **#121417** to suggest depth

**Labels / Axis Text**
- Typeface: monospaced or technical-looking sans-serif  
- Color: **#A1AAB5** at 70–80% opacity  
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
- Cold, Technical, Mechanical  

---
