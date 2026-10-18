# Septuvo VPN — Advanced Desktop UI: Binding Owner Specification

**Document ID:** `SEPTUVO-UI-ADVANCED-001`  
**Document version:** `1.1 — reconciled with owner decisions D-037–D-042 (2026-10-18)`  
**Prepared:** 2026-10-18  
**Product:** Septuvo VPN desktop client, **Advanced interface**  
**Canonical repository destination:** `docs/ui/desktop/ADVANCED-UI.md`  
**Canonical image (provided separately by owner):** `docs/ui/desktop/Advanced_VPN_UI.png`  
**Companion default UI specification:** `docs/ui/desktop/DEFAULT-UI.md`  
**Governance:** This file captures the owner-approved Advanced UI image, approved visible components, the approved Home tile behavior, the clarification that **detailed Secure DNS configuration exists only in Settings**, and the shared behavior inherited from the already approved Default UI where explicitly stated. **Version 1.1** applies the owner's final decisions of 2026-10-18: the **Kill Switch** and **DNS Protection** tiles are truthful **status indicators, not switches**; **Local Network Access** stays an interactive control; Lockdown Mode; one shared Settings structure; window, tray and profile-switching behaviour; cross-platform targets; updates. Every change is listed in §19 and in [`desktop-ui-reconciliation.md`](../../reviews/desktop-ui-reconciliation.md). The approved main-window design is unchanged.

> **HARD RULE FOR ALL AGENTS:** The owner-approved Advanced UI screenshot is the only authorized Advanced main-window visual design. DO NOT substitute another mockup, DO NOT invent a different navigation system, DO NOT add extra tabs, DO NOT replace the hero layout, DO NOT move the status tiles to a sidebar, DO NOT create a separate DNS editor pop-up from the Home screen, and DO NOT redesign this into a different product. This document is an implementation contract. If behavior is not explicitly authorized here, stop and ask the owner instead of guessing.

---

## 0. Purpose and approval scope

This file defines the **Advanced interface mode** for the Septuvo VPN desktop client.

It exists because the owner approved two separate main-window approaches:

- **Default / Simple mode** → compact, low-complexity, “grandma friendly” main screen
- **Advanced mode** → more information and more direct controls on the Home screen for power users

The Advanced mode is **not** a separate application. It is the same desktop client with a different interface mode selected in Settings.

### 0.1 What this file covers

This file covers:

- the approved Advanced main-window layout
- exact function of every visible element
- top navigation behavior
- Home page behavior
- Connect / Disconnect behavior
- behavior of the Home tiles: Kill Switch and DNS Protection (status indicators) and Local Network Access (control)
- which functions are shared with the Default UI
- what appears in Settings in Advanced mode
- the critical clarification that **all Secure DNS editing lives inside Settings only**
- sizing, spacing, window behavior, and resizing expectations
- exact capitalization and naming rules
- constraints to prevent agents from improvising functionality or UI

### 0.2 What this file does not authorize

This file does **not** authorize:

- redesigning the main Approved Advanced UI image
- adding extra Home tiles beyond the approved visible ones
- moving the Home tiles into a different structure
- making DNS editing happen directly inside the Home tile
- inventing unapproved profile sharing workflows
- inventing a server admin GUI
- inventing security claims not yet supported by actual validated implementation state
- claiming a real secure SVP tunnel before the security-review gates allow it

### 0.3 Source-of-truth order

1. The owner’s later explicit decisions override earlier chat suggestions.
2. This file controls the **Advanced** desktop UI.
3. `Advanced_VPN_UI.png` controls the approved Advanced main-window appearance.
4. `DEFAULT-UI.md` remains authoritative for shared behavior unless this file overrides it.
5. Recorded owner decisions **D-037 to D-042** (`docs/DECISIONS.md`) prevail over any earlier wording in either file.
6. Existing repository rules (`AGENTS.md`, decisions, security model, gate rules, licensing boundaries) remain mandatory and cannot be weakened by UI work.

---

