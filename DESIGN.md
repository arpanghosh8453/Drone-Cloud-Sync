---
name: drone-cloud-sync
description: "Field index: an open, precise workspace for cloud flight logs and verified local files."
colors:
  background: "#eef2f6"
  surface: "#ffffff"
  subtle: "#f6f8fa"
  border: "#dce3ea"
  control-border: "#8191a3"
  text: "#25364a"
  muted: "#5c6e82"
  primary: "#263d59"
  primary-hover: "#354f6d"
  on-primary: "#ffffff"
  tint: "#eaf1f7"
  success: "#306e65"
  danger: "#b13838"
  warning: "#865d1f"
  rail: "#263d59"
  rail-text: "#ffffff"
  rail-muted: "#cfdae7"
  rail-hover: "#344d6a"
  rail-selected: "#3e5773"
  focus: "#216d80"
  dark-background: "#151e29"
  dark-surface: "#1b2633"
  dark-subtle: "#202e3d"
  dark-border: "#364555"
  dark-control-border: "#71859a"
  dark-text: "#e3eaf2"
  dark-muted: "#a7b7c8"
  dark-primary: "#b8cede"
  dark-primary-hover: "#d1e2ef"
  dark-on-primary: "#192a3c"
  dark-tint: "#2a3f53"
  dark-success: "#9acdc0"
  dark-danger: "#f1a2a2"
  dark-warning: "#e1bf83"
  dark-rail: "#142437"
  dark-rail-muted: "#bbcede"
  dark-rail-hover: "#213952"
  dark-rail-selected: "#2d4963"
  dark-focus: "#9acddd"
typography:
  headline:
    fontFamily: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
    fontSize: 28px
    fontWeight: 620
    lineHeight: 1.2
    letterSpacing: -0.03em
  title:
    fontFamily: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
    fontSize: 17px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: -0.02em
  section:
    fontFamily: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
    fontSize: 14px
    fontWeight: 600
    lineHeight: 1.5
  body:
    fontFamily: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
    fontSize: 14px
    fontWeight: 400
  control:
    fontFamily: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
    fontSize: 13px
    fontWeight: 550
    lineHeight: 1.3
  hint:
    fontFamily: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif
    fontSize: 12px
    lineHeight: 1.6
  mono:
    fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", monospace'
    fontSize: 11px
    lineHeight: 1.7
rounded:
  control: 5px
  compact: 3px
  progress: 2px
spacing:
  tight: 4px
  compact: 6px
  control: 8px
  small: 10px
  row: 12px
  field: 14px
  section: 18px
  group: 24px
  page: 36px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: 0 14px
    height: 40px
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.on-primary}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: 0 14px
    height: 40px
  button-outline:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: 0 14px
    height: 40px
  button-ghost:
    backgroundColor: transparent
    textColor: "{colors.muted}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: 0 14px
    height: 40px
  button-destructive:
    backgroundColor: transparent
    textColor: "{colors.danger}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: 0 14px
    height: 40px
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: 0 12px
    height: 40px
  nav-active:
    backgroundColor: "{colors.rail-selected}"
    textColor: "{colors.rail-text}"
    rounded: "{rounded.control}"
    padding: 0 13px
    height: 44px
  badge-success:
    textColor: "{colors.success}"
  settings-section:
    padding: 26px 0
  library-summary:
    textColor: "{colors.muted}"
    padding: 18px 0
  flight-row:
    textColor: "{colors.text}"
    padding: 0 14px
    height: 60px
---

# Design System: drone-cloud-sync

## Overview

**Creative North Star: "Field index"**

Field index treats the interface as a working file library: a slate-blue navigation rail, cool white work surfaces, fine row rules, and readable task controls. The density supports scanning filenames, dates, and transfer states without decorative dashboard cards.

The same restrained structure carries through sign-in, downloads, settings, and flight details. Verification uses a quiet teal signal alongside explicit text; dark mode preserves the semantic roles on blue-gray surfaces. Light, dark, and system appearance are established choices.

**Key Characteristics:**

- Open tables and divided sections instead of decorative dashboard cards.
- Slate-blue navigation with a clear selected destination.
- Compact controls, fine rules, and explicit status text.
- Brief state transitions with a reduced-motion override.

## Colors

The primary is a muted slate blue; cool neutral surfaces carry the work, and restrained semantic colors identify state. The frontmatter records the exact CSS values. Every `dark-` entry is the corresponding override in the root dark theme; the rail text remains white in both themes.

### Primary

- **Slate blue:** primary actions and selection, with a separate hover value and contrasting on-primary text. In dark appearance the primary action becomes pale blue with dark text.
- **Navigation slate:** the rail has independent background, muted text, hover, and selected roles so its contrast remains stable in both themes.
- **Focus teal-blue:** the visible keyboard outline and input caret; rail controls use the rail text color for focus instead.

### Neutral

- **Workspace white:** the main surface and field background.
- **Cool mist:** the outer background, subtle surface, and tinted selection establish quiet layers.
- **Ink and secondary ink:** primary text and muted explanatory text remain separate in both themes.
- **Fine rule and control stroke:** content dividers use border; interactive outlines use the stronger control-border role.

Verification uses success, recoverable attention uses warning, and errors or destructive actions use danger. These are semantic signals rather than additional brand accents.

**The Semantic State Rule.** Use verification, warning, and danger colors to identify actual states, with readable text beside the signal.

## Typography

One system sans stack serves headings, labels, controls, and records. The code stack is limited to hashes and raw metadata. There is no display-font role or oversized hero treatment in this utility.

### Hierarchy

