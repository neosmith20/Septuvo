# Desktop UI reconciliation (approved designs → repository decisions)

**Prepared 2026-10-18** after the owner finalized the Septuvo VPN desktop designs:
- [`docs/ui/desktop/DEFAULT-UI.md`](../ui/desktop/DEFAULT-UI.md) with [`Default_VPN_UI.png`](../ui/desktop/Default_VPN_UI.png) (Simple mode);
- [`docs/ui/desktop/ADVANCED-UI.md`](../ui/desktop/ADVANCED-UI.md) with [`Advanced_VPN_UI.png`](../ui/desktop/Advanced_VPN_UI.png) (Advanced mode).

The owner also gave final decisions on the kill switch (Mullvad model), Lockdown Mode, DNS Protection, Local Network Access, window and close behaviour, profile switching, updates, cross-platform targets and the Rust rule. This record lists every conflict found between those approvals and the earlier documents, how each was resolved, what the resolution means for implementation, what needs independent review, and what still needs an owner decision.

**Nothing here redesigns either interface.** Where an approved image shows sample content (a country flag, an IP address, "Protected"), the element stays and its content is defined truthfully.

## 1. New and changed decisions

| Decision | Summary | Replaces or amends |
| --- | --- | --- |
| **D-037** Desktop interfaces | The two designs are the binding desktop targets for Windows, Linux and macOS. Simple is the default; Advanced is optional; switching never touches the connection. Both modes share one implementation of connections, configuration, import/export, Settings, diagnostics, updates and policies | New. D-014 (platform order) unchanged |
| **D-038** Kill switch and Lockdown Mode | Kill switch mandatory and not user-disableable (Mullvad model); the Advanced tile is a truthful status. **Lockdown Mode** (off by default) keeps blocking after deliberate disconnect, app exit, service restart and from boot | Amends **Q2** (deliberate disconnect restores networking *unless Lockdown Mode is on*) and **Q4** (always-on blocking from boot *is* Lockdown Mode). Retires `behavior.always_on` |
| **D-039** DNS Protection | DNS leak prevention mandatory during protected connections; the tile is a status. Configurable secure DNS (Automatic, Standard, DoH, DoT, bootstrap, validation, fail-closed fallback, global and per-connection) **only** under Settings → DNS Protection | Extends NP-D1–NP-D6. DoH/DoT blocked on the TLS decision (section 5) |
| **D-040** Local Network Access | Allowed / Not Allowed, default Not Allowed. Allowed = only the directly attached local subnets, size-capped, never overlapping tunnel routes; discovery stays blocked (Q3) | Applies D-006 and NP-E7 to a user toggle |
| **D-041** Window, tray and switching | Minimize never disconnects. X by default = deliberate disconnect then exit; with **Send to System Tray**, X hides and keeps the connection. A GUI crash or kill changes nothing in the service. Selecting another profile while connected switches automatically, with protection kept throughout | **Supersedes D-019** ("closing never disconnects") |
| **D-042** Updates | Check, notify and manual check from official GitHub releases. A release is **not** a secure update mechanism; automatic installation stays disabled until signed updates exist (S-12, G3) | Amends **Q9** (see open decision 2) |
| **D-005** (H-020) | Already corrected: Septuvo-authored code is Rust; external components are unrestricted in language but reviewed | — |

## 2. Conflicts found and how they were resolved