## 1. Relationship to the Default UI

Advanced mode is intentionally related to the Default UI.

### 1.1 Same product, different presentation density

The owner-approved interpretation is:

- **Default UI** = fewer visible controls on the main screen; lower cognitive load
- **Advanced UI** = more visible status, more quick controls, more direct access to settings that advanced users care about

Advanced mode must **not** become a separate product or a totally different visual language.

### 1.2 Shared functional areas

The following functions are explicitly shared between the Default UI and Advanced UI:

- **Connections** → same connection-management behavior
- **Settings** → same overall settings area, but Advanced mode reveals more options directly
- **Diagnostics** → same diagnostics area and functions
- **Connect / Disconnect** → same core behavior
- **X button behavior** → same default exit/disconnect vs send-to-tray behavior
- **Import / Export** behavior → same native desktop file-dialog behavior
- **Profile switching** → same disconnect-then-connect behavior

### 1.3 Advanced-only presentation differences

Compared with Default mode, the Advanced Home screen adds or exposes:

- top navigation tabs
- a larger information-rich Home panel
- a row of three tiles:
  - **Kill Switch** — truthful status indicator (not a switch)
  - **DNS Protection** — truthful status indicator (not a switch)
  - **Local Network Access** — interactive control (Allowed / Not Allowed)
- visible region/location card on the Home screen
- visible current public IP on the Home screen

---

## 2. Approved visible structure

The approved Advanced UI image shows the following top-level visible areas.

### 2.1 Exact visible elements on the Advanced main window

| # | Visible element | Mandatory label / purpose |
|---|---|---|
| 1 | Header brand area | Official **Septuvo VPN** brand lockup |
| 2 | Top navigation | **Home**, **Connections**, **Settings**, **Diagnostics** |
| 3 | Native-style caption controls | Minimize, Maximize/Restore, Exit |
| 4 | Main Home hero panel | Status headline, supporting text, Disconnect/Connect action, current connection summary, hero globe graphic |
| 5 | Status and control tiles row | **Kill Switch** (status), **DNS Protection** (status), **Local Network Access** (control) |

No additional top-level navigation items are approved.

### 2.2 Capitalization rules

All labels must use professional capitalization exactly as follows:

- **Home**
- **Connections**
- **Settings**
- **Diagnostics**
- **Connect**
- **Disconnect**
- **Kill Switch**
- **DNS Protection**
- **Local Network Access**
- **Allowed**
- **Not Allowed**
- **On**
- **Off**
- **Your New IP Address**
- **Unknown**
- **Failed**
- **Internet Blocked**

Do not use childish, inconsistent, or casual capitalization such as:

- `Kill switch`
- `Dns protected`
- `local network access`
- `you’re connected` in a button-like label area

Sentence-style descriptive text may be sentence case, but headings and labels must remain clean and professional.

---

## 3. Visual and brand contract

The Advanced UI uses the same Septuvo brand system as the Default UI.

### 3.1 Brand assets

Use only approved assets from the repository brand kit, especially:

- official Septuvo VPN lockup
- approved blue/electric-blue VPN color family
- approved icons and tokens
- approved Inter typography

### 3.2 Color role expectations

Advanced mode remains part of the **VPN** product family, so blue is the primary accent.

Use the same semantic brand roles as the Default UI:

- dark midnight / navy backgrounds
- blue primary actions
- electric-blue highlights
- white primary text
- muted secondary text
- green only for truthful positive states
- yellow for warning / in-progress / limited states
- red for failures and destructive warnings

Do not switch to the Protocol teal or Server purple as the main Advanced UI accent.

### 3.3 Typography

Use **Inter** consistently.

Recommended desktop sizes:

- main Home headline: **46–54 px Bold**
- supporting body text: **16–18 px Regular**
- top nav labels: **15–17 px Medium**
- primary action button text: **20–24 px SemiBold**
- tile labels: **16–18 px SemiBold**
- tile value text (`On`, `Off`, `Allowed`, `Not Allowed`): **16–18 px SemiBold**
- helper text under tiles: **13–15 px Regular**
- location card title: **14–16 px Regular**
- country / city display: **18–22 px SemiBold**
- IP address: **16–18 px Medium**

