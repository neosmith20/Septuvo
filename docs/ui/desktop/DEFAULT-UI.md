# Septuvo VPN — Default (Simple) Desktop UI: Binding Owner Specification

**Document ID:** `SEPTUVO-UI-DEFAULT-001`  
**Document version:** `1.0 — owner-directed implementation specification`  
**Prepared:** 2026-10-18  
**Product:** Septuvo VPN desktop client, **Default / Simple interface**  
**Canonical repository destination:** `docs/ui/desktop/DEFAULT-UI.md`  
**Canonical image (provided separately by owner):** `docs/ui/desktop/Default_VPN_UI.png`  
**Advanced UI image:** Owner will supply and approve separate functional specification; **NOT defined in this file**.  
**Governance:** This file captures the owner's approved image, approved seven-area functionality, approved Connections operations, profile switching, X/tray behavior, and subsequently approved delegated choices for the configuration editor, diagnostics, settings, and import/export. The measured UI geometry and specific implementation constants below are **engineering specifications derived from that reference**, **not separately drawn/approved UI artwork**. Before final UI sign-off the owner compares working screenshots against the exact image.

> **HARD RULE FOR ALL AGENTS:** The owner-approved screenshot is the only default main-window visual design. DO NOT substitute another mockup, turn this into a list/sidebar dashboard, invent navigation, relocate controls, introduce additional tiles, redesign the globe/shield/status area, remove anything in the image, replace the logo, alter the layout, or implement assumed peer provisioning. This document is the behavior, sizing, copy, and accessibility contract—not permission to redesign. All changes require an explicit owner decision. No implementation or repository push is authorized merely because this specification exists.

## 0. Priority, scope, and terminology

### 0.1 Source-of-truth order

1. Owner's later, explicit written decisions prevail over earlier suggestions in chat.
2. This file controls the **Default UI's specific approved behavior and main-window composition**.
3. `Default_VPN_UI.png` controls the appearance and relative placement of the seven elements. Actual brand assets/tokens control faithful colors/logos; an attractive approximation is not acceptable.
4. Repository security rules (`AGENTS.md`, `docs/DECISIONS.md`, security gates, policy specs) remain mandatory. **A UI document cannot waive G1, implement encryption, bypass authorization, change licensing, or claim security that is not verified.** If a requested design behavior clashes with those rules, stop and ask the owner—do not quietly choose a side.
5. The Advanced UI's content is **not** specified here; **do not infer it from this file**. Interface Mode only switches to the separately approved advanced design when that mode is implemented.

### 0.2 Exact seven components on the approved main image

The numbers are for documentation only. Do **not** render numbered labels on the UI.

| # | Existing component | Placement in approved image | Mandatory purpose |
|---|---|---|---|
| 1 | **Home VPN** dropdown (name changes with selected profile) | Upper center beneath logo | List all saved profiles; select profile; automatic switch if a different one is selected while connected |
| 2 | **Connections**; subtitle **Manage Saved Connections** | Bottom **left** tile | Open saved connection manager with 3-dot actions Edit, Rename, Export, Duplicate, Delete |
| 3 | **Add / Import**; subtitle **VPN Configuration** | Bottom **center** tile | New blank draft configuration or import existing Septuvo configuration/invitation once supported |
| 4 | **Help & Diagnostics**; subtitle **Get Help or Run Tests** | Bottom **right** tile | Open troubleshooting and diagnostics window |
| 5 | Large main **Connect / Disconnect** button plus status display/shield | Center | Act on selected/current profile and display truthful state |
| 6 | **Settings** cog | Upper **right**, below title-bar controls | Open global settings; mode, close behavior, startup, connection, updates, About/Credits/version |
| 7 | **— / □ / X** native-style caption controls | Top-right edge | Minimize / maximize-restore / owner-approved X disconnect-and-exit or send-to-tray |

Header brand icon + **Septuvo VPN** wordmark, large shield, glow, earth/network visual, status heading, supporting lines, duration line, horizontal separator, and these three footer tiles are **part of the approved view**, not dispensable decoration.

### 0.3 Application boundaries

- Main client and other Septuvo-owned VPN/Server **source** stays Rust, per the owner's clarified requirement. External OS APIs, services, drivers and DLLs are allowed to be non-Rust; dependency, licensing and security review still applies. No assumption that a Rust wrapper automatically provides security.
- This is the **desktop** Default UI. Primary target: Windows; same visual design intended for macOS/Linux with OS-appropriate window controls, menus and file dialogs, separately verified. No Android/iOS layout is authorized by this document.
- Septuvo Server remains CLI-first unless separately authorized; this file does not create a server administration GUI.
- All active network configuration, credential handling, policy enforcement and service operations reside in the authoritative privileged background service, never in an elevated GUI. The GUI only communicates through the authorized service/API boundary.
- **Current pre-G1 software cannot make a real SVP VPN connection.** It must clearly show development/unavailable status, never a fake green shield or a “Protected” result, until all necessary security checks genuinely pass.

## 1. Branding and visual contract (non-negotiable)

### 1.1 Official repository assets

Use existing, approved assets (do not invent new logos or color codes):

- `brand/04 - Colors And Tokens/brand-tokens.md`
- `brand/04 - Colors And Tokens/septuvo-colors.json`
- `brand/04 - Colors And Tokens/septuvo-theme.rs`
- VPN-specific icons under `brand/03 - App Icons/VPN/`, for example `VPN_Dark_256.png`, with suitable resolution variants.
- Approved horizontal Septuvo VPN brand lockup from the brand kit's Logos/Product Family; **do not assemble an approximate logo using an arbitrary typeface** when the official lockup is available.
- Source image: `./Default_VPN_UI.png` (same folder as this Markdown file, **owner uploads it**).

**No images are embedded, generated or delivered with this specification.** The image must be provided by the owner. If the referenced image or individual approved image elements (e.g. the world-network background art) are not available in usable source form, notify the owner and obtain approved source art or an explicit rendering plan **before** substituting graphics. Do not render a whole screenshot as a fake interactive UI; all text and buttons must be real accessible widgets.

### 1.2 Exact brand colors; use by semantic role

| Official name | Hex | Intended usage in this UI |
|---|---|---|
| Midnight | `#050A14` | Main backdrop and deep surfaces |
| Navy | `#0B1220` | Raised/darker surfaces, inactive selector/card background |
| Surface | `#1E293B` | Elevated panel fills when a solid fill is needed |
| Border | `#334155` | Non-focused borders/separators, adjusted opacity for glowing blue outlines |
| Primary Blue | `#0066FF` | Primary button base, selections, focus outline |
| Electric Blue | `#00D5F8` | VPN product highlights, globe/light rings, hover accent |
| Sky Blue | `#38BDF8` | Secondary links/information highlights |
| White | `#F8FAFC` | Primary text and light icons |
| Muted | `#94A3B8` | Supporting text/subtitles |
| Success Green | `#10B981` | Verified positive status **only** |
| Warning Yellow | `#F59E0B` | Connecting, attention, limited/unknown states |
| Error Red | `#EF4444` | Failure / lost service; destructive confirmation |

The brand kit includes additional Teal, Aqua and Purple colors, but **Septuvo VPN is the blue / electric-blue product family**. Do not substitute Protocol teal or Server purple as the main VPN UI accent. The image contains glow/gradient blends: sample **their composition** visually from the approved reference, using the official solid color tokens as input. Do not invent a different neon palette or flat white redesign.

### 1.3 Typography and exact capitalization rules

