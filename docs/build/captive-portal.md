# Captive-portal sign-in (N3-02): design, evaluation and phased plan

**Status:** phase 1 implemented (lab only). Phases 2 and 3 need owner decisions.

**Why this was revised.** The first version of N3-02 (PR #63, before review) let **every** program send web traffic (TCP 80/443) and DNS to the network's resolver outside the tunnel for up to five minutes. The owner rejected it, and was right to:
- unrelated applications could leak traffic during the window;
- it trusted the hotel's DNS server;
- it relied on the window being short, rather than on the exception being narrow.

This document records the owner's seven requirements and how each is met, deferred or declined.

## The model: one isolated helper, everything else unchanged

```text
                      ┌──────────────── kill switch (unchanged for everyone else) ───┐
 ordinary programs ──▶│ tunnel only · DNS only to the tunnel resolver · fail closed   │
 (any user, root)     └────────────────────────────────────────────────────────────────┘
                      ┌──────────────── helper account "septuvo-portal" only ─────────┐
 sign-in helper ─────▶│ never the tunnel · never hosts the policy names · never the   │
                      │ server endpoint · nothing at all, EXCEPT while a window is     │
                      │ open: TCP 80/443 outside the tunnel (+ the network's DNS only  │
                      │ if --network-dns was given for that window)                    │
                      └────────────────────────────────────────────────────────────────┘
```

**The rules for the helper:**
- **The kernel picks out the helper by account.** Every helper rule matches the socket's owner (`meta skuid UID`). The helper account is a dedicated system account with no login, no home and no other rights. The packages create it as `septuvo-portal`, and the service refuses root.
- **The helper's rules come first** (right after loopback) and **end in a final drop**. So no later rule (server endpoints, LAN DNS exceptions, `direct` rules, the default) ever applies to the helper.

**For every other program the ruleset is exactly the kill switch without captive-portal mode.** A renderer test checks this line by line.

**A window is three nftables timeout sets** (`captive_web`, `captive_dns4`, `captive_dns6`):
- opening a window adds elements that expire after at most 5 minutes;
- **the kernel deletes them itself**, so a crashed or killed service cannot keep a window open.

## The owner's requirements

| # | Requirement | Phase 1 (this PR) | Remaining |
| --- | --- | --- | --- |
| 1 | No unrestricted web bypass; an isolated helper with narrow permissions | **Done.** Ordinary programs never get the exception. Only the helper account, only TCP 80/443, only outside the tunnel, never to policy-listed hosts | Phase 2 may narrow the helper's *destinations* too (see below) |
| 2 | Trusted DNS first: DoH (Cloudflare or user-chosen), real TLS verification, narrow exception | **Firewall side done.** By default the helper has **no plain DNS at all**, so it must use DoH over its web ports | A Septuvo DoH client needs a TLS library: phase 2 (owner decisions) |
| 3 | RFC 8910 / RFC 8908 discovery; network data untrusted | **Evaluated** below | Phase 2: needs DHCP/RA access and an HTTPS client |
| 4 | No automatic fallback to untrusted DNS; never system-wide | **Done.** The network's DNS opens only with an explicit `--network-dns`, for that one window, for the helper only. **The resolver file is never changed** | — |
| 5 | Never weaken HTTPS verification; name the unsafe legacy cases | **Done.** Nothing in Septuvo touches certificates. Unsupported cases are listed below | Phase 2 helper must refuse certificate errors |
| 6 | Kernel-enforced expiry; repeats cannot keep a bypass open | **Done.** Kernel timeout sets; asking again restarts from now (never adds up); **at most 3 windows per connection**, each logged; disconnect or reconnect ends it | — |
| 7 | Security tests | **Done:** renderer tests and lab cases CP00–CP53 (below) | — |

## Using phase 1

```sh
septuvo captive-portal                  # a 5-minute window, trusted DNS only
septuvo captive-portal --minutes 2      # shorter
septuvo captive-portal --network-dns    # ALSO the network's DNS, for the helper only, this window only
```

The sign-in must then happen **in a program running as the helper account**, for example a browser started as `septuvo-portal` with strict DNS over HTTPS and no fallback (Firefox calls this "Max Protection"), pointed at Cloudflare or another trusted resolver.

**Ordinary programs cannot use the window**, including the user's normal browser. That is the point of the design, and also why phase 1 is not yet convenient: there is no Septuvo launcher for the helper yet (phase 2).

**Status** shows:
- whether a window is open, and the time left;
- the warning;
- the helper account and its DNS mode;
- the windows left on this connection.

## RFC 8910 and RFC 8908 (requirement 3)

**RFC 8910** puts the URI of a captive-portal *API* in DHCPv4 option 114, DHCPv6 option 103 and IPv6 Router Advertisement option 37. The value `urn:ietf:params:capport:unrestricted` means "no portal". The RFC's own security section says these options carry the same risk as any DHCP/RA data: **an attacker on the network can inject, change or block them**. It also points out the benefit: a client that knows it is behind a portal can open the page in a sandbox, labelled as untrusted.

**RFC 8908** defines the API itself:
- **Transport:** the API **must** be reached with an `https` URI. The server certificate must match the host name from the provisioning, and if it cannot be validated the client **must not** use the API.
- **Response:** `application/captive+json` with `captive` (required), `user-portal-url` (must be reached over TLS), `venue-info-url`, `can-extend-session`, `seconds-remaining` and `bytes-remaining`.
- **What TLS proves:** in the RFC's own words, TLS only proves that the server matches what the network handed out, **not** that the network's provisioning is honest.

**Conclusions for Septuvo:**
- **Worth using.** It is the standards-based way to learn *that* a portal exists and *where* it is, without guessing via plain-HTTP interception.
- **Every value is untrusted input**, because an attacker on the network can forge it:
  - the API URI is a hint;
  - `user-portal-url` may only be opened in the isolated helper, and only over HTTPS with a valid certificate;
  - `seconds-remaining` must never lengthen Septuvo's window beyond its own cap.
- **Not implementable in phase 1:**
  - **reading the DHCP/RA options** needs the network-configuration integration of the real client (M5): the lab service has no DHCP or RA client;
  - **calling the API** needs an HTTPS client with certificate validation, which means a TLS library: a dependency decision (D-015), and a ruling that a TLS *client* for portals/DoH is outside the G1 boundary on SVP cryptography. The N3 work order forbids any cryptography crate.

## DNS over HTTPS (requirement 2)

- **Recommended default:** Cloudflare's resolver (`https://cloudflare-dns.com/dns-query`, RFC 8484; addresses 1.1.1.1, 1.0.0.1, 2606:4700:4700::1111, 2606:4700:4700::1001), with a user-configurable alternative.
- **Requirements for a Septuvo DoH client (phase 2):**
  - full certificate and host-name validation, with no "continue anyway";
  - the resolver's addresses configured, so no plain DNS is needed to find it;
  - answers used only inside the helper, never written to the system resolver.

**Firewall scope today.** The helper's web exception already covers DoH, so no separate rule is needed in phase 1. A rule allowing *only* the DoH resolver's addresses is worth having **only if the helper's other web destinations are narrowed too**. That is the phase 2 option below.

**Stated plainly:**
- **During a window the helper may reach any web server outside the tunnel.** It needs this because the portal, its redirects and its payment or login providers are not known in advance. This is permitted for sign-in, *not* merely to enable DoH.
- **The firewall cannot check that the helper verifies certificates.** That is the helper program's job (phase 2).

## Legacy portals that cannot be supported safely (requirement 5)

| Portal behaviour | Phase 1 result | Why |
| --- | --- | --- |
| HTTPS with an invalid, self-signed or mismatched certificate, or HTTPS interception | **Not supported, by design** | Accepting it would mean bypassing certificate validation |
| Login page only over plain HTTP | Reachable by the helper on TCP 80, **insecure** | Anything typed (room number, card details) can be read on the network. Septuvo cannot make that safe; phase 2 must warn before any form is sent |
| Portal works only by hijacking DNS (answers every name with its own address) | Only with `--network-dns` | The answers are untrusted by definition; isolated to the helper |
| Portal blocks DoH resolvers until sign-in | Only with `--network-dns` | Same |
| Portal needs QUIC, other ports or UDP | **Not supported** | The helper gets TCP 80/443 only |
| Portal or its login provider sits on an address the policy lists (`direct`, `block`, `tunnel`) | **Not reachable by the helper** | Those destinations are deliberately out of the helper's reach |
| Sign-in needs the user's normal browser cookies or saved logins | **Not available** | The helper is a separate account by design |

## Phased plan

1. **Phase 1 (this PR, done):**
   - kernel isolation of a helper account;
   - trusted-DNS-by-default firewall;
   - explicit, helper-only network DNS;
   - kernel expiry, window limits and logging;
   - security tests.
2. **Phase 2 (needs owner decisions; recommended for the real client, M5):**
   1. **TLS library decision (D-015)** and a ruling that a portal/DoH TLS *client* is outside the G1 boundary. Without it, nothing below can be built in Rust.
   2. **A Septuvo sign-in helper:**
      - discovery via RFC 8910 (from the OS network manager), then the RFC 8908 API over validated HTTPS;
      - a DoH client with strict validation;
      - opening `user-portal-url` in an isolated, clearly labelled browser window running as the helper;
      - refusing certificate errors and warning on plain-HTTP forms.
   3. **Optionally, narrower destinations:** open only the DoH resolver's addresses, then only the addresses the helper resolved for the portal and its redirects. This closes the remaining "any web server" gap, but needs the helper and the service to cooperate on every redirect. **Evaluate it before committing.**
3. **Phase 3:** usability on real networks (desktop launcher, notifications when a portal is detected), and the G2 lab on real Wi-Fi.

## Tests

- **Renderer** (`septuvo-policy`):
  - with no helper, no captive rule or set exists;
  - rules for every other program are identical with and without a helper;
  - every captive rule is scoped to the helper's account;
  - the helper's rules come first and end in a drop;
  - root is refused as the helper.
- **Engine** (`septuvo-daemon`):
  - refusals: no helper, not connected, status socket, bad lengths, a fourth window;
  - trusted DNS by default, network DNS only when asked;
  - restarting the window never adds up;
  - the window ends on disconnect, reconnect and kernel expiry;
  - a window opened before a restart.
- **Namespace lab** `tests/lab/namespaces/captive.sh` (cases CP00–CP53, IPv6 required in CI). It uses a deliberately **malicious network resolver** and three kinds of program: the helper, root and an ordinary user. It proves:
  - **ordinary programs cannot use the window** for web traffic, DoH or the network's DNS;
  - **the helper's scope:** sign-in page, HTTPS and the DoH stand-in work. Nothing else works: QUIC, other ports, DNS over TLS, the tunnel, the tunnel resolver, policy-blocked or `direct` hosts, the server endpoint;
  - **network DNS needs explicit authorization**, and then works for the helper only;
  - **malicious answers redirect nothing:** one pointing behind the tunnel, or at a blocked host, connects nowhere, and ordinary lookups never reach the network's resolver;
  - **the resolver file is unchanged** during the window;
  - **a service killed mid-window** still re-blocks on time, with no service running;
  - **window limits:** asking again restarts, the fourth window per connection is refused, every window is logged, and disconnect ends it.
- **Install test:** the helper account is created on install, the installed rules are scoped to it, and uninstall removes it.

**Recorded 2026-10-17** in this workspace (Linux x86_64, Rust 1.97, nftables 1.0.9, no IPv6 in this kernel): **51 passed, 0 failed, 3 skipped** (the IPv6 cases, which CI requires).

**No real traffic leaves the lab:** `1.1.1.1` exists only as an address inside a throw-away namespace.