### 3.4 Rendering rules

- Maintain the dark blue premium visual language from the approved image.
- Do not flatten the UI into a bland utilitarian panel.
- Do not add visual clutter, dense data tables, or admin-dashboard styling.
- Home must remain clean and readable first, informative second.
- Quick-tile controls must look like polished product controls, not raw developer widgets.

---

## 4. Window sizing and layout contract

### 4.1 Reference-image note

The owner-approved Advanced image is a rendered design reference, not an engineering blueprint. Dimensions below are derived implementation targets that preserve the approved composition.

### 4.2 Baseline window targets

| Item | Runtime specification |
|---|---|
| Initial Advanced window size | **980 × 640 logical px** |
| Minimum usable window size | **760 × 560 logical px** |
| Preferred larger working size | **1080 × 700 logical px** |
| Saved size / position | Remember last size and position |
| Maximize behavior | Standard OS maximize / restore |
| Oversized monitors | Center content, do not stretch it awkwardly |

Advanced mode is intentionally wider than the Default mode and should feel like a modern compact desktop app, not a giant enterprise dashboard.

### 4.3 Main layout zones

The window is divided into these major vertical sections:

1. **Header row**
   - logo / brand on left
   - navigation in upper center
   - caption controls on right

2. **Main Home content panel**
   - large rounded panel occupying most of the central width
   - left side: status headline, supporting copy, action button, location card, IP card
   - right side: decorative globe and shield illustration

3. **Bottom status and control tiles row**
   - three horizontally aligned tiles:
     - Kill Switch
     - DNS Protection
     - Local Network Access

### 4.4 Baseline measurements (logical px)

These are target implementation sizes, not optional suggestions.

| Element | Target size / position |
|---|---|
| Outer window corner radius | 16–18 px visual treatment |
| Header row height | 72–84 px |
| Brand area width | approx. 250–290 px |
| Navigation item spacing | 30–42 px between items |
| Main Home panel | approx. 88–92% of available content width; height 360–400 px |
| Main action button | approx. 300–360 px wide; 60–74 px high |
| Location card | approx. 250–300 px wide; 74–88 px high |
| IP card | approx. 250–300 px wide; 58–70 px high |
| Bottom status tile row height | 120–150 px |
| Bottom tile width | three equal-width tiles with 16–24 px interior padding |
| Main content padding | 24–32 px |
| Tile gap | 0 or thin divider treatment as shown in approved image |

### 4.5 Resizing rules

- If the window shrinks, the main Home panel may contract, but all primary controls must remain visible.
- Below the comfortable width, content may reflow vertically **without changing the identity of the approved layout**.
- If needed, allow internal scrolling rather than clipping critical content.
- The bottom tiles must remain readable and clickable.
- Do not allow the Disconnect/Connect button to overlap cards or hero artwork.

---

## 5. Header and top navigation

### 5.1 Brand area

Top-left must show:

- Septuvo brand symbol
- **Septuvo VPN** lockup

This is not clickable for navigation unless explicitly approved later.

### 5.2 Navigation items

Top navigation labels are exactly:

- **Home**
- **Connections**
- **Settings**
- **Diagnostics**

### 5.3 Navigation behavior

#### Home
Returns to the main Advanced dashboard.

#### Connections
Opens the saved-connections management area.

#### Settings
Opens the shared settings area.

#### Diagnostics
Opens the diagnostics and troubleshooting area.

### 5.4 Selected-tab treatment

The currently selected page must be visually indicated.

For the Home state shown in the approved image:

- Home icon and label are highlighted
- a blue underline or active indicator is visible beneath Home

Do not use a completely different active-tab style than the approved image without explicit approval.

---

## 6. Home screen behavior

The **Home** page is the main Advanced dashboard.

### 6.1 Main status headline

The approved image shows:

- **You’re Connected**

This is a state-driven heading.

Examples of equivalent heading states:

- **Disconnected**
- **Connecting**
- **Disconnecting**
- **Connection Lost**
- **Reconnecting**
- **Limited Protection** (only if factually true and needed)
- **Internet Blocked** (Lockdown Mode on and not connected; D-038)
- **VPN Not Yet Available** (pre-G1 development builds; DEFAULT-UI §7.1)

Status headlines must always be truthful. The approved image's **You’re Connected** appears only when all four checks pass (D-020); before G1 it never appears.

### 6.2 Supporting text

Under the headline is explanatory status text.

Connected example:

> Your internet is secure, private, and protected with Septuvo VPN.

Disconnected example:

> Your VPN is not connected. Connect to protect your internet traffic.

Connecting example:

> Establishing a secure connection. Please wait.

### 6.3 Primary action button

This is the main action control.

States:

- disconnected → **Connect**
- connected → **Disconnect**
- connecting → **Connecting** (disabled or spinner state)
- disconnecting → **Disconnecting** (disabled)

Button behavior:

- acts on the currently selected/active profile
- must not silently switch profiles without user intent
- must give immediate visual feedback
- must not claim connected before the service can truthfully do so

### 6.4 Current connection summary

The connected Home view includes a location / endpoint summary card.

The approved image visually shows **sample content**: a United States flag, **United States** and **New York**. The card, its position and its layout are approved; its content must be **truthful**.

This area represents the currently active profile, or the selected one when disconnected.

**Content rules (reconciled 2026-10-18; owner to confirm, see the reconciliation record):**
- **First line:** the connection's friendly name.
- **Second line:** its server address.
- **Icon:** the neutral connection icon (DEFAULT-UI §3.1). **No country, city or flag is inferred** from names or addresses.
- A country/city line and flag appear only once a trustworthy source exists: a location the user's own Septuvo Server reports over the verified tunnel (after G1), or a location label the owner approves adding to connections.

**Behavior:**
- Clicking the card opens **Connections** with this connection selected. This is the one click behavior; it is the same everywhere.
- No geolocation lookup, no third-party service.

### 6.5 Public IP section

The approved image shows:

- label: **Your New IP Address**
- value: example IPv4 address
- copy icon on the right

**Source rule (reconciled 2026-10-18):** the address comes **only from the user's own Septuvo Server over the verified tunnel** (a protocol message defined in M2 and available after G1). Septuvo never asks a third-party "what is my IP" service for it unless the owner separately approves one (LO-2, D-017).

**Behavior:**

- if connected and the server has reported the exit address, show it
- clicking the copy icon copies the displayed IP to clipboard
- otherwise show `Not Connected` (disconnected) or `Not Available` (connected, address not reported), never sample or stale data

### 6.6 Hero globe / shield art

The right side hero artwork is decorative and informative.

Rules:

- it must not become an extra control unless explicitly approved later
- it must not display misleading technical values
- it must not replace accessible text
- it must visually reinforce status only

---

## 7. Connections

The owner approved that **Connections** in Advanced mode is the same feature area as in the Default UI.

### 7.1 Display name and purpose

Recommended page heading:

**Connections**  
Subheading: **Manage Saved Connections**

Purpose:

> View, organize, edit, and manage your existing VPN connections.

This wording is owner-approved.

### 7.2 What appears in Connections

The page/window must show a list of saved connections.

Each saved connection row should show at minimum:

- friendly connection name
- optional server/location summary
- optional last-used indicator
- optional currently active indicator
- three-dot menu on the right

### 7.3 Three-dot menu actions

The owner approved these exact actions:

- **Edit**
- **Rename**
- **Export**
- **Duplicate**
- **Delete**

### 7.4 Action definitions

#### Edit
Change the saved configuration, including endpoint, routing, DNS, and other permitted fields.

#### Rename
Change the user-visible friendly name.

#### Export
Save a configuration backup using a native file-save dialog, with safe handling of sensitive information.