- UI font **Inter** from the brand kit; Regular 400, Medium 500, SemiBold 600, Bold 700. Use appropriately licensed font data if bundled; respect existing brand/font licenses.
- Logo text must use the official Septuvo VPN lockup, not recreated text.
- Normal button/menu/section/setting **labels use Title Case**: `Connect`, `Disconnect`, `Connections`, `Manage Saved Connections`, `Add / Import`, `VPN Configuration`, `Help & Diagnostics`, `Get Help or Run Tests`, `Interface Mode`, `Close Button Behavior`, `Check for Updates`.
- Full explanatory sentences use **sentence case**, normal punctuation: `Your connection is secure and private.`
- Acronyms must be correctly capitalized: **VPN, IP, DNS, LAN, MTU, CLI, TLS, SVP, QR, GitHub, Windows, macOS, Linux**. Never `Vpn`, `Dns`, `Ip`, `Github`.
- Consistency rule: buttons and standalone field/section headings Title Case; helper text and error sentences sentence case. No ALL CAPS in ordinary labels; all-caps only where established brand lockups use it.
- Standard sizes at intended baseline: brand wordmark as supplied; protected heading **44 px Bold**; other state headings **32–40 px Bold** according to length; normal body/status help **16 px**; main action button **24 px SemiBold**; selector name **23 px SemiBold**; footer tile title **14–15 px SemiBold**; footer tile subtitle **13 px Regular**; settings page heading **24 px SemiBold**; settings section title **17 px SemiBold**; fields **14–16 px**; captions **12–13 px**.
- Minimum body 14 px at normal OS scaling except true short captions 12–13 px; do not simply shrink every font when window narrows. Respect user text scaling/accessibility; allow scrolling rather than clipping.
- Line heights: headline 1.12–1.2; body 1.35–1.5; card titles 1.3. No font substitution without discussing render differences with owner.

### 1.4 Shape and rendering

- Dark window with subtle blue outline and corner radius matching screenshot (reference nominal outer radius **18 px**).
- Selector/card surfaces have small blue borders and deep navy fills; rounded corners approximately **14–16 px**.
- Main action button is a bright blue → cyan blue-gradient rounded pill/rectangle (radius approximately **40 px** at screenshot size); not a plain text link.
- Hero world/network motif stays **behind** shield/status content and must never interfere with text readability.
- Large ringed cyan/blue shield icon (visually distinct from the small header brand S mark) is central, not a new connection-toggle control. The separate labeled Connect/Disconnect button performs the action.
- Hover/focus states may add a subtle existing-brand outline/glow; no unapproved moving, pulsating, spinning or distracting animation. Respect reduced-motion settings; no indefinite background animation requirement.
- Minimum 4.5:1 normal text contrast and 3:1 large-text/UI boundary contrast where applicable; status always has a **word** in addition to color/icon.

## 2. Window sizing and layout measurements

### 2.1 How to interpret the image sizes

The original user-supplied **PNG is 1448 × 1086 image pixels**, which includes **desktop wallpaper outside the application**. Approximate bounding rectangle of the depicted app is **x = 241–1167, y = 92–965**, i.e. **926 × 873 image pixels**. The outside blue wallpaper is **not** part of the app and must not be recreated as an application background.

The following is a careful **visual measurement guide** from that exact raster, accurate approximately to **±4–8 image pixels** because blurred glow has no crisp edge. It is not a claim that the artwork came with engineering coordinates. All rectangles in the table use the APP'S TOP-LEFT as `(0,0)`; numbers are **reference-image pixels** at the screenshot's display scale. Maintain their **relative composition**, then use the runtime size contract in §2.3.

| Element | Left (x) | Top (y) | Width | Height | Notes |
|---|---:|---:|---:|---:|---|
| Outer app frame | 0 | 0 | 926 | 873 | Approximately 18 px rounded frame |
| Brand icon / S symbol area | 30 | 23 | 56 | 80 | Original branded asset; don't squish/stretch |
| Brand wordmark area | 98 | 44 | 265 | 45 | “Septuvo VPN” official lockup |
| Window caption controls area | 737 | 12 | 168 | 35 | Minimize, maximize/restore, close |
| Settings cog hit region | 859 | 59 | 44 | 44 | Upper right below caption controls |
| Profile dropdown | 123 | 111 | 687 | 79 | Full rounded rectangular selector |
| Profile leading home icon | 148 | 129 | 53 | 48 | Use home icon only when profile is Home; more detail §3.1 |
| Profile friendly name | 224 | 134 | 420* | 42 | `Home VPN` in reference; *flexible text region* |
| Profile chevron | 759 | 138 | 28 | 27 | Downward chevron always at right |
| Large centered circular shield area | 328 | 207 | 267 | 268 | Ring+shield, visually centered; not a button |
| Main status heading | 300 | 470 | 327 | 57 | `Protected` exemplar, centered |
| Supporting text group | 251 | 529 | 427 | 61 | Two centered lines in protected state |
| Connected-duration/status line | 337 | 598 | 258 | 25 | Color-coded circle + text |
| Main Connect/Disconnect button | 165 | 639 | 597 | 91 | Large blue-gradient, rounded action |
| Divider above footer cards | 24 | 743 | 879 | 1 | Low-contrast line |
| Footer Connections card | 24 | 760 | 285 | 89 | Left |
| Footer Add / Import card | 321 | 760 | 283 | 89 | Center |
| Footer Help & Diagnostics card | 617 | 760 | 285 | 89 | Right |

The world-network effect is a **decorative layer** across the center/background of the main window. It does not create a separate card, page, header, or control. Decorative glow may extend beyond the crisp bounds listed for the shield, as in the image.

### 2.2 Layout math and relationships (mandatory)

- App frame padding: reference **24 px** around bottom tiles; body elements centrally aligned to app midpoint `x = W/2`.
- Three bottom tiles: **three equal widths** at baseline, with nominal **12–16 px** gaps; 3-column order never changes: Connections → Add / Import → Help & Diagnostics.
- Settings cog stays in upper-right **below** native titlebar control row, not moved to an arbitrary side menu.
- Connection selector is centered with approximate width **74%** of content window at the reference size, not a sidebar/list.
- Main action button centered at approximate width **64%** of content window and height approximately **91 reference pixels**; centered below duration/status text and **above** footer separator.
- Shield/ring centered, approximately **267 reference pixels** square, and must remain visibly larger than every small icon.
- The main status text is CENTER-ALIGNED, under shield; two helper lines and timer are below it; never put this content in a left sidebar.
- The brand header stays TOP LEFT, native caption buttons TOP RIGHT, Settings cog below them.
- Decorative image never contains interactive or baked-in status lettering. Accessible text is separately rendered as actual widgets.

### 2.3 Runtime logical-pixel targets (implement, test, adjust for OS DPI without redesign)

Screenshot pixel coordinates do not map directly to physical pixels at all desktop DPI scales. For Windows use **device-independent logical pixels** and OS scaling. The following are the intended functional sizes, derived from the relative screenshot composition and the owner's request that this be a compact, resizable desktop client:

| Item | Runtime specification |
|---|---|
| **Initial Simple window client size** | **780 × 760 logical px** (excluding any OS-added non-client frame); centered on active screen on first run |
| **Minimum usable window size** | **520 × 580 logical px**; scrolling must expose all controls without obscured/hit-inaccessible footer |
| **Saved size/position** | Remember last non-maximized dimensions and screen-relative position; recover automatically if monitor unplugged/offscreen; don't make critical UI unreachable |
| **Maximum size** | No artificial maximum below OS desktop size; maximize/restore standard behavior. Constrain main content to **1100 logical px** wide, centered in larger window, preserving proportions |
| **Header** | ~80 logical px high at baseline, logo top-left; Settings hit area **44 × 44 logical px** |
| **Profile selector** | Baseline **580 × 66** logical px; centered; min **240** wide; height at least **52** |
| **Main ring/shield** | Baseline **220 × 220** logical px; reduce to **160 × 160** on narrow/short windows if needed, without changing its form |
| **Main action button** | Baseline **500 × 76** logical px, width min(500, available width – 40), height at least **56**, with 16–20 px text/icon gap |
| **Footer tile row** | Three equal tiles, each min approximately **215 × 74** logical px when there is room; 12 px gaps; 20 px outer horizontal margin; target internal padding 12–16 px |
| **Footer icon** | **34–40** logical px; never less than 24 px |
| **Menu/action hit targets** | At least **40 × 40** logical px desktop, favor **44 × 44** for small icons/tray actions |
| **Text** | Inter; see §1.3. Text sizes are NOT scaled down solely to fit a window; wrap or scroll |
| **Standard panel gap** | **12–16** logical px; section gap 20–24; 1 px hairline borders; use supplied tokens |

Responsive breakpoints and behavior:

- **Width ≥ 700 logical px:** the approved **three-across footer layout**; centered selector and action button; full-size header branding.
- **Width 520–699 logical px:** retain SAME content and ordering, but **stack the three footer cards vertically** (Connections, Add / Import, Help & Diagnostics) after the main action rather than reducing text below readable size. This is a defined responsive adaptation, NOT an alternate design. The vertical content area scrolls.
- **Height below fit:** allow vertical scrolling of content rather than hiding the button/footer. Header controls stay accessible. Never snap-scroll back to top on status refresh, or clip controls.
- **Maximized/wide:** keep content centered, maintain the same top-to-bottom sequence and status-centered composition; do not replace the default layout with the Advanced dashboard.
- **DPI/text scaling:** honor Windows 100%, 125%, 150%, and 200%; do not multiply hardcoded physical pixels; rerun scroll/accessibility tests. The OS may supply caption button sizes; don't fight their hit areas.
- User can switch Simple/Advanced while connected without reset/reconnect. On returning to Simple, show the same live selected/connected status, not placeholder data.

**Pixel-accuracy rule:** In N4-01 or the relevant UI work order, show owner a screenshot at the agreed initial client size and another at narrow/minimum size, alongside the canonical reference. Any meaningful change to layout, proportions, button position, shield/art, or labels must be proposed and approved; agents may tune render alignment to match reference but not reinterpret the approved look.

## 3. Existing main-window element functionality — fully defined

### 3.1 [1] Profile selector — `Home VPN` in reference

**Placement:** centered upper selector rectangle under brand row; fixed chevron at right.

**Closed state:** show the **friendly name** of the saved profile that is selected, not a physical endpoint, DNS address, or organization name. `Home VPN` is **example text**, not a forced real profile name.

**Open behavior:**

1. Click anywhere within the selector (or activate with Enter/Space) to open a popover/dropdown anchored directly beneath the existing selector, matching its width and approved colors.
2. List **all saved profiles** in persisted display order, one row each, **single selection**. Use their stored friendly names; prevent empty names and visually ambiguous duplicates. No fabricated entries for hypothetical public VPN locations.
3. Each row is a **minimum 44 px** tall; available names left-aligned; the current active profile may carry a small written `Connected` indicator, but not a misleading status; current selection visibly indicated without relying only on color.
4. If list has more than **6 profiles**, restrict popover list to **6 rows / 264 px** and scroll **inside the dropdown**. Do not let popup cover the Connect button unnecessarily; if platform constraints force placement above, keep focus and accessibility correct.
5. Click selected name / Enter → close dropdown and select profile.
6. Clicking the **same** profile does nothing to tunnel or timer.
7. If disconnected, choosing a different profile just selects it. The main button continues to read `Connect`.
8. **If a DIFFERENT profile is currently connected, the owner's decision is automatic switch:** initiate disconnect from old server, preserve fail-closed policy, then connect to newly selected server. No second Connect click, no routine confirmation, no simultaneous conflicting tunnels. Display `Switching Connection…` while transitioning. See §7.
9. After a successful switch, new profile is active and selected. If new connection fails, leave **new** profile selected, report `Connection Failed`, old tunnel not active; **do not silently reconnect the old one**. Preserve kill switch per security model.
10. Selection persists between launches; choosing a new profile is not permission to change stored config parameters.

**Empty state (zero profiles):** selector displays `No Connections`; disable Connect and direct the user to the **existing** `Add / Import` footer button with one short accessible note `Add or import a VPN configuration to get started.` Don't create an additional hero button. No dummy preinstalled `Home VPN` configuration.

**Profile incomplete draft:** list entry available to edit/select; Connect disabled for this profile with readable explanation `Complete this configuration before connecting.` and one route through existing Connections → Edit.

**Icon:** screenshot shows a house icon because its example is `Home VPN`. Do not infer geography from profile names or assign country flags. For arbitrary profiles use one consistent approved neutral connection icon, preserving the existing icon position and dimensions; if suitable art is not available, request owner sign-off before substituting a new style.

### 3.2 [2] `Connections` — `Manage Saved Connections`

**Placement:** bottom-left tile. Owner approved subtitle change from `Manage VPN Servers` to **`Manage Saved Connections`**. Preserve tile dimensions, server-stack pictogram, right chevron, dark blue styling.

**Action:** open a **secondary Connections manager window** (not a third main-screen layout). It lists **all locally saved profiles** with human names and status. Use vertical, scrollable rows. Each row has a three-dot overflow menu `⋯` at right. Every profile row contains no secret material.

**Approved three-dot menu, EXACT ordered items:**

1. **Edit** — open the configuration editor (§4.2) for that saved profile, with endpoint, routing, DNS and other supported fields; no silent disconnect simply to view/edit. Preview/validate before commit; if currently connected, service safely applies/reconnects according to backend capability; otherwise explain that change will take effect on next connection. Never allow a leaky transient state.
2. **Rename** — change **friendly display name only**; no cryptographic rekey, endpoint change, network side effect or disconnect. Validate trimmed, nonempty unique name, at most **64 Unicode characters** (implementation UX constraint); reject duplicates case-insensitively to avoid confusing users; no silent overwrites.
3. **Export** — use standard OS **Save As** file picker, choose destination and filename; see §4.4. This is an **existing-device backup**, not another user's ready-to-use enrollment pass.
4. **Duplicate** — create a **new draft** using non-secret, reusable configuration settings only; do **not** copy device identity, private credentials, tokens, PSKs, key material, completed enrollment, or authentication grants; generated suggested name `Copy of <Original Name>` (append number if needed); user may rename it later.
5. **Delete** — explicit confirmation naming the target, e.g. `Delete “Home VPN”?`; `Cancel` / `Delete Connection`. No recoverability is implied. If this is the active profile, first safely disconnect it, handle any failure, and do not delete it while still in use. If this is the selected profile, select the next remaining saved profile or show empty state. No backend device revocation is implied by local deletion; if enrollment revocation exists later, make that distinction plain rather than claiming deletion revokes access.

**Connections manager window specification:**

