# Living Voices — UI Design Style Guide

A design spec for the Slint UI of **Living Voices**, a local roleplay chat app.
Write for another LLM: follow these tokens and rules when generating `.slint` files.
Reference mockup: `ui-ux-mockup-of-a-fantasy-themed-rolepla-e3eb65.png` (see /rool-drive).

## 1. Theme & Mood

Dark warm fantasy: candlelit library at night. Parchment and aged wood textures
as *backdrops only* — interactive elements (bubbles, buttons, inputs) stay clean
and modern for readability. No heavy ornamentation on controls.

- Backdrop: aged parchment, wooden scroll rods, warm candlelight.
- Foreground elements: flat dark panels with subtle borders, gold accents.
- Serif for headers/labels, humanist sans for body text.

## 2. Color Palette

```slint
// Global tokens — define once in a shared .slint file
global Theme {
    // Backgrounds
    property <color> bg-deepest:     #1a120b;  // window background
    property <color> bg-panel:       #2b1d12;  // sidebar / side panels
    property <color> bg-panel-alt:   #33220f;  // narrator panel, cards
    property <color> bg-parchment:   #e8d5a3;  // chat scroll backdrop
    property <color> bg-parchment-dark: #d4bd8a; // scroll edges / shading

    // Message bubbles (on parchment)
    property <color> bubble-bg:      rgba(30, 20, 12, 0.82);  // dark translucent
    property <color> bubble-narrator: rgba(45, 30, 15, 0.75); // italic narrator
    property <color> bubble-user:    rgba(60, 40, 18, 0.85);  // own messages

    // Accents
    property <color> gold:           #d4a545;  // active/selected, headers
    property <color> gold-bright:    #f0c060;  // hover, focus ring
    property <color> amber-glow:     #ff9d3f;  // candle accents, icons
    property <color> bronze:         #8a6a3a;  // borders, secondary buttons

    // Text
    property <color> text-light:     #f3e6c8;  // on dark panels
    property <color> text-ink:       #2e2214;  // on parchment
    property <color> text-muted:     #a08860;  // timestamps, hints
    property <color> text-narrator:  #4a3520;  // italic on parchment

    // Status
    property <color> success:        #6db56d;  // "Active" badge
    property <color> danger:         #c06050;
}
```

## 3. Typography

- **Headers / brand / nav labels:** serif (e.g. `EB Garamond`, `Cinzel`,
  or fallback `serif`). Title ~22px, panel headers ~15px, small caps feel
  for nav items. Gold color for brand and active nav.
- **Chat message body:** humanist sans (e.g. `Noto Sans`, `Source Sans`,
  fallback `sans-serif`), 14–15px, `text-ink` on parchment bubbles.
- **Narrator text:** same sans, *italic*, slightly larger line spacing.
- **Name labels in bubbles:** serif, small caps, gold or bronze.
- **Timestamps:** 11px, `text-muted`, right-aligned inside bubble header.
- **Input placeholders:** 14px italic, `text-muted`.