#### Duplicate
Create another draft based on the connection without improperly copying protected identities or credentials.

#### Delete
Remove the saved configuration after confirmation.

### 7.5 Shared behavior with Default UI

All connection-list operations, import/export behavior, draft creation, and edit flows are shared with the Default UI unless explicitly changed later.

---

## 8. Settings

The owner approved that **Settings** exists in both modes and that Advanced mode may reveal more detail.

### 8.1 Overall role

Settings is where:

- interface mode is chosen
- connection behavior defaults are managed
- close/tray behavior is managed
- version and credits are shown
- detailed DNS settings are configured
- advanced security/network options are exposed

### 8.2 One shared Settings structure (D-037)

Settings are **the same in both modes**; their complete specification is [`DEFAULT-UI.md`](DEFAULT-UI.md) §6. Advanced mode opens them from the **Settings** tab instead of a separate window, and shows the rows marked *Advanced mode*. The sections, in order:

1. **General** — Start Septuvo with Windows (Linux/macOS: Start Septuvo at Login), Connect Automatically at Startup
2. **Interface** — Interface Mode (Simple / Advanced), Close Button Behavior (Exit and Disconnect / Send to System Tray), Show Notifications
3. **Connection & Security** — Reconnect Automatically, Kill Switch (read-only status), **Lockdown Mode**, Encryption Algorithm (read-only until G1); *Advanced mode:* Connection Timeout and Retry Behavior once the connection engine exists
4. **DNS Protection** — DNS Mode (Automatic, Standard DNS, DNS over HTTPS, DNS over TLS), Primary and Secondary DNS Server, Advanced DNS Settings (Bootstrap DNS, Certificate Validation, Fallback Behavior, Local DNS Exceptions, Per-Connection Overrides), Test DNS Settings
5. **Network** — Local Network Access (the same setting as the Home tile)
6. **Updates** — Automatically Check for Updates, Notify When an Update Is Available, Check for Updates, Automatic Installation (disabled until signed updates exist)
7. **About & Credits** — version, build, credits, licences, project links; version also at the bottom of Settings

### 8.3 What changed from version 1.0 of this section

These rows were removed or merged so that each function has one label and one place (reconciliation C7, C8):
- **Kill Switch default behavior** and **DNS Protection On / Off**: removed. Both protections are mandatory (D-038, D-039).
- **Send to Tray** and **Minimize to Tray When Closed**: replaced by **Close Button Behavior → Send to System Tray**.
- **Launch at Sign-In**: the same as **Start Septuvo with Windows / at Login**.
- **Check for Updates Now** → **Check for Updates**; **Notify When Updates Are Available** → **Notify When an Update Is Available**; **Default Encryption Algorithm** → **Encryption Algorithm**.
- **Remember Last Used Connection** and **Use Last Connected Profile Automatically**: the last selection is always remembered; automatic connection is **Connect Automatically at Startup**.
- **Profile Switching Behavior**: fixed by the owner's decision (§10.2); not a setting.
- **Route All Traffic Through VPN**: the per-connection **Routing Mode** in the connection editor.

### 8.4 DNS Protection

> **Owner-approved rule:** Secure DNS settings are in the **Settings menu only** (Settings → DNS Protection, D-039).

That means:

- the Home **DNS Protection** tile is a **status indicator only**: no toggle, no editor, no dropdown, no pop-up
- detailed DNS editing does **not** happen in a separate DNS pop-up invented by agents unless the owner later approves one
- DNS leak prevention is mandatory during protected connections; there is **no DNS Protection On / Off** anywhere

The capabilities (DNS Mode with Automatic, Standard DNS, DoH and DoT; primary, secondary and bootstrap servers; certificate validation; fallback; global settings and per-connection overrides; testing) are specified in `DEFAULT-UI.md` §6.4. DoH and DoT are shown as `Not available yet` until implemented.

### 8.5 Updates

The owner explicitly requested update-checking capability (D-042). Specification: `DEFAULT-UI.md` §6.6. A GitHub release is not a secure update mechanism; automatic installation stays disabled until signed updates exist.