- Opens centered over parent; initial **700 × 540 logical px**, min **500 × 380**, resizeable, min outer padding **20 px**; title top-left **Connections**; short subtitle `Manage Your Saved Connections`.
- Each row baseline **72 px** tall, full available width less panel 20 px side margins; icon 32 px; 2 text lines max (name and state); `⋯` hit target **44 × 44 px**; row bottom border 1 px; gap **8 px**.
- Scroll profile list within window; do not scroll the main default window behind a modal editing dialog. 1-click on row selects highlight for inspection; overflow `⋯` contains the commands, not hidden right-click-only behavior.
- Overflow menu anchor aligned to three-dot control, width **180–220 px**, rows **40–44 px** with focus and Escape dismissal. Use exactly approved action order above. Destructive Delete separated subtly and uses approved error color; no surprise bulk operations.
- No visible Add/Import button is necessary in this manager; entry to creation remains the EXISTING `Add / Import` tile, as approved. Do not introduce client-to-client invitations, live peer list, server management dashboard, or device generator here.
- If no saved profiles, show `No saved connections yet.` with instructional text pointing to `Add / Import`, without inventing an alternative setup wizard.
- Editing/draft/name/export actions must update the top dropdown immediately on successful save, no full app restart.

### 3.3 [3] `Add / Import` — `VPN Configuration`

**Placement:** bottom-center tile. Existing blue circle-plus pictogram, chevron, title and subtitle; no replacement main-screen button.

**Action:** open secondary **VPN Configuration** choice dialog, initial **560 × 360 logical px**, min **480 × 330**, with two clearly labeled actions using the approved functionality:

- **Create New Configuration**: open blank editor §4.2; allow saving incomplete configuration as draft; not a working VPN until all required values and enrollment are valid.
- **Import Configuration**: open the platform's native **Open File** dialog directly (§4.3). The user finds the file wherever stored and clicks **Open**. Do not ask them to paste file text, know filesystem paths, or use a command line.

This choice dialog is **secondary only**, not a new permanent main screen. Caption `VPN Configuration`, explanatory sentence `Create a new connection or import one provided to you.` Buttons each min **44 px** high, 12 px gap, simple Cancel/close. Do not start an unapproved multi-page setup wizard.

An invitation / QR enrollment option may be implemented later only under an approved enrollment design after G1; it is **not** a functional promise today. Do not label a plain configuration file as a trusted invitation.

### 3.4 [4] `Help & Diagnostics` — `Get Help or Run Tests`

**Placement:** bottom-right tile; existing question-mark pictogram, chevron, capitalization.

**Action:** open separate, resizable **Help & Diagnostics** window with initial **740 × 600 logical px**, min **540 × 440**, content scroll if necessary. Do not change or replace the main screen. Details §5.

### 3.5 [5] Central shield + `Connect` / `Disconnect`

**Layout:** large centered circular shield and world backdrop; status heading below; two helper lines; small status-dot duration row; large centered blue primary button. None of these is moved into a sidebar or replaced with a graph.

- **Connect** starts the selected, fully valid configuration only. If no complete profile, disable Connect and display a reason and direction to Add/Import or Edit.
- **Disconnect** deliberately disconnects the **actually active** tunnel; when connected show `Disconnect`. When a different selection triggers switch, keep actual active identity distinct until disconnect confirmation.
- The central shield is a **status indicator, not a hidden second Connect button**.
- A real `Protected` state requires service reachability, authenticated server identity, functional adapter/tunnel traffic and valid DNS policy/checks as defined by authoritative backend (D-020). Fail/unknown in one means **not Protected**.
- Duration `Connected for HH:MM:SS` counts from the **verified** successful connection only, not click time; resets on disconnect/switch; after 24 hours show `DD:HH:MM:SS` or an unambiguous days+time format, not an overflowing hour display.
- When not connected, **do not** show a running connection timer, check-mark shield, green dot, or statement that internet is encrypted. Maintain same element positions using truthful text/state/icon treatment.
- While connecting or disconnecting, disable repeated conflicting actions; support retry after definite failure, not unbounded automatic spam.
- Errors must be specific in plain language, never silent. Distinguish service unavailable, invalid configuration, unreachable endpoint, failed authentication, DNS failure, and kill-switch blocking when the backend can distinguish them. Don't assert a specific diagnosis from one failed ping.
- `Connect` is not a promise of internet privacy for a **split-tunnel** profile. Supporting copy must say the connection protects the configured networks only when applicable; no generic `All your internet activity is encrypted` claim for split mode.

**Main action sizes and focus:** button baseline **500 × 76 logical px**; minimum **56 px** high; main text 24 px SemiBold; leading power symbol 28–32 px; accessible name exactly `Connect to <name>` / `Disconnect from <name>` while visible text remains `Connect` / `Disconnect`. Enter/Space activates when focused. Busy state visible and announced.

### 3.6 [6] Settings cog

**Placement:** upper-right below caption buttons, same as image; hit box min **44 × 44 logical px**; recognizable gear icon; keyboard label `Settings`.

**Action:** secondary **Settings** window, initial **700 × 640 logical px**, min **520 × 480**, resizable, scrollable, no change to the approved main UI. All settings and defaults are in §6; no unapproved tabs/settings/advertising or online accounts.

### 3.7 [7] Caption buttons `—`, `□`, `X`

Use OS-standard position/semantics (Windows as pictured). Hover, focus, maximize behavior follow OS conventions. Title bar must remain keyboard and mouse usable at any size.

| Control | Definitive behavior |
|---|---|
| **— Minimize** | Standard minimize to taskbar/dock; **does not** disconnect the VPN, regardless of X preference. |
| **□ Maximize / Restore** | Standard maximize to working area / restore previous window bounds; **not** exclusive full-screen kiosk and does not alter connection. |
| **X — DEFAULT** (`Close Button Behavior = Exit and Disconnect`) | If the service is controlling **any active, connecting, switching, or kill-switch-only/wanted state**, **request deliberate disconnect** through service; after confirmed success, **exit app**. If already fully disconnected with no active policy session, simply exit. This is the **factory default**, not keep-running-in-background. No routine extra confirmation dialog. |
| **X — when `Send to System Tray` selected** | Hide window into system tray, keep Septuvo app running and existing tunnel connected; if disconnected, keep it disconnected. Do not disconnect, change profile or show a shutdown warning. |

**Safety exceptions:** If X's requested disconnect fails or service is unresponsive, **do not claim disconnection or silently exit leaving an apparently disconnected state**. Keep GUI open, show `Could not disconnect safely.` and a clear Help & Diagnostics route; keep the existing protective firewall state and allow retry. Never disable the kill switch merely to force window close.

**Tray semantics (secondary OS interface, not main-window redesign):**

- When tray is available, tray icon uses official VPN brand icon, with accurately labeled status. Left-click or `Open Septuvo VPN` restores window; no surprise Connect or Disconnect on tray-icon click.
- Minimal tray menu: `Open Septuvo VPN`, `Connect` or `Disconnect` depending on actual state, `Help & Diagnostics`, `Exit Septuvo VPN`. No server list, no new management UI invented in tray.
- `Exit Septuvo VPN` in tray **always means deliberate disconnect and full exit**, whether or not X is configured to send to tray. Use the same failure handling. The owner-approved X policy applies to **clicking X**, not overriding explicit `Exit`.
- If the OS has no usable tray host, selecting `Send to System Tray` must show a warning/explanation and fall back to normal visible window behavior; **never hide an irrecoverable window** or silently disconnect. Do not advertise tray functionality where unsupported.
- If Septuvo's background daemon exists after GUI exit, the X-close policy still requires the **tunnel to be disconnected**. No reliance on GUI process termination alone for disconnect.

## 4. Secondary configuration windows and file behavior

### 4.1 Shared conventions for all secondary windows