## 4. Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Top bar (48px): brand left, nav tabs center, profile right   │
├────────────┬──────────────────────────────────┬──────────────┤
│ Sidebar    │  Chat area                       │  Right panel │
│ 240px      │  (flex: 1)                       │  260px       │
│ characters │  parchment scroll backdrop       │  Narrator    │
│ list       │  bubbles on top                  │  event card  │
│            │                                  │  scene image │
│            │  composer (rich input) + actions │  story clock │
└────────────┴──────────────────────────────────┴──────────────┘
```

- Window min size: 1024×640. Panels fixed width, chat flexes.
- 12px outer padding, 8–10px between panels, 8px between bubbles.
- Chat area is one large parchment rectangle with rounded corners (8px)
  and wooden rod decorations (12px dark-brown bars) at top and bottom.
- Sidebar character cards: 72px tall, avatar 48px circle left, name (serif,
  gold) + role lines, active border glow when selected.

## 5. Components

### Home page (Campaigns)
The home screen is the campaign list, shown under the same top bar as the
chat (nav tabs: Campaigns, Character, Lore, Settings, Notifications,
Profile; Campaigns active).

- Hero: parchment banner with serif title "Welcome back, <CharacterName>"
  in `text-ink` on parchment; candle-glow vignette around window edges.
- Campaign cards in a responsive grid (2-3 per row, min card ~260x160px).
  Each card is styled like a leather-bound tome:
  - Dark brown leather background `#3a2412`, rounded 8px, thin embossed
    gold border; book-spine strip on the left edge in a darker shade.
  - Title: serif, gold, 18-20px.
  - Genre tag pill top-right (Fantasy, Mystery, Horror): bronze border,
    10px small caps.
  - Progress line: 12px `text-light`, e.g. "Chapter 4 - Evening" (mirrors
    the campaign's Story Clock state), above a thin 3px progress bar
    (bronze track, gold fill).
  - Row of small (24px) circular character avatars.
  - Gold `Continue` pill button bottom-right.
- Active campaign card: glowing gold border (2px `gold-bright` + soft
  outer glow) - corresponds to the last-opened campaign.
- New Campaign card: same tome shape but dashed bronze border, large gold
  plus sigil centered, label "New Campaign".
- Quick start panel: small side card, header "Quick start" (serif gold),
  hint text, and a vertical list of preset scenario pills (Tavern Meeting,
  Dungeon Crawl, Court Intrigue) - bronze pills, hover gold.

### Message bubble
- Dark translucent bubble (`bubble-bg`), corner radius 10px, 1px border
  `rgba(212,165,69,0.35)`.
- Header row: `[Name]` in serif small-caps + timestamp right (11px, muted).
- Narrator variant: italic text, `bubble-narrator`, no name-box styling —
  label `[Narrator]` only, reads as scene direction.
- Own messages: slightly warmer tint (`bubble-user`), right-aligned optional.

### Composer (single rich input with inline segments)
ONE unified multi-line text editor, not separate fields. The user composes a
single flowing message mixing action types as inline styled segments:

- Segments are visually distinguished inline:
  - **Speech** - plain text, default color (`text-light` / `text-ink`).
  - **Action** - amber-tinted segment, e.g. `*draws a dagger slowly*`,
    background `rgba(212,165,69,0.18)`, radius 6px, italic.
  - **Thought** - purple-tinted dimmed segment (color `#9a86c9` on
    `rgba(90,70,140,0.25)`).
  - **Whisper** - segment with a small circular avatar chip of the target
    plus a `to <Name>:` prefix, purple-dim tint, e.g. `I whisper to Morwyn: ...`.
- The buttons below do NOT create separate messages; they are formatting
  modes: pressing `Do` wraps the next typed text (or current selection) as an
  action segment; `Whisper` opens a small target-picker (avatar chips of
  present characters) then wraps the next text as a whispered segment;
  `Say` returns to plain speech. Any order, any number of segments per send.
- Send button: gold paper-plane arrow, right of the editor.
- One send produces ONE message whose body stores the ordered segments; the
  event log keeps the flow intact (speech, action, speech read naturally).

### Rendered message bubbles
A sent message renders its inline segments in one bubble, preserving order:
plain speech text, amber-italic action text `*like this*`, purple-dim
thought text, and whispered text prefixed with `-> <Name>:`. Timestamps and
the `[Name]` header stay as before.

### Action bar
- Row of buttons: `Say` `Do` `Think` `Whisper` plus a settings gear on the right.
- Buttons: dark brown pill (`#3a2412`), 1px bronze border, serif label.
- Buttons are one-shot formatting switches, not multi-select checkboxes. The
  chosen mode glows: gold border plus warm inner glow for Say/Do, purple
  tint for Think/Whisper. `Whisper` shows the target's tiny avatar chip on
  the button while a whisper segment is open.
- Default mode: `Say`.

### Narrator panel (right)
- Header "Narrator" (serif, gold) with a small quill/scroll icon.
- Event card: parchment-dark background, italic serif text of the interrupt,
  plus an `EVENT: <name>` tag line in small caps bronze.
- Below: scene snapshot image (16:9, rounded 8px, thin gold border).

### Story clock
- Brass sundial-style circular widget, ~120px, gold ring, needle.
- Digital time centered (e.g. 19:48) and phase word below in small caps
  (EVENING, NIGHT, MORNING…). Update on story-clock tick events.

### Sidebar character card
- Avatar (circular, 48px, thin gold ring), name (serif gold), class/role
  line, optional status badge ("Active" — green pill, 10px).
- Selected card: gold left border or soft outer glow.

### Top bar
- Brand left: serif, gold, with a small pentacle/scroll emblem.
- Nav: small-cap serif labels with icons; active tab underlined gold.
- Right: notifications bell + profile avatar.

## 6. Slint Implementation Notes

- Define all colors/typography in a `global Theme {}` (or `Palette`) in a
  shared `theme.slint`; reference as `Theme.gold` etc.
- Fonts: bundle `.ttf` files and register via `Window { default-font-family: }`
  or per-element `font-family`. Provide serif fallback since system serif
  varies.
- Use `TouchArea` states for formatting-mode buttons (single active mode,
  one-shot switch, not radio exclusion of Whisper target selection). For the
  inline segments use `Inline`/rich `Text` markup or a small segment-flow
  layout inside the editor; keep one logical message per send.
- Bubbles are `Rectangle` with `border-radius: 10px;` inside a
  `VerticalLayout` in a `Flickable` over the parchment backdrop.
- Keep textures (parchment, wood) as `Image { source: }` tiled/stretched
  backgrounds — never render text on raw textures without a translucent
  bubble overlay.
- Accessibility: min contrast of body text on parchment (dark ink) and on
  bubbles (light cream) — both exceed 4.5:1 with the tokens above.
- Animations: subtle — 150ms ease for hover/toggle states, gentle fade-in
  (200ms) for new bubbles, slow candle-glow pulse on accents if desired.