## 9. Diagnostics

The owner approved Diagnostics in Advanced mode as the same functional area as in Default mode.

### 9.1 Purpose

Diagnostics provides help and basic troubleshooting for normal users.

It must remain human-readable and not become a developer-only dump.

### 9.2 Required diagnostics capabilities

At minimum, Diagnostics should support:

- checking reachability of the selected configured server
- checking general internet reachability
- checking Septuvo service status
- checking VPN connection state
- checking DNS resolution behavior
- checking DNS Protection status
- checking Kill Switch status
- opening or referencing the operating system’s network troubleshooting tools if appropriate

### 9.3 User-facing test examples

Examples that fit the owner’s intent:

- **Test Connection to Selected Server**
- **Test Internet Connectivity**
- **Check DNS Resolution**
- **Check Septuvo Service Status**
- **Open Network Troubleshooter**
- **Save Diagnostic Report**

### 9.4 Reporting style

Every result must clearly say:

- what was checked
- whether it passed or failed
- what it means
- what the user should do next if it failed

---

## 10. Connect / Disconnect behavior

The owner explicitly approved that this is the same function as in the Default UI.

### 10.1 Action behavior

- **Connect** connects using the currently selected profile.
- **Disconnect** disconnects the current VPN tunnel.
- During active transitions, the control must visibly show connecting/disconnecting state.

### 10.2 Profile switching while connected

This behavior is owner-approved and must be documented exactly:

If the user selects a different saved connection while already connected:

1. disconnect the current connection
2. switch to the newly selected profile
3. connect to the new profile

This is standard modern VPN behavior and is mandatory unless the owner later changes it.

### 10.3 Truthfulness rule

The UI must not claim connected, protected, or secure unless that state is actually true according to the authoritative service.

---

## 11. Status and control tiles on the Advanced Home screen

The Advanced Home screen includes three tiles. They are part of what distinguishes Advanced mode from Default mode. **Two are truthful status indicators and one is a control** (D-038, D-039, D-040). Every value comes from the service; the tiles never assume.

---

### 11.1 Kill Switch tile (status indicator)

#### Label
**Kill Switch**

#### Value states (from the service)
- **On** — the kill-switch rules are installed and verified (connecting, connected, reconnecting, switching, or Lockdown Mode blocking)
- **Off** — no connection is active and Lockdown Mode is off, so nothing is being enforced
- **Unknown** — the service cannot be reached or cannot verify the rules
- **Failed** — enforcement was expected but the rules are missing or wrong (error colour)

#### Supporting text
- On: `Stops internet if the VPN connection drops.`
- Off: `Turns on automatically when you connect.`
- Unknown: `Septuvo cannot check the kill switch right now.`
- Failed: `Protection could not be enforced. Open Diagnostics.`

#### Behavior
- **Not a switch.** The kill switch is mandatory and cannot be disabled during a protected connection (D-038).
- The tile has **no click action**: it is not focusable as a button and never changes protection. Lockdown Mode is set in **Settings → Connection & Security**.
- It never shows **On** unless enforcement is verified.

---

### 11.2 DNS Protection tile (status indicator)

#### Label
**DNS Protection**

#### Value states (from the service)
- **On** — DNS leak prevention is enforced and verified: DNS can reach only the permitted resolvers through the permitted path
- **Off** — no connection is active, so DNS is not being managed by Septuvo
- **Unknown** — the service cannot be reached or cannot verify it
- **Failed** — enforcement was expected but is not in place (error colour)

#### Supporting text (truthful; encrypted DNS transport is not leak prevention)
- On, with the verified tunnel or verified DoH/DoT carrying the queries: `Your DNS queries are encrypted and private.`
- On, otherwise: `DNS queries can only go to your approved resolvers.`
- Off: `Turns on automatically when you connect.`
- Unknown / Failed: as the Kill Switch tile, naming DNS