- Same brand font, blue controls, dark surfaces, status vocabulary; readable, minimal and professional. Secondary windows are utility dialogs, **not alternative main UI designs**.
- Each window centered on current parent, stays within visible desktop work area; child windows may scroll/rescale; no off-screen Apply/Save buttons.
- Primary submit button lower right, **Cancel** to its left; Escape closes if no unsaved edits, otherwise confirm discard of pending changes. Clicking X in a **secondary dialog** closes only that dialog—**not** VPN/disconnect; main-window X policy doesn't apply to child dialogs.
- No auto-save of incomplete typing into privileged config; user explicitly saves. Unsaved field changes do not alter active backend policy. Invalid fields show errors next to relevant input, with plain language and an optional exact technical explanation.
- Keyboard tab order top-to-bottom, left-to-right; visible focus outlines, logical label binding, Enter only triggers explicit default action when safe; Delete requires affirmative confirmation.
- Min field touch/click targets 40–44 px, minimum field height **40 px**, dialog body text 14–16 px, 12–16 px vertical space, helper text 12–13 px.

### 4.2 `VPN Configuration` editor (New, Edit, Duplicate)

**Window:** `VPN Configuration` title; baseline **700 × 620 logical px**, min **540 × 450**, resizable. Header shows context-specific subtitle: `New Configuration`, `Edit Configuration`, or `Duplicate Configuration` (not new main-window modes). **Basic fields visible first**, technical fields in single **Advanced Configuration** disclosure collapsed by default (remember editor expansion only if helpful; new users first see it collapsed).

**Basic form (first screen):**

| Row | Label (exact) | Field and behavior |
|---|---|---|
| 1 | **Connection Name** | Single-line text, max 64 Unicode characters, required for saving even a draft, unique case-insensitively, friendly example `Home VPN`; no cryptographic effect |
| 2 | **Server Address** | Single-line endpoint text, sample `203.0.113.10:51820`; help `Enter the server address and port provided to you.` Backend currently accepts **IP:port only**, not a hostname; don't falsely accept `vpn.example.com` until the security/config spec supports it |
| 3 | **Advanced Configuration** | Disclosure button/row, collapsed by default, keyboard and screen-reader accessible; expands the fields below |

**Advanced Configuration fields:**

| UI Label | Underlying concept | Constraints / guidance |
|---|---|---|
| **Routing Mode** | `routing.mode` | `Full Tunnel` (protect all supported traffic via tunnel) or `Split Tunnel` (only specified destinations); explain clearly; don't imply split mode encrypts entire internet |
| **Networks Through VPN** | `routing.tunnel` | Valid IPv4/IPv6 CIDR prefixes for routed VPN networks; show example and validate with existing strict parser |
| **Direct Network Access** | `routing.direct` | Approved direct CIDR exceptions; show warning where direct traffic bypasses VPN; no blanket open internet override |
| **Blocked Networks** | `routing.block` | CIDRs blocked by policy, validated by backend |
| **DNS Mode** | `dns.mode` | `VPN DNS` or `System DNS` only where routing policy permits; forbid System DNS with Full Tunnel per existing config spec |
| **VPN DNS Servers** | `dns.resolvers` | 1–4 IP addresses routed through tunnel; validate routes and address types; no DNS leak allowed |
| **Local DNS Exceptions** | `dns.lan_exceptions` | Up to 8 allowed local resolver IPs covered by direct routes; explicit warning about direct DNS |
| **Interface Name** | `tunnel.interface` | Valid interface name 1–15 chars; if absent backend default `septuvo0` (platform may need separate implementation later) |
| **MTU** | `tunnel.mtu` | Integer 1280–9000; backend default 1420; show useful help, not jargon in Basic mode |
| **Always On** | `behavior.always_on` | Only where actual service supports semantics. Must not override user's deliberate disconnect on X; no unapproved background reconnect following exit |
| **Log Retention (Days)** | `logging.retention_days` | 0–365, default 7; no private keys or secrets in logs |

**Server identity/enrollment:** no editable cryptographic key/identity field before G1. Once verified enrollment exists, provide a separate approved flow; do not invent QR token structure, invitations or secret material here.

**Editor bottom actions:** `Cancel`, **`Save Configuration`**. When a config is incomplete but has valid Connection Name, allow **`Save Draft`** rather than falsely claiming it can connect; if essential required fields missing, no Connect until complete. An incomplete draft is **application-owned draft data**, NOT a malformed `septuvo.toml` written to the privileged service; only validated complete device configurations may be applied to the backend. Backend validates again when used; GUI validation is not authoritative.

**Edit of an ACTIVE profile:** display current saved values (never blank). Permit editing while connected. On save: stage and validate; have privileged service atomically commit/reload safely and reconnect **only if** service has safe atomic apply support. If safe apply/reconnect is unavailable, refuse with a specific explanation and retain original configuration; do **not** force silent disconnect or leave half-applied settings. On failure restore previous settings/policy and report what happened. No risky mid-save leaks.

**Duplicate:** opens same editor with nonsecret copied fields, draft status, suggested unique `Copy of <Name>`, *never* copies existing identity/enrollment/private keys/tokens/PSKs. One click to save or cancel.

### 4.3 Import — standard native Open dialog

1. Click **Add / Import**, then **Import Configuration**.
2. Immediately open the operating system's standard native **Open File** chooser, starting at OS default/recent folder, with a filter for supported **Septuvo** configuration files and an `All Files` fallback as safe. Do NOT invent a `.svp`, `.svpn`, or special binary format. Existing development device configs are TOML; support `.toml` when compatible with validated device schema.
3. User navigates any accessible folder, selects file, clicks **Open**. Cancel returns without changes.
4. Read size-bounded content (currently **64 KiB** device-config limit), validate strictly against authoritative parser, reject unknown keys/unsafe destinations without partial save or policy change. No automatic execution of import content.
5. Show readable name and **import preview** of nonsecret settings; allow user to set friendly Connection Name if source has no name (current TOML device schema has no name field). Detect duplicate name or identical profile and ask for a unique name; **never silently overwrite** another saved profile.
6. Confirm with **`Import Configuration`**; save locally, add to dropdown and Connections manager, and select the new profile **without auto-connecting**. Show success `Configuration imported.`
7. If file is not complete/enrolled, clearly label saved item **Draft / Setup Required** and disable Connect pending proper enrollment. This is expected pre-G1.
8. Never treat receipt of a file as proof that the server/device is trusted. Authenticity of enrollment must be established separately through the approved protocol design.
9. Error examples: `This file is not a supported Septuvo configuration.`; `The file is too large.`; `The configuration contains an invalid server address.` Include details only where trustworthy and actionable.

### 4.4 Export — standard native Save As dialog

1. Open **Connections**, use selected row's `⋯ → Export`.
2. Open OS native **Save As** dialog. Suggest filename derived from friendly name after safe filesystem sanitization, e.g. `Home VPN.toml` for a compatible **non-secret** TOML configuration; filename is user-editable. Do not force a folder.
3. User chooses folder/name and clicks **Save**. OS overwrite confirmation is required if file exists; never overwrite silently.
4. Export must be a **valid, parsable configuration** with a documented schema and be re-importable in the same intended context. Preserve only allowed settings. When encrypted backup support is eventually approved, define file extension and credential protections then—not now.
5. **No plaintext export of private keys, private device identity, enrollment tokens, PSKs or other authentication secrets** by default. A nonsecret configuration export may require enrollment again on import; warn clearly: `This backup does not include private credentials. You may need to set up access again.`
6. **Export ≠ invite someone else.** Do not generate new client identities, share private secrets, or provision peers via Export. Peer-to-peer invitations need separately approved SVP design/G1.
7. Saving succeeds → `Configuration exported.`; OS/file/write failure → clear message, no profile mutation. Exports are local only; no automatic cloud upload.