- **Headline:** page titles use the headline token. At the narrow breakpoint the heading becomes (25px); sign-in uses (25px) with a (1.3) line height.
- **Title:** second-level headings use the title token; settings and details headings use a compact variant (15px).
- **Section:** third-level headings use the section token.
- **Body:** the root reading size is the body token; paragraphs use a (1.6) line height. Table values and task text commonly step down to (13px).
- **Control:** buttons use the control token; form labels use (12px) and weight (550).
- **Hint:** supporting text uses the hint token. Filenames, counts, and metadata can use (11px) where their secondary role is clear.
- **Mono:** integrity hashes and raw metadata use the mono token. Dates, counts, and transfer numbers use tabular numerals within the sans stack.

**The Working Type Rule.** Use the system sans hierarchy for tasks and records; reserve monospace for hashes and raw metadata.

## Layout

The desktop shell fills the viewport, with a navigation rail (218px), a flexible work area, and a folder footer (44px). Work-area padding is (36px 36px 28px). Related controls use small gaps from the recorded spacing scale; major groups generally separate by (24px). Settings remain within (820px), while sign-in has an open form column capped at (370px), beside a separate navigation-colored region.

The flight library is an open table with sticky headers (42px) and rows (60px), separated by fine horizontal rules. Cells use horizontal padding (14px). The table scrolls in its own region; filters wrap and filenames truncate where necessary rather than stretching the shell. The inline library summary has top and bottom rules, wrapping counts, and a secondary note.

At or below (1200px), the rail becomes (190px), work-area padding becomes (28px 24px), and an adjacent details pane becomes (300px) instead of (320px). At or below (1000px), the rail is (178px) and details overlay the work area as a right sheet capped at (360px). At or below (700px), navigation becomes a compact horizontal top bar, work-area padding becomes (25px 18px), search occupies a full filter row, details fill the work area, and sign-in becomes one column with `auto 1fr` rows so its brand header hugs its content and the form receives the remaining space. The supported body minimum width is (320px).

## Elevation & Depth

The workspace is flat at rest. Surface tones and one-pixel rules distinguish headers, rows, selected records, footers, settings sections, and details. Normal tables, settings sections, inputs, and controls have no shadow. The narrower overlaid details sheet alone uses a soft left shadow (`-12px 0 36px #0f22351a`); the full-width narrow sheet removes it.

**The Flat Workspace Rule.** Separate normal content with row rules and surface tones. Reserve the observed shadow for the overlaid flight-details sheet.

## Shapes

Controls, notices, local paths, and popovers use the control radius. Checkboxes and inner theme options use the compact radius; progress uses its smaller radius. This is a compact, gently squared form language rather than a pill system. Tables and settings sections remain open, with square shared boundaries and fine rules. Icons are restrained line SVGs; the verification badge uses a small CSS-drawn check beside its text.

## Components

### Buttons

Direct, compact actions use a consistent control silhouette. Standard buttons are (40px) tall with horizontal padding (14px) and a (1px) border. Small buttons are (32px) tall with horizontal padding (10px); icon buttons are (34px) square. Primary uses primary/on-primary; outline uses the surface with a control stroke; ghost uses muted text on transparent; destructive uses danger text on transparent. Hover changes the background, using primary-hover, tint, or a low danger mix. Disabled controls use opacity (0.5) and a not-allowed cursor.

### Inputs / Fields

Text inputs and selects share the standard control height, surface background, control stroke, and radius. Inputs use horizontal padding (12px); search adds a leading SVG and extra left padding. Hover strengthens the border to focus. Placeholders remain muted and fully opaque. Keyboard focus uses a (2px) outline offset by (3px). Labels remain outside the field.

### Navigation

The blue rail holds the brand, three destinations, and account status. Navigation targets are (44px) high with a muted default, lighter hover surface, and distinct selected fill with stronger text weight. At the narrow breakpoint the same destinations become text tabs in a horizontal bar; the long brand label and account block are hidden.

### Status badges and progress

Badges are plain inline text (12px), rather than colored pills. A successful badge adds a small check; warning and danger retain explicit status wording. Progress is a thin track (4px), with a (200ms ease-out) width transition; indeterminate activity uses the observed moving segment. Download rows use rules and align actions near the relevant transfer.

### Containers and settings

Settings are divided sections, with vertical padding (26px), no lifted card frame, and grouped controls. Notices and active sync feedback are the bounded exceptions: they use a border, the control radius, and a tinted or low semantic-color background. Output paths wrap inside a quiet bordered region.

### Inline library summary and flight table

The signature summary places Cloud records and Local verified counts in one ruled line, joined by an arrow. Counts use stronger (18px) tabular text; the note stays secondary. Table rows use subtle hover and tinted selected/current states. Sorting, checkboxes, filters, and the optional Columns disclosure stay conventional.

### Flight details and motion

Flight details sit beside the library when space permits and become a dismissible sheet below the details breakpoint. A close button and Escape dismiss the panel, and focus returns to the invoking control. Technical information lives in disclosures below the primary file actions. Details enter with a small horizontal translation over (180ms), using `cubic-bezier(0.16, 1, 0.3, 1)`. Color and border state transitions take (160ms). Reduced-motion preference collapses animations and transitions to (0.001ms) and one iteration.

## Do's and Don'ts

### Do:

- Do use semantic CSS variables so controls and states follow light, dark, and system appearance.
- Do keep labels, buttons, and navigation in normal sentence case.
- Do preserve horizontal table scrolling and wrapping controls at narrow widths.
- Do pair status color with text and keep visible keyboard focus.
- Do use the inline Cloud records → Local verified summary when showing this library relationship.

### Don't:

- Don't add gradients, promotional slogans, feature tiles, or decorative dashboard cards to this working utility.
- Don't give ordinary settings sections or table rows lifted card shadows.
- Don't use verification teal as a general decorative accent.
- Don't make integrity hashes or raw metadata the main reading layer.
- Don't delay access while a reveal animation runs.