#### Behavior
- **Not a switch.** DNS leak prevention is mandatory during protected connections (D-039).
- This tile has **no click action**: it does **not** open a DNS editor, a dropdown or any configuration window, and contains no DNS settings.

#### Critical owner-approved rule
All Secure DNS configuration details live in:

**Settings → DNS Protection**

Nowhere else unless separately approved later.

---

### 11.3 Local Network Access tile (control)

#### Label
**Local Network Access**

#### Value states
- **Allowed**
- **Not Allowed** (default)

#### Supporting text
- Allowed: `You can access devices on your local network.`
- Not Allowed: `Devices on your local network are blocked while you are protected.`

#### Behavior
- clicking toggles Allowed / Not Allowed; it is the same setting as **Settings → Network → Local Network Access**
- the change is sent to the service, which applies it safely (immediately when connected; for the next connection otherwise) and reports the result; the tile shows the **service's** value, never an assumed one
- **Allowed** adds only explicit, restricted exceptions: the directly attached local subnets, computed and size-capped by the service, never overlapping the tunnel's networks; local discovery (mDNS, broadcast) stays blocked; server-side authorization is unchanged (D-040)
- if the service refuses or fails, the tile keeps the previous value and says why

#### Warning guidance
Turning it to **Allowed** shows one short note: `Devices on this network will be able to reach you, and you them, outside the VPN.` It must not become noisy.

---

## 12. Window controls and tray behavior

The owner explicitly approved the following behaviors.

### 12.1 Minimize (Dash)

- minimizes the application window to the taskbar / dock as appropriate for the OS
- does not disconnect the VPN

### 12.2 Maximize / Restore (Square)

- maximizes or restores the window
- standard desktop behavior

### 12.3 Exit (X)

Default behavior:

- clicking **X** exits the app
- clicking **X** disconnects the tunnel

If the user has chosen **Close Button Behavior → Send to System Tray** in Settings:

- clicking **X** sends the UI to the system tray (menu bar on macOS) instead of fully exiting
- the VPN connection remains active

This behavior must match the Default UI (DEFAULT-UI §3.7, D-041), including: the GUI ending for any other reason (crash, kill, logout) changes nothing in the service; with Lockdown Mode on, blocking continues after a disconnect or exit.

---

## 13. State handling

The Home screen must remain truthful and coherent in every state.

### 13.1 Connected

Expected elements:

- headline indicates connected
- action button says Disconnect
- location summary populated
- public IP populated if available
- tiles show the service's verified values

### 13.2 Disconnected

Expected elements:

- headline indicates disconnected
- action button says Connect
- current profile/target information may still be shown
- IP may be hidden or marked unavailable / not connected

### 13.3 Connecting

Expected elements:

- headline indicates connecting
- action button disabled or shows connecting state
- progress messaging visible

### 13.4 Disconnecting

Expected elements:

- headline indicates disconnecting
- action button disabled or shows disconnecting state

### 13.5 Error / failure

Expected elements:

- headline indicates failure or lost connection
- explanatory text visible
- recovery path visible (retry, open diagnostics, etc.)

### 13.6 Reconnecting

Expected elements:

- headline indicates reconnecting
- action button should not misrepresent state

---

## 14. Accessibility and usability requirements

### 14.1 General

- all controls must be keyboard reachable
- focus order must be logical
- tab order must follow visual order
- labels must be readable by screen readers
- status changes should be accessible
- color must not be the only state indicator

### 14.2 Quick tiles

Each tile must expose:

- label
- current state
- action possibility

For example:

- `Kill Switch, On, status`
- `DNS Protection, On, status`
- `Local Network Access, Allowed, button`

### 14.3 Content clarity

Advanced mode may be richer, but it still needs to be understandable to a normal human being. Do not turn it into an engineer-only control panel.

---

## 15. Explicit implementation constraints for agents

### 15.1 Do not invent UI structure

Agents must not:

- add more top navigation tabs
- convert the layout into a left sidebar
- add a separate DNS page accessible from the Home tile
- move the Home tiles into a settings drawer
- insert developer graphs/log streams onto the Home screen
- redesign the visual concept into a completely different window