## 5. Help & Diagnostics specification

**Window title:** `Help & Diagnostics`  
**Size:** initial **740 × 600 logical px**, min 540 × 440, resizable/scrollable  
**Primary action:** **`Run Diagnostics`** button, at least 44 px high  
**Secondary actions:** `Ping VPN Server`, `Ping Public Host`, `Open Windows Network Settings` where supported, `Save Diagnostic Report`, `Show Technical Details` collapsed by default.

### 5.1 Test set, exact labels, behavior, and honesty

| Label in results | Test intention | Report only what is proved; never overclaim |
|---|---|---|
| **Internet Connection** | Test available local connectivity and manually initiated outbound reachability (e.g. public website such as Google); honor VPN/kill-switch policy, no hidden bypass | Public test destination failure ≠ internet definitively down; identify test destination on details |
| **DNS Resolution** | Test OS/current VPN resolver resolving a known public name and/or expected profile resolver | A failed DNS query may be blocked by valid kill-switch policy; explain rather than assume the OS is broken |
| **VPN Server** | Resolve/check selected stored endpoint where allowed, ICMP ping if possible, and eventually SVP handshake reachability when approved | ICMP timeouts **do not** prove server offline; UDP service often won't answer ping; use `Not Confirmed`/`No ICMP Response` where appropriate |
| **VPN Connection** | Check tunnel/interface, verified identity, actual packet traffic and DNS via authoritative status model | Only `Protected` when all required checks pass; not merely because adapter exists |
| **DNS Protection** | Check that configured DNS policy and expected tunnel resolver enforcement are active | Do not claim end-to-end privacy solely from config file |
| **Kill Switch** | Ask service for actual firewall rule status and blocking outcome where verifiable | If unable to verify, `Unknown`, not `On` |
| **Septuvo Service** | Check status socket, service version/protocol and responsiveness | Lost service is immediate distinct fault; never show stale last-good status |

Default behavior:

- On opening, show one primary **`Run Diagnostics`** command and a short explanation `Checks your internet connection, VPN server, and Septuvo protection.` No confusing advanced checklist required.
- Tests start only on user request (except non-network status already visible). For internet tests contacting a public service, show destination/consent information in nearby help; do not transmit diagnostics, IDs or logs off-device.
- Execute independent tests with bounded timeouts (suggested **5 seconds/test**, overall **30 seconds**); no permanent UI freeze. User can Cancel running checks; closing diagnostics never disconnects VPN.
- Result rows: label, icon AND textual status `Working`, `Problem`, `Not Available`, `Unknown`, or `Not Tested`; a one-sentence explanation and `What You Can Do` when actionable. Never display only a red/green dot.
- `Show Technical Details` expands raw status values, endpoint IP/port (masked appropriately in saved report if sensitive), timings and errors; collapsed by default. This is **the only advanced layer inside this diagnostics window**, not an Advanced main UI redesign.
- `Ping VPN Server` pings selected endpoint address where permitted; separate its result from an actual VPN handshake. `Ping Public Host` uses a documented public test target (e.g. Google) **only when requested**, respecting active kill-switch/routing rules and never creating an unauthorized network bypass. Report packet loss/timeout explicitly; if kill switch blocks public traffic, explain it may be expected.
- `Open Windows Network Settings` opens the OS-supported network settings/troubleshooter where available. Do not depend on a legacy troubleshooter existing on every Windows version; on macOS/Linux use OS-appropriate Network Settings or omit that platform-only action with explanation.
- `Save Diagnostic Report` invokes native Save As to a text file (`.txt`) with timestamp, software version, selected (nonsecret) settings as appropriate, seven labeled test outcomes and actionable errors. Preview/redact before save: **no private keys, enrollment secrets, tokens, unneeded personal user names or home paths**. No telemetry, silent upload, or external service calls.
- If no profile selected, Internet/DNS/service checks still work; VPN-specific tests say `No VPN Configuration Selected` rather than failing mysteriously.
- Service lost while diagnostics open must change display on next refresh; previous `Working` statuses cannot be presented as current proof.

### 5.2 Error copy examples (professional, concise)

- `Your internet connection appears to be working.`
- `Septuvo cannot contact the selected VPN server.` (only if connection attempt itself actually establishes that)
- `The server did not respond to ping. This does not always mean it is offline.`
- `The Septuvo service is unavailable. Check that the service is running.`
- `DNS requests are blocked by the current protection policy.`
- `The VPN connection could not be verified. Your connection is not marked Protected.`
- `Some tests could not run because the VPN is intentionally blocking traffic.`

Do not show stack traces as first-level UX; technical details available on request. Error names and help text must be proofread and correctly capitalized.

## 6. Settings specification — all approved/delegated entries

**Window:** `Settings`; initial **700 × 640 logical px**, min 520 × 480; scrollable; header `Settings`, top-right dialog Close. Section order EXACTLY below; no new Settings categories without owner approval. Each row min **56 px**, label + 1-line explanation, control on right (or below on narrow dialogs), section gap 20 px. Settings generally persist **when explicitly changed**; no unannounced connection resets.

### 6.1 General

| Exact label | Widget / options | Default | Behavior |
|---|---|---|---|
| **Interface Mode** | Two-choice selector: `Simple` / `Advanced` | **Simple** | Simple = owner's `Default_VPN_UI.png`. Advanced = separately approved Advanced UI. Switching saves choice immediately and **must never disconnect/reconnect tunnel**. Do not invent a third mode. |
| **Close Button Behavior** | Two-choice selector: `Exit and Disconnect` / `Send to System Tray` | **Exit and Disconnect** | Controls X specifically, per §3.7. Store across launches. If tray unsupported, tell user and refuse hiding; don't silently strand window. |
| **Start Septuvo with Windows** | Off / On toggle | **Off** | Launch GUI after user signs in when supported. Not the same as automatically connecting. On macOS/Linux adapt text to OS product name. Do not request privilege simply for GUI startup. |
| **Connect Automatically at Startup** | Off / On toggle | **Off** | When enabled and a complete previously selected profile exists, ask service to connect on supported startup path, with protection/permissions enforced. Explicit user disconnect and Exit-and-Disconnect must not be undone immediately. This is distinct from daemon `always_on` safety policy; don't conflate without design verification. |

### 6.2 Connection & Security

| Exact label | Widget / options | Default | Behavior |
|---|---|---|---|
| **Reconnect Automatically** | Off / On toggle | **On** | For **unexpected** loss of a previously explicitly established connection, retry safely subject to security model; never auto-reconnect after deliberate Disconnect, X Exit-and-Disconnect, or user refusal. Clearly report reconnection/failed state. |
| **Kill Switch** | **Read-only current status** with a short explanation | **Enforced** while protected/connecting, per backend | Current backend disallows a kill-switch-off configuration and removes rules only on deliberate disconnect. **Do not invent an Off toggle** or undermine Q2. Show verified status, not a fake switch. Any future disable option requires separate security and owner approval. |
| **Encryption Algorithm** | `Automatic (Recommended)` displayed (read-only until G1 design and approved options exist) | **Automatic (Recommended)** | Owner wanted option to select algorithm if supported. **Do not fabricate AES/ChaCha/ML-KEM menu choices or weaker downgrade** before G1. Only show selectable vetted suites after review, negotiation design and owner sign-off. ML-KEM is a KEM, not an interchangeable data-encryption algorithm. |

