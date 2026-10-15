# M1 — product decisions (decided 2026-10-15)

**Status:** **Decided 2026-10-15: all 14 as recommended** (owner instruction to the agent, recorded as [D-032](../DECISIONS.md)). Under each question, the **Recommendation** is now the decision. The requirements in the [threats](../spec/threats.md), [network policies](../spec/network-policies.md), [identity and privacy](../spec/identity-and-privacy.md) and [platform feasibility](../spec/platform-feasibility.md) documents apply them.

**Changing a decision later:** say so in ordinary words (for example "change Q4 to on by default"). An agent records it as a new decision and updates the requirements in a pull request for you to merge.

**None of these is a cryptographic decision.** Cryptography stays with M2 and the G1 outside review.

Each question below has:

- **What it means:** in plain words.
- **Options:** with their trade-offs.
- **Recommendation:** and why.
- **Where it is used:** the requirement IDs.

---

### Q1 — Captive portals (hotel and café login pages)

- **What it means:** many public Wi-Fi networks show a login page before allowing internet access. The kill switch blocks everything outside the tunnel, so it also blocks that login page, and the tunnel can't start until you've logged in.
- **Options:**
  - (a) A short login mode that you start yourself: for at most 5 minutes, web traffic may go out directly, with a clear warning; then blocking resumes.
  - (b) No special mode: you must switch Septuvo off entirely to log in. This is simple, but people forget to switch it back on.
  - (c) Automatic detection and bypass: convenient, but a hostile network could trigger it on purpose.
- **Recommendation:** (a). It keeps protection for everything else and is never automatic.
- **Where it is used:** network-policies NP-U5; threats R-09.

### Q2 — Does blocking stay on after you press Disconnect?

- **What it means:** when the VPN fails, Septuvo always blocks traffic. The question is what happens when **you** deliberately disconnect.
- **Options:**
  - (a) Normal internet comes back. You chose to go unprotected.
  - (b) Blocking stays until you change a setting. Safer, but confusing ("my internet is broken").
- **Recommendation:** (a). Failures still block, and the always-on setting (Q4) gives (b) to anyone who wants it.
- **Where it is used:** network-policies NP-E2, section 8.

### Q3 — Local discovery (`.local` names, casting, printers)

- **What it means:** devices on a network find each other with broadcast and multicast messages. On a hostile network those messages reveal your device and can be spoofed.
- **Options:**
  - (a) Blocked in version 1: reach your NAS by address or by a name you set in Septuvo.
  - (b) Allowed when you approve a network: convenient for casting and printers, but much harder to make safe.
- **Recommendation:** (a) now; revisit after version 1.
- **Where it is used:** network-policies NP-S4.

### Q4 — Always-on mode (blocking from the moment the computer starts)

- **What it means:** with always-on, nothing can reach the internet until the tunnel is up, even during start-up.
- **Options:**
  - (a) Offered as a setting, off by default.
  - (b) On by default. More protection, but a new user with a setup mistake has no internet at all.
- **Recommendation:** (a), with setup offering to turn it on once the connection is working.
- **Where it is used:** network-policies section 8; threats R-10.

### Q5 — When the café network uses the same addresses as your home

- **What it means:** many routers use `192.168.1.x`. If both home and café do, traffic for `192.168.1.10` could go to the wrong place.
- **Options:**
  - (a) The tunnel wins while connected; café devices at those addresses are unreachable.
  - (b) Ask each time. Annoying, and wrong answers leak.
  - (c) Translate the home network to different addresses at the server. Clean, but it needs its own design.
- **Recommendation:** (a) for version 1; consider (c) later.
- **Where it is used:** network-policies NP-S5; threats R-04.

### Q6 — Your own devices talking to each other through the server

- **What it means:** whether your phone can reach your laptop over the VPN, not just the home network.
- **Options:**
  - (a) Off by default; the administrator can allow it.
  - (b) On by default.
- **Recommendation:** (a). It limits what a stolen or compromised device can reach.
- **Where it is used:** network-policies NP-G2; threats R-07.

### Q7 — Invitation lifetime and confirming new devices

- **What it means:** an invitation (QR code or link) lets one new device join your server. Anyone who gets it first could use it.
- **Options:**
  - Default lifetime: 1 hour, 24 hours or 7 days.
  - Administrator confirmation: on or off. With it on, the server and the new device show the same short code, and you approve.
- **Recommendation:** 24 hours default, 7 days maximum, at most 5 wrong attempts, confirmation **on**. Confirmation stops a stolen invitation from working.
- **Where it is used:** identity-and-privacy EN-2, EN-5, EN-7.

### Q8 — Backup of the server's identity

- **What it means:** if the server's identity key is lost (disk failure), every device must be added again.
- **Options:**
  - (a) An encrypted backup that you export by hand and store offline.
  - (b) No backup: losing the server means re-adding devices.
  - (c) Automatic backup. Convenient, but the key ends up in more places.
- **Recommendation:** (a), available but never automatic.
- **Where it is used:** identity-and-privacy ID-2, RV-5.

### Q9 — Checking for updates

- **What it means:** an update check contacts a download site, so it is the only network call that isn't to your own server.
- **Options:**
  - (a) Off by default, asked once during setup.
  - (b) On by default.
  - (c) Never: updates only by hand.
- **Recommendation:** (a). Nothing contacts outside services unless you agree.
- **Where it is used:** identity-and-privacy LO-2.

### Q10 — How long logs are kept

- **What it means:** logs help troubleshooting but are a privacy record. They never contain visited sites, DNS names or secrets (PR-2), only connection times, errors and device names.
- **Options:** 0, 7 or 30 days, on devices and on the server.
- **Recommendation:** 7 days, changeable down to 0.
- **Where it is used:** identity-and-privacy PR-3, PR-4.

### Q11 — Septuvo Server on ARM64 (for example a Raspberry Pi)

- **What it means:** many home servers are small ARM boards rather than x86 PCs.
- **Options:**
  - (a) A version 1 target: built and tested in CI later, but not called supported until its lab tests pass.
  - (b) x86_64 only for version 1.
- **Recommendation:** (a). x86_64 stays first.
- **Where it is used:** platform-feasibility section 1.

### Q12 — A home server needs an open port on your router

- **What it means:** for your devices to reach a server at home, your router must forward one UDP port to it. Internet scanners will see that port.
- **Options:**
  - (a) Accept this for version 1, with strict limits on what unauthenticated packets can make the server do (S-07).
  - (b) Require a relay service. This breaks "no outside services" (D-007).
  - (c) Support only servers on a rented VPS. This loses the home-privacy option.
- **Recommendation:** (a). Relays only ever as an opt-in later.
- **Where it is used:** threats R-01; network-policies NP-S2.

### Q13 — Default tunnel address range

- **What it means:** inside the tunnel each device gets an address. If that range clashes with a network you visit, things break.
- **Options:**
  - (a) A random `10.x.y.0/24` plus a random private IPv6 range, picked per server at setup.
  - (b) One fixed range for everyone. Simpler, but more clashes.
- **Recommendation:** (a), changeable.
- **Where it is used:** network-policies sections 1 and 11.

### Q14 — Hiding that you are using a VPN (anti-censorship)

- **What it means:** some networks block or slow VPNs. Disguising VPN traffic ("obfuscation") is a large, separate design problem.
- **Options:**
  - (a) Not a version 1 goal; revisit after version 1.
  - (b) Make it a version 1 goal. This adds significant protocol work before G1.
- **Recommendation:** (a).
- **Where it is used:** threats section 8 (non-goals).