### 15.2 Do not invent behaviors

Agents must not assume:

- profile-sharing workflows that were not approved
- advanced connection editing on the Home screen
- peer-to-peer setup flows unless separately approved
- fake success states
- hidden auto-connect changes not reflected in settings

### 15.3 Respect approved separation of concerns

- Home = at-a-glance state + quick controls
- Connections = saved-connection management
- Settings = configuration, including Secure DNS details
- Diagnostics = help and troubleshooting

### 15.4 DNS rule must not be violated

This is repeated because it is important:

> **Detailed Secure DNS settings belong in Settings only.**

The Home DNS Protection tile is only a status indicator.

---

## 16. File placement in the repository

When the owner chooses to add the approved materials to the repo, the canonical location is:

```text
Septuvo/
└── docs/
    └── ui/
        └── desktop/
            ├── DEFAULT-UI.md
            ├── Default_VPN_UI.png
            ├── ADVANCED-UI.md
            └── Advanced_VPN_UI.png
```

Agents should treat those files as the visual and behavioral source of truth for the desktop client.

---

## 17. Final owner decisions captured in this file

This document records the following owner-approved decisions:

1. The provided image is the approved **Advanced UI**.
2. The visible items are:
   - Home
   - Connections
   - Settings
   - Diagnostics
   - Connect / Disconnect
   - Kill Switch
   - DNS Protection
   - Local Network Access
   - Minimize / Maximize / Exit
3. Connections, Settings, Diagnostics, and Connect/Disconnect are functionally shared with the Default UI.
4. The Home tiles behave as follows (as amended 2026-10-18):
   - Kill Switch = truthful status indicator, not a switch (D-038)
   - DNS Protection = truthful status indicator, not a switch (D-039)
   - Local Network Access = Allowed / Not Allowed control (D-040)
5. Exit behavior matches the Default UI:
   - default X = exit and disconnect
   - if Send to System Tray is chosen, X sends the app to the tray and keeps the tunnel alive
6. Detailed Secure DNS settings belong in the **Settings menu only**.
7. Advanced mode may reveal more detailed settings than Simple mode.
8. The resulting documentation must leave no room for agents to guess or improvise.

---

## 18. Implementation reminder

This file is a binding implementation specification, not a brainstorming note.

If an agent encounters a detail that is still genuinely undecided, the correct action is:

1. preserve the approved layout,
2. preserve the approved behavior,
3. avoid guessing,
4. ask the owner.

No agent may use ambiguity as permission to “improve” the design.

---

## 19. Changes in version 1.1 (owner decisions of 2026-10-18)

| Where | Change | Decision |
|---|---|---|
| §0.3, §1.3, §2 | Decisions D-037–D-042 prevail; the tiles row is two status indicators and one control; new values Unknown, Failed, Internet Blocked | D-037–D-040 |
| §6.1 | Internet Blocked (Lockdown Mode) and VPN Not Yet Available (pre-G1) states; You’re Connected only after all four checks | D-020, D-038 |
| §6.4 | Location card content made truthful: connection name and server address, neutral icon, no inferred country or flag; click opens Connections | Reconciliation C13 (owner to confirm) |
| §6.5 | Public IP only from the user's own server over the verified tunnel; never a third-party lookup | Reconciliation C14 |
| §8 | One shared Settings structure (DEFAULT-UI §6); conflicting or duplicate rows removed or merged; DNS Protection has no On/Off | D-037, D-039 |
| §11 | Kill Switch and DNS Protection tiles are truthful, non-interactive status indicators; Local Network Access is a service-confirmed control with restricted exceptions | D-038, D-039, D-040 |
| §12 | Send to System Tray naming; GUI crash/kill and Lockdown Mode behavior | D-041 |
| §14 | Accessibility roles: the two status tiles are statuses, not buttons | — |

The approved main-window image, its visible elements and their layout are unchanged.