Connection-specific DNS, MTU, LAN access, allow/block routing and per-profile security details belong under **Connections → Edit → Advanced Configuration**. Don't duplicate different toggles across Settings and editor, causing inconsistent truth.

### 6.3 Updates

| Exact label | Widget / options | Default | Behavior |
|---|---|---|---|
| **Automatically Check for Updates** | Off / On toggle | **On** | When distribution/update support is released, check official signed/authenticated release metadata on app launch and at most every **24 hours**; do not treat arbitrary URLs/unverified GitHub releases as safe to install. |
| **Notify When an Update Is Available** | Off / On toggle | **On** | Nonintrusive notification with version and official release notes; no forced reconnection or changes to data. |
| **Check for Updates** | Manual action button | Available when update service exists | Users can always manually check. Show `Up to Date`, `Update Available`, or meaningful failure. No pretend successes before updater exists. |
| **Automatic Installation** | Off / On option **reserved/disabled until signed-update mechanism approved** | **Off** | The owner wants easy release updates. GitHub publishing **does not automatically push safe updates** to users. Only enable installation after release signing, verified artifacts, rollback, admin permissions and restart/disconnect policy are separately approved. Never silently interrupt an active protected tunnel. |

No background telemetry or analytics. Updater network check is a narrow explicitly explained function. Release channels, signatures, installer elevation, automatic update protocol and package verification are separate future approved work; don't invent them as part of N4.

### 6.4 About & Credits

- Section heading **About & Credits**; display official Septuvo VPN name, tagline `Security Without Compromise.`, project/source links from verified configuration, contributors/attributions and license notices required by bundled libraries, with no fabricated ownership or certification claims.
- **Actual application version at the absolute bottom** of Settings/About, in subtle muted Inter text (e.g. `Version X.Y.Z` **derived from build metadata**, never hardcode arbitrary 1.0.0); include channel `Development Preview` when applicable.
- Show official support/website only if valid approved URLs exist; do not invent a privacy policy, server fleet, app store links or support inbox.
- Settings bottom separator and About content must be scroll/reflow accessible at minimum dimensions; never overlap Save/close controls.

### 6.5 Setting persistence / correctness

- Persist Interface Mode and Close Button Behavior across restarts with per-user app preferences. Global application preferences are distinct from individual connection config and from root-owned service config.
- Display current actual service status rather than persisting `Protected = true` across restarts.
- If settings write fails, report it and retain prior behavior; never show toggled control as saved when it isn't.
- No privilege request in the unprivileged app merely to change GUI preferences; service-controlled values require the authenticated control path and actionable permission explanation.

## 7. Connection/session state machine and profile switching

### 7.1 Truthful main-screen states and copy

**Text is mandatory; do not infer protection from color alone.** The image illustrates the first row only.

| State from authoritative service | Status heading | Supporting copy / detail | Main action | Color / icon guidance |
|---|---|---|---|---|
| Full verified active tunnel | **Protected** | `Your connection is secure and private.` + `Your online activity is encrypted.` **ONLY if full tunnel / claims are actually true**; otherwise `Your selected networks are protected.` | **Disconnect** | Green `Connected for …`, approved checkmark shield |
| No tunnel, no pending connection | **Disconnected** | `Select a VPN connection and click Connect.` | **Connect** (when eligible) | Muted blue/neutral shield; NO green check or protection statement |
| Starting tunnel | **Connecting…** | `Connecting to <Connection Name>…` | Disabled/busy `Connecting…` | Warning/working symbol, no verified checkmark |
| Changing from one profile to another | **Switching Connection…** | `Disconnecting <Old> and connecting <New>…` | Disabled during critical transition | Working color, preserve kill-switch state |
| Ending current tunnel | **Disconnecting…** | `Safely ending your VPN connection…` | Disabled/busy `Disconnecting…` | No premature `Disconnected` before service confirmation |
| Attempt failed | **Connection Failed** | Short **verified** reason and `Open Help & Diagnostics for more information.` | **Connect** (retry) | Error indicator, no checked shield |
| Service unreachable | **Service Unavailable** | `Septuvo cannot communicate with its background service.` | Disabled, diagnostic route | Error, no stale last-known protected state |
| Firewall blocks traffic / no verified tunnel | **Traffic Protected by Kill Switch** ONLY when blocking actually verified | `Internet access is restricted while the VPN is unavailable.` | Connect/Retry only when safe | Warning (not green connected state); avoid claiming tunnel protected |
| No saved profiles | **No Connections** | `Add or import a VPN configuration to get started.` | Disabled **Connect** | Neutral, no imaginary Home VPN |
| Incomplete draft selected | **Setup Required** | `Complete this configuration before connecting.` | Disabled **Connect** | Warning, route to Connections → Edit |
| Current pre-G1 test/lab software | **VPN Not Yet Available** | `This development build does not establish an encrypted VPN tunnel.` | Disabled or lab-only control explicitly labeled for developer testing | Neutral/warning; **never** the artwork's “Protected” claim |

Use the exact existing **positions** of icon, heading, copy, timer, and button for every status. No pop-up dashboard, no extra cards on failure. If an explanation is longer, wrap up to two lines then use Diagnostics; scroll as needed. Backend source-of-truth four checks take precedence over mockup artwork.

**Duration line:** show `Connected for HH:MM:SS` only once verified protected; use no duration line or succinct `Not connected` otherwise, at same reserved location to avoid layout jumping. No fake `00:12:34` on launch.

### 7.2 User actions (deterministic transitions)

| User action | Expected behavior |
|---|---|
| Select another profile **while disconnected** | Change selection only, persist, **no automatic connect** |
| Select a DIFFERENT profile **while connected** | **Automatic safe switch**: disconnect active → retain appropriate fail-closed protection → connect new → report success/failure. No extra Connect click. |
| Select the same active profile | Do nothing; do not restart tunnel or reset timer |
| Click Connect | Validate selected complete profile and authorization; initiate connection; show Connecting; on success show Protected ONLY after all proofs pass |
| Click Disconnect | Send deliberate disconnect; confirm service result; show Disconnected; no automatic reconnect caused by `Reconnect Automatically` |
| Click X with default behavior | Deliberate Disconnect (if needed) then **exit**; failure keeps UI open with error and protection intact |
| Click X with tray option | Hide main window to tray; **no disconnect**, no new connection |
| Minimize or Maximize | Change window visibility/size only; **no** network action |
| Switch Simple ↔ Advanced | Change presentation only; **no** network action and no timer reset |
| Open/close any secondary window | Never changes connection state by itself |
| Save or import a profile | No surprise immediate connection; user chooses or switches explicitly after profile has valid enrollment |
| Update becomes available | Inform user; **do not** restart/disconnect automatically merely to install it |

**Profile-switch failure specifics:** service checks permissions/config before trying; new profile must be usable before tearing down old whenever possible, but user’s chosen behavior remains automatic switch. On failure keep the new selection, old session disconnected if its disconnect succeeded, and kill switch fail-closed. Do not silently connect to fallback VPN or permit plain traffic. If old disconnect itself fails, do not start conflicting new session; surface error and keep truthful active identity.

## 8. Security, credentials and sharing boundaries