| # | Conflict | Where | Resolution |
| --- | --- | --- | --- |
| C1 | "Closing the app never disconnects" vs the approved X behaviour | D-019, N4 work order ("closing is not disconnecting"), network-policies §8, TEST_MATRIX T3 | D-041 supersedes D-019. Every document now states: X default = disconnect and exit; Send to System Tray = hide and stay connected; the GUI process ending for any other reason changes nothing |
| C2 | Advanced Kill Switch tile was an On/Off toggle with a "turn off" warning | ADVANCED-UI §11.1, §8.7 | Now a non-interactive status indicator (On / Off / Unknown / Failed), never a switch. No "Kill Switch default behavior" setting |
| C3 | Advanced DNS Protection tile was an On/Off toggle; Settings had "DNS Protection On / Off" | ADVANCED-UI §11.2, §8.6 | Tile is a status indicator. No On/Off anywhere: leak prevention cannot be disabled |
| C4 | "Your DNS queries are encrypted and private" confuses leak prevention with encrypted transport | Advanced image helper text | Helper text depends on what is verified (section 3 of ADVANCED-UI). "Encrypted" only when the verified tunnel or verified DoH/DoT carries the queries |
| C5 | Q2: deliberate disconnect always restores networking | D-032, network-policies, daemon docs | Still true with Lockdown Mode off (the default). With Lockdown Mode on, blocking stays |
| C6 | `behavior.always_on` meant "connect automatically" in config.md, but "blocking from boot" in Q4, and appeared as a per-connection editor field | config.md, Q4, DEFAULT-UI §4.2 | Split into two separate things, never conflated: **Lockdown Mode** (blocking, Settings → Connection & Security) and **Connect Automatically at Startup** (Settings → General). `behavior.always_on` is retired from the connection file and removed from the editor |
| C7 | Settings structure: DEFAULT-UI listed four sections ("no new categories"); ADVANCED-UI listed seven; the owner's decisions name Connection & Security and DNS Protection | DEFAULT-UI §6, ADVANCED-UI §8.2 | One shared Settings structure for both modes: **General, Interface, Connection & Security, DNS Protection, Network, Updates, About & Credits**. Advanced mode shows extra rows, never different behaviour |
| C8 | Duplicate or conflicting Settings rows: "Send to Tray" vs "Send to System Tray"; "Minimize to Tray When Closed"; "Launch at Sign-In"; "Check for Updates Now"; "Notify When Updates Are Available"; "Default Encryption Algorithm"; "Remember Last Used Connection"; "Use Last Connected Profile Automatically"; "Profile Switching Behavior"; "Route All Traffic Through VPN" | ADVANCED-UI §8 | One label each: **Send to System Tray** (macOS: Menu Bar), **Start Septuvo with Windows** (Linux/macOS: **Start Septuvo at Login**), **Check for Updates**, **Notify When an Update Is Available**, **Encryption Algorithm**. The last-used connection is always remembered (no setting). Profile switching is fixed by the owner's decision (no setting). Routing mode is per connection (editor), not global |
| C9 | DNS fields in the connection editor vs "all detailed DNS configuration exclusively under Settings → DNS Protection" | DEFAULT-UI §4.2 | The editor shows no DNS fields, only which DNS settings apply and a link. Per-connection DNS overrides are edited in Settings → DNS Protection |
| C10 | `dns.mode = "system"` is accepted in split mode, where it would send names of protected (home) networks to the local network's DNS (NP-D2) | config.md | Recorded as an implementation change: refuse `system` unless the protected domains are always sent through the tunnel (per-domain DNS, NP-D2). Strengthens, never weakens |
| C11 | "Primary target: Windows" | DEFAULT-UI §0.3 | Both designs are shared targets for Windows, Linux and macOS (owner, 2026-10-18). Build order stays Linux first (D-014) |
| C12 | Q9: update checks off by default and asked once at setup; DEFAULT-UI: "Automatically Check for Updates" defaults On | D-032 Q9, LO-2, DEFAULT-UI §6.3 | Reconciled as: setup asks once with **On preselected**; nothing contacts GitHub until the user has answered. **Owner to confirm** (open decision 2) |
| C13 | Advanced location card shows a country flag and city; DEFAULT-UI forbids inferring geography or using flags | ADVANCED-UI §6.4, DEFAULT-UI §3.1 | The card stays. It shows the connection's name and server address with the neutral connection icon until a trustworthy location source exists. **Owner to confirm** (open decision 3) |
| C14 | "Your New IP Address" needs a source; a third-party "what is my IP" service would be an unapproved outside connection (LO-2, D-017) | ADVANCED-UI §6.5 | Shown only when the user's own Septuvo Server reports it over the verified tunnel (needs a protocol message, M2/G1). Until then `Not Connected` / `Not Available`. Never a third-party lookup unless the owner approves one |
| C15 | Rust rule wording in older documents | AGENTS, ARCHITECTURE, D-015, N4 | Already corrected in H-020 (PR #68); this change builds on it |
| C16 | GTK: H-020 kept the 2026-10-17 GTK rejection as the N4 toolkit decision; the owner has since said GTK is acceptable if it is the best fit | D-005 (H-020 text), N4-D1 | GTK is permitted. N4-D1 keeps **egui/eframe** because the approved designs are a custom, brand-drawn, cross-platform look (glows, gradients, globe art) that a native-theme toolkit would not reproduce consistently on Windows and macOS (section 4) |
| C17 | N4 work order and task cards describe a different app (status page, sidebar navigation, "closing is not disconnecting") | N4.md, TASK_CATALOG, ROADMAP, PR #69 | N4 re-scoped to the approved designs (section 6). PR #69's screens contradict DEFAULT-UI (sidebar navigation is forbidden), so it is reworked into the new N4-01 rather than merged as is |
| C18 | Agents' mandatory references did not include the UI files | AGENTS, AGENT_OPERATIONS | Added as binding references |
| C19 | Captive-portal sign-in (N3-02) has no control in either approved design | captive-portal.md | Not added to the UI (no invented controls). It stays a command-line feature. **Owner to decide later** (open decision 5) |
| C20 | TEST_MATRIX T3 says closing the UI must not disconnect | TEST_MATRIX | Rewritten for D-041; Lockdown, LAN access and profile-switch tests added to T2 |

## 3. What the existing implementation does today, and what is missing

| Area | Today (N1–N3, lab only) | Needed for the approved designs |
| --- | --- | --- |
| Kill switch | `connect` installs nftables rules before anything else; `disconnect` removes them and restores the resolver file; a crash or stop leaves them in place; restart reports them | Atomic **profile switch** that replaces the policy in one nftables transaction with no unblocked moment; **verified** status fields for the tile (installed *and* matching the expected ruleset) |
| Lockdown Mode | Not implemented | A service setting; blocking ruleset while not connected (loopback, DHCP/ND, saved server endpoints, Local Network Access if allowed, captive-portal helper on request); kept after disconnect, app exit and service restart; **installed at boot before the network is up**; removed by turning it off or by uninstall; a documented administrator recovery command |
| Local Network Access | Only explicit `routing.direct` prefixes per configuration | A service setting; compute the directly attached subnets from the interface's own address and prefix length (netlink); size caps (IPv4 no shorter than /16, IPv6 on-link no shorter than /64, plus link-local `fe80::/10` on that interface); never a prefix overlapping tunnel routes or endpoints; recompute on every network change; discovery and multicast still blocked (Q3) |
| DNS leak prevention | DNS only to the configured resolvers through the tunnel (plus listed LAN DNS exceptions); 53/853/DoQ elsewhere dropped; lab resolver file written and restored | Verified status fields for the tile; refuse `dns.mode = "system"` without per-domain protection (C10); global and per-connection DNS settings stored by the service; a real resolver integration on hosts (systemd-resolved), not only the lab file |
| Encrypted DNS (DoH/DoT) | Not implemented (no TLS library; N-tracks forbid cryptography crates) | A Septuvo stub resolver on loopback, run by the service; DoH/DoT to resolvers **through the tunnel** with full certificate validation; IP-address bootstrap or bootstrap only through the tunnel; firewall rules scoping resolver traffic to the stub; fail-closed fallback (block, or standard DNS through the VPN only if the user chose it); a DNS test command. **Blocked on the TLS decision** |
| Profiles | One `device_config` file in `septuvod.toml` | A service-owned profile store (validated complete profiles), app-owned drafts, select/list/apply over IPC, size-bounded framed transfer (as N4-05 planned), no secrets in exports |
| Closing and tray | Not applicable (no GUI) | App preference store (per user); X handling with confirmed disconnect; StatusNotifierItem tray with the approved menu; fallback when no tray host exists |
| Diagnostics | `status` and `watch` | Service `DIAGNOSE`; ICMP ping of the server endpoint and of a public host **only on request** (by the service, inside the firewall policy); report with redaction preview |
| Updates | Not implemented | GitHub release check over HTTPS (**TLS decision**); notification; signed-update installation only after S-12/G3 |
| Status for the UI | Four stages, service state, kill switch, DNS policy, resolver file, captive window | Fields for every approved state (Connecting, Switching, Disconnecting, Reconnecting, Failed with reason, Lockdown blocking), the active profile, verified tile values, connection start time from verification |

## 4. Toolkit (N4-D1): egui/eframe kept, GTK permitted

The owner allowed GTK if it is the best fit. For these designs it is not, so egui/eframe stays:
- **The designs are custom-drawn.** Glowing rings, gradient pill buttons, a globe illustration behind live text and dark brand surfaces have to be painted the same way on Windows, Linux and macOS. egui paints everything itself and matches a reference image exactly on all three. GTK 4 and libadwaita follow the platform theme; reproducing this look means fighting the theme, and GTK on Windows and macOS is a second-class, heavy install.
- **One code base for three platforms**, with native window controls, file dialogs and tray provided per platform (D-037).
- **Accessibility:** AccessKit gives screen-reader support on all three platforms.
- **GTK remains allowed** under D-005 if a future need appears; it would go through the normal dependency review.

## 5. Security implications needing independent review

These go to the qualified reviewer **in addition to** the G1 protocol review, before the related features ship:

1. **Lockdown Mode at boot and its recovery path.** Rules installed before the network starts, kept across service restarts, removed only by the user, administrator recovery or uninstall. Risk: a user locked out of the internet with a broken service; or a gap at boot.
2. **Local Network Access on hostile networks.** The "local subnet" comes from DHCP and router advertisements, which an attacker on the network controls (NP-6.2). The size caps, the overlap rules and the no-discovery rule must be reviewed and lab-tested.
3. **Encrypted DNS client.** TLS library choice, certificate validation, trust store, bootstrap and fallback rules, and the stub resolver's own exposure.
4. **Update checking and installation.** HTTPS client, release authenticity (signatures, not "it came from GitHub"), downgrade protection, behaviour during an active connection (S-12, G3).
5. **Profile switching without a gap.** Atomic replacement of the ruleset while the old session ends and the new one starts.
6. **Configuration export and import.** No secrets exported; imported files treated as hostile input; never treated as enrollment.

## 6. Implementation work and where it belongs

| Work | Milestone / task | Status |
| --- | --- | --- |
| App shell for both modes: Simple main window, Advanced main window and tabs, mode switch, shared navigation, truthful pre-G1 states, preference store, X/minimize behaviour; native-code gate and dependency review | **N4-01, N4-02** (revised N4) | Authorized by merging this change |
| Service: profile store, select/apply, atomic switching, status fields for every approved state | **N4-03** | Same |
| Service: Lockdown Mode (including boot) and Local Network Access | **N4-04** | Same |
| Connections manager, Add / Import, configuration editor with drafts, native Open/Save dialogs, profile selector | **N4-05** | Same |
| Settings (all seven sections, DNS Protection page with Automatic and Standard DNS working; DoH/DoT shown as not yet available), tray and notifications, start at login, connect at startup | **N4-06** | Same |
| Help & Diagnostics window, service `DIAGNOSE`, pings on request, report | **N4-07** | Same |
| Rust installer, packaging, accessibility pass, user guide, visual-fidelity review against both reference images | **N4-08** | Same |
| DoH/DoT, update checks | **After the TLS decision** (new work order) | Blocked |
| Automatic update installation | **G3** (signed builds, S-12) | Blocked |
| Real connection states, public IP from the server, server-reported data | **M5** after **G1** | Blocked |
| Windows and macOS clients with the same designs | **M7** (D-014) | Later |

## 7. Open owner decisions

1. **TLS client (blocks DoH/DoT, update checks and captive-portal phase 2).** Options:
   - (a) Treat a TLS *client* for DNS, updates and captive portals as outside the G1 boundary, reviewed under D-015 like any other dependency; **recommended**.
   - (b) Wait for the G1 reviewer to cover it too.

   Library choices to review either way: `rustls` with a reviewed crypto provider, or the operating system's TLS.
2. **Update-check default (C12).** Options:
   - (a) Ask once at setup, On preselected, no contact before the answer; **recommended**.
   - (b) On by default without asking (changes Q9 and LO-2).
   - (c) Off by default.
3. **Advanced location card (C13).** Options:
   - (a) Connection name and server address with the neutral icon until the server can report a location after G1; **recommended**.
   - (b) Add a user-entered "Location" label (and optional flag) to each connection.
4. **Hero artwork source.** The approved images contain the globe, network and glowing-ring artwork only as part of full screenshots, with text baked in. Faithful implementation needs either layered source art from the owner, or approval of a plan: draw the rings, shield and glows in code from the brand tokens, plus one globe illustration without text. DEFAULT-UI §10 already requires asking.
5. **Captive-portal sign-in in the UI (C19).** Options:
   - (a) Leave it as a command-line feature until phase 2; **recommended**.
   - (b) Add a control, which would need an approved design.
