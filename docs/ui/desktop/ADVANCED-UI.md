# Septuvo VPN — Advanced Desktop UI: Binding Owner Specification

**Document ID:** `SEPTUVO-UI-ADVANCED-001`  
**Document version:** `1.0 — owner-directed implementation specification`  
**Prepared:** 2026-10-18  
**Product:** Septuvo VPN desktop client, **Advanced interface**  
**Canonical repository destination:** `docs/ui/desktop/ADVANCED-UI.md`  
**Canonical image (provided separately by owner):** `docs/ui/desktop/Advanced_VPN_UI.png`  
**Companion default UI specification:** `docs/ui/desktop/DEFAULT-UI.md`  
**Governance:** This file captures the owner-approved Advanced UI image, approved visible components, approved quick-toggle behavior, the clarification that **detailed Secure DNS configuration exists only in Settings**, and the shared behavior inherited from the already approved Default UI where explicitly stated.

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
- quick-toggle behavior for Kill Switch, DNS Protection, and Local Network Access
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
- moving the quick toggles into a different structure
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
5. Existing repository rules (`AGENTS.md`, decisions, security model, gate rules, licensing boundaries) remain mandatory and cannot be weakened by UI work.

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
- quick On/Off status tiles for:
  - Kill Switch
  - DNS Protection
  - Local Network Access
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
| 5 | Quick status / action tiles row | **Kill Switch**, **DNS Protection**, **Local Network Access** |

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

3. **Bottom quick-action tiles row**
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

Status headlines must always be truthful.

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

The approved image visually shows:

- country: **United States**
- city/region: **New York**

This area represents the currently active profile or endpoint.

Behavior:

- shows the selected or active connection target
- if disconnected, it may still show the currently selected profile target
- clicking the card may optionally open the current profile details or Connections page, but if implemented it must do so consistently
- if no click behavior is implemented, it remains informational only

### 6.5 Public IP section

The approved image shows:

- label: **Your New IP Address**
- value: example IPv4 address
- copy icon on the right

Behavior:

- if connected and a current VPN-exit public IP is known, show it
- clicking the copy icon copies the displayed IP to clipboard
- if disconnected, show either:
  - `Not Connected`, or
  - hidden/placeholder state, but not misleading fake data

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

### 8.2 Required top-level settings sections

Advanced mode Settings must contain, at minimum, these sections:

1. **General**
2. **Interface**
3. **Connection**
4. **DNS Protection**
5. **Network**
6. **Updates**
7. **About**

Subsection order may vary slightly, but these categories must exist.

### 8.3 General

Recommended settings:

- **Start with Windows**
- **Launch at Sign-In** (if platform-specific naming differs, keep it professional)
- **Connect Automatically at Startup**
- **Reconnect Automatically**
- **Remember Last Used Connection**

### 8.4 Interface

Required settings:

- **Interface Mode**
  - **Simple**
  - **Advanced**
- **Close Button Behavior**
  - **Exit and Disconnect**
  - **Send to Tray**
- **Minimize to Tray When Closed** (if phrased separately, must be consistent)
- **Show Notifications**

### 8.5 Connection

Recommended settings:

- **Default Encryption Algorithm**
- **Connection Timeout**
- **Retry Behavior**
- **Use Last Connected Profile Automatically**

No setting may falsely imply real cryptographic choices are active before product state allows it.

### 8.6 DNS Protection

This section is critically important.

> **Owner-approved rule:** Secure DNS settings are in the **Settings menu only**.

That means:

- the Home tile for DNS Protection is only a quick **On / Off** control
- detailed DNS editing does **not** happen in the Home tile
- detailed DNS editing does **not** happen in a separate DNS pop-up invented by agents unless the owner later approves one

#### Required DNS Protection settings capabilities

The Advanced Settings area must support detailed DNS configuration including, as applicable:

- **DNS Protection** On / Off
- **DNS Mode**
  - Automatic
  - DNS over HTTPS (DoH)
  - DNS over TLS (DoT)
  - other modes only if explicitly approved later
- **Primary Secure DNS**
- **Secondary Secure DNS**
- **Bootstrap DNS**
- **Fallback DNS Behavior**
- **Use Global DNS Settings**
- **Allow Per-Connection DNS Override**
- **Test DNS Settings**

If the implementation later needs simpler naming for usability, it must remain faithful to these functions.

### 8.7 Network

Required network-related settings:

- **Kill Switch** default behavior
- **Local Network Access** default behavior
- **Route All Traffic Through VPN** (if supported)
- **Profile Switching Behavior**
- **Advanced Network Options** only if approved and understandable

### 8.8 Updates

Required update options:

- **Automatically Check for Updates**
- **Notify When Updates Are Available**
- **Check for Updates Now**

The owner explicitly requested update-checking capability.

### 8.9 About

About section must include:

- application version
- build/release information if available
- credits
- legal / license information
- project links as appropriate

Version information should also appear at the bottom of the Settings area.

---

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

## 11. Quick toggles on the Advanced Home screen

The Advanced Home screen includes three quick-toggle tiles.

These are part of what distinguishes Advanced mode from Default mode.

---

### 11.1 Kill Switch tile

#### Label
**Kill Switch**

#### Value states
- **On**
- **Off**

#### Supporting text
When On, helper text should explain that it blocks internet traffic if the VPN drops.

#### Behavior
- clicking the tile toggles the setting
- change should apply immediately when possible
- if disconnected, it should still set the preference for next use if immediate action is not meaningful

#### Recommended warning
When turning it Off, show a warning similar to:

> Turning off Kill Switch may allow internet traffic outside the VPN.

---

### 11.2 DNS Protection tile

#### Label
**DNS Protection**

#### Value states
- **On**
- **Off**

#### Supporting text
Explains that DNS queries are protected/private when enabled.

#### Behavior
- clicking the tile toggles DNS Protection On / Off
- this tile does **not** open a DNS editor
- this tile does **not** contain detailed DNS settings

#### Critical owner-approved rule
All Secure DNS configuration details live in:

**Settings → DNS Protection**

Nowhere else unless separately approved later.

#### Recommended warning
When turning it Off, show a warning similar to:

> Turning off DNS Protection may expose DNS lookups outside the protected tunnel.

---

### 11.3 Local Network Access tile

#### Label
**Local Network Access**

#### Value states
- **Allowed**
- **Not Allowed**

#### Supporting text
Explains whether local devices on the current LAN may be accessed while connected.

#### Behavior
- clicking toggles Allowed / Not Allowed
- change should apply immediately if possible
- if disconnected, save as the preferred behavior for next connection

#### Warning guidance
If turning local access on or off has significant security implications, a concise explanation may be shown, but it must not become noisy.

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

If the user has enabled **Send to Tray** in Settings:

- clicking **X** sends the UI to the system tray / background instead of fully exiting
- the VPN connection remains active

This behavior must match the Default UI.

---

## 13. State handling

The Home screen must remain truthful and coherent in every state.

### 13.1 Connected

Expected elements:

- headline indicates connected
- action button says Disconnect
- location summary populated
- public IP populated if available
- quick toggles reflect actual settings

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

- `Kill Switch, On, button`
- `DNS Protection, Off, button`
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
- move the quick toggles into a settings drawer
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

The Home tile is only a quick toggle.

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
4. The quick Home tiles behave as follows:
   - Kill Switch = On / Off
   - DNS Protection = On / Off
   - Local Network Access = Allowed / Not Allowed
5. Exit behavior matches the Default UI:
   - default X = exit and disconnect
   - if Send to Tray is enabled, X sends the app to tray and keeps the tunnel alive
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