- Importing a profile is NOT enrollment and doesn't establish server identity automatically. An invitation or QR flow requires separate approved, authenticated SVP design after G1.
- Do not conflate **saved configuration**, **per-device key/identity**, **server access authorization**, **active VPN tunnel**, and **exported nonsecret backup**.
- Server/client direct peer connection is **not** automatically part of Connections, Add/Import, or this UI. No peer invitation generator or direct-connect protocol without authorization.
- Never store private keys or authentication tokens in plain exported config; never display them in UI/status/diagnostic reports. Future credential storage design belongs to independent security review and platform services.
- No user-facing dropdown for arbitrary encryption primitives/cipher suites while G1 is pending. No protocol or cryptographic implementation is authorized by this specification.
- The UI must not request elevation or modify OS firewall/DNS/routes itself. Use privileged service IPC with correct authorization and atomic/fail-closed operations.
- A failed operation must never silently disable protection, permit DNS escape, or inaccurately show green status. Treat interrupted IPC responses as unknown until refreshed from service.
- Only permitted, explicitly approved network probes/update checks may leave the device. No telemetry or hidden report uploads.

## 9. Accessibility, keyboard, mouse, and quality requirements

### 9.1 Keyboard tab order (main window)

When the application uses custom controls, focus should proceed: profile dropdown → main Connect/Disconnect button → Connections → Add / Import → Help & Diagnostics → Settings cog → normal OS window frame controls according to OS convention. On small windows, scroll focused control into view. On Windows standard caption buttons may be outside app tab order; respect OS conventions.

- Tab / Shift+Tab move focus. Enter/Space activates focused actions. Alt+F4 is handled like main-window X (owner-approved close policy); don't leave tunnel connected when Exit-and-Disconnect is configured.
- Open dropdown: Up/Down moves entries, Enter selects and triggers active-switch behavior if appropriate, Escape closes without changes. Dropdown has type-ahead searching where supported without inserting a new search bar into the main image.
- Overflow menus: keyboard-operable, Escape closes, deletion confirmation keyboard accessible. Dialog Escape closes only when it won't discard uncommitted edits without warning.
- All icons have text labels or accessibility names; shield decorative artwork is not read as a separate meaningless button.
- Support screen readers via platform accessibility APIs, OS text size preferences, mouse/keyboard, reduced motion and high-DPI displays.
- Color alone never communicates status; use text. All disabled buttons expose why they're disabled; tooltips supplement but do not replace visible helper text.

### 9.2 Consistent click/animation behavior

- Hover and press feedback subtle (existing brand blue/electric accent), no surprise navigation on hover, no popups obscuring primary action.
- Spinner/connecting transitions may appear only during real asynchronous work and must stop when finished or timed out; never endless falsely reassuring animation.
- IPC calls/diagnostics/file picker operations must not block UI thread. Refresh live status every **1–2 seconds** where supported, preserving scroll location; clear stale status immediately on service loss. Connection duration can tick locally **only while** verified state persists, revalidated by service.
- App remembers selected profile, Interface Mode, X preference and restored window bounds. It must never remember a stale `Protected` flag as authoritative state.
- Feedback on saves/imports/renames/exports is concise and truthful; any errors include actionable fix; no modal confirmation for routine profile selection/switching or ordinary Connect/Disconnect unless a real destructive/security exception demands it.

### 9.3 Quality gate / visual review before merge

All agents must demonstrate these verifiable acceptance cases; a green build or screenshot alone is insufficient:

1. **Visual fidelity:** show screenshots compared side by side to the owner's `Default_VPN_UI.png`; header brand, selector, central shield/globe, protected-state hierarchy, primary button, divider, three footer tiles, Settings cog and caption buttons match. Do not draw a different UI and call it inspired.
2. **Size responsiveness:** capture at **780×760**, **520×580**, **1280×800**, maximized on a 1920×1080 screen, and 150% DPI. Footer buttons always reachable; narrow layout scrolls; no offscreen dropdown/menu/dialog; no unexpected scroll reset.
3. **Exact labels:** spelling/capitalization/spacing audited; `Manage Saved Connections` replaces old image subtitle as owner-approved; no `VPN servers` text in the default tile.
4. **Empty profile:** no default fake connection, Connect disabled, Add / Import accessible.
5. **Dropdown:** all saved named profiles visible, scroll >6, duplicate name refusal, selection persistence.
6. **Switching:** when connected, selecting different profile disconnects old then attempts new, no second click and no unprotected window; same-profile selection no operation; new failure behaves as §7.
7. **Connections menu:** exactly **Edit / Rename / Export / Duplicate / Delete**, in order; edit safe; duplicate strips secrets; delete confirms; import/export changes reflected instantly.
8. **Editor:** blank draft allowed; existing values prefilled; basic form does not show advanced fields unless expanded; strict validation; bad changes preserve prior config/protection.
9. **File pickers:** native Open for Import and native Save As for Export; cancel changes nothing; duplicate-name and overwrite handling; no export of private secrets.
10. **Diagnostics:** Run Diagnostics plain-language outcomes, all seven checks, optional technical details, public-ping caveat, redact/preview Save Diagnostic Report, no upload.
11. **Settings:** all exact entries/sections/defaults match §6; Simple initially; mode switch never disconnects; Settings persists across app restart.
12. **X/tray:** default X / Alt+F4 actually disconnect + exits; opted-in X hides + keeps connected; minimize just minimizes; explicit tray Exit disconnects; failures never silently leave unintended connection.
13. **Truthful states:** service outage, partial handshake, DNS failure, fail-closed block never display `Protected`; screenshot's green sample only after authoritative checks.
14. **Security and licensing:** no pre-G1 crypto, credentials leak, unaudited dependency, unexpected network or system permission actions; don't claim a functional VPN in a lab-only preview.
15. **Accessibility:** all 7 primary main areas keyboard-reachable; screen-reader labels, color contrast, text scaling and dialog behavior verified.

Agents should record screenshots/test evidence per work-order policy, report deviations rather than fix by unauthorized redesign, and leave **merge/push decisions with the owner**. This file is not permission to push or merge.

## 10. No-assumptions / escalation list for agents

The following are **genuine future technical/security dependencies**, not permission for an agent to make arbitrary UI choices:

1. **Advanced UI:** separate owner-approved visual and functional spec required. Do not invent its layout from general VPN trends.
2. **Artwork extraction:** owner screenshot exists but layered production assets for its globe/ring may not; request an approved source/faithful rendering plan if necessary; no unrelated stock art.
3. **SVP enrollment/invitations/QR/trusted key distribution:** not defined before G1; don't invent and do not treat exported backup as device invitation.
4. **Cryptographic algorithms:** owner requested a future selection capability, but actual vetted suites cannot be decided until the protocol review. Disabled/default-only until approval.
5. **Hostnames:** current config requires IP:port, not `vpn.example.com`; implementing hostname resolution is a separate network/security design decision.
6. **Programmatic status proof:** use real service checks, not a screen-stub declaring connected. No real encryption before G1.
7. **Safe active edit/profile switching:** service may need atomic config/policy application support; if missing, implement/test it under an authorized work order or stop rather than compromising protection.
8. **Signed auto-update delivery:** checking/notification is approved UX, but update trust, signatures, installer restart policy, channels and rollbacks require their own implementation approval. Don't treat GitHub releases as auto-pushed installers.
9. **Platform integration:** Windows main-window behavior specified; macOS/Linux must preserve design intent and use native OS controls, with specific platform adaptation approved as needed.
10. **When in doubt:** a missing backend feature is NOT permission to add a new control, change approved screen layout, fabricate network status or ship unsafe temporary behavior. Raise a concise owner question referencing this section.

**Owner change-control reminder:** No assets, docs, implementation, PR, branch write, release, or GitHub push is authorized by this document alone. The owner will place this Markdown file and `Default_VPN_UI.png` in the repository only after deciding to publish the design requirements. Agents MUST obtain normal work-order authority before acting.
