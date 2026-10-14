# Septuvo — architecture blueprint (draft)

**This is a design target, not proof of implementation or cryptographic security.** See SECURITY_MODEL.md, DECISIONS.md and PROTOCOL_REVIEW_CHECKLIST.md.

## User experience comes first

A normal user installs **Septuvo VPN**, selects **Connect**, **Host my own** or **Organization**, verifies the server's identity, then enrolls by QR code / short-lived invitation. The application obtains the minimum needed OS permissions and configures networking with clear explanations. It must not require users to compile crates, choose ciphers, edit OS routes or provision a database.

For **Septuvo Server**, the same guided install starts a standalone gateway on a supported host. Optional enterprise coordination/identity management is installed/managed separately by deployment tooling—not forced onto a personal server. A private home setup does not call a vendor control plane.

## Trust and execution boundaries

~~~text
  [Unprivileged UI / optional CLI]   [Optional organization control plane]
                | signed local API                 | authenticated scoped API
                v                                  v
        [Septuvo node / privileged service: Rust]
          | identity & enrollment  | policy + routes + DNS + kill switch
          | session control        | key epochs / peer authentication
          | independent SVP state machine
          v
        [IP TUN interface] <--> [AEAD packet engine] <--> [UDP socket]
                                                        |
                                                HOSTILE UNDERLAY
                                              (Wi-Fi/LAN/cellular)

   Standalone: node + local approval store. No SaaS dependency.
   Large deployments: additional control plane, directory & relays.
~~~

- The unprivileged UI never receives private session secrets and cannot directly bypass policy.
- Privileged system changes (network interfaces, routes, firewalls, DNS, installs) must be transactional and recoverable after crash/uninstall.
- Trust established by authenticated enrolled identity, **not** by the source address, SSID, Ethernet adapter or RFC1918 prefix.
- Cryptographic implementations and sensitive policy enforcement are Rust-owned; any OS API/native framework dependency must be inventoried.
- The data plane must be able to continue existing authorized sessions through short control-plane outages under finite, documented policy lifetimes; no infinite credential validity.

## Protocol and packet-engine direction (pending G1)

- Layer 3: IPv4 and IPv6 packet tunneling over UDP. No Ethernet bridge or LAN broadcast extension in v1.
- Authenticated hybrid classical/post-quantum handshake **candidate**, transcript-bound identity and negotiation.
- AES-256-GCM **preferred candidate**, runtime acceleration only through approved implementations. ChaCha20-Poly1305 is a separate algorithm candidate, **not** a fallback for ML-KEM.
- Data frames should carry only required session/version/epoch/replay metadata, but **no layout is authorized yet**.
- Design for migration across Wi-Fi/cellular, peer endpoint changes, key rollover, packet loss and device sleep without claiming session continuity until measured.
- Device identities and initial pairing must not embed private keys in QR codes; invitations expire and are single-use or limited-use as specified.
- No unauthenticated 0-RTT application data by default.

## Networking and policy model

Default = protected traffic through an authenticated VPN path, with **fail-closed OS enforcement** if path fails. The underlay must still allow only the minimal traffic needed to establish the VPN (e.g., link configuration and approved server endpoint/relay); those exceptions are explicitly modeled and tested.

Approved LAN mode = owner permits **specific destinations or subnets** to use the physical network. A local network remains untrusted and applications may need TLS/SMB signing/etc. Avoid a blanket “allow all private networks” option as an insecure default.

Custom split mode = explicit protected/exempt prefixes (and optionally apps only where OS supports it), route priorities, DNS policy and deterministic conflict resolution. Both 0.0.0.0/0 and the pair 0.0.0.0/1 + 128.0.0.0/1 may be represented where useful; **route tricks are not a kill switch**. Equivalent IPv6 policy needs separate implementation; no IPv6 bypass.

Two use cases must be validated independently:
1. **At home while tunneled:** direct authorized local NAS/Jellyfin access, encrypted internet access, no accidental leak.
2. **Away from home:** authenticated remote home-subnet access through a home gateway, internet via another configured VPN exit if desired, safe handling of overlapping subnets.

Explain “interface initialized”, “handshake authenticated”, “path checked”, “DNS checked” separately. Do not show a vague “Connected” solely because TUN exists.

## Deployment profiles

| Profile | Infrastructure | Mandatory complexity |
| --- | --- | --- |
| Personal | One self-hosted Septuvo Server + devices | Local state; no central database/cloud |
| Organization | Multiple nodes/gateways plus optional auth/policy manager | Per-device authorization, SSO, admin roles, audit records |
| Provider | Regionally distributed gateway clusters + HA control plane | Tenant isolation, enrollment automation, rate limits, capacity routing, abuse control, service APIs |

- Encryption flows directly between data-plane peers where possible; a coordination server should not automatically decrypt data.
- Tenant identity, keys, policies, IP allocation and logging must be isolated. Never assume unique IP addresses are a sufficient security boundary.
- For roaming and HA, preserve authenticated session ownership and prevent replay across failover or worker migration.
- Capacity planning measures concurrent sessions, packets/s, Gbps, latency, RAM, CPU and recovery; no arbitrary “one node handles 10,000 users” promises.

## Client platforms and Rust policy

Start with isolated Linux-to-Linux development and official TUN facilities; **do not create a kernel driver in the first phase**. Later adapt OS-native VPN APIs on Windows, macOS, Android and iOS, with a single Rust security/session core. A fully Rust-authored UI is a goal; platform service glue and third-party OS components must be disclosed, not disguised as Rust code.

UI must support resizing/scrolling on 1920x1080 and smaller windows, clear failures, safe background connection persistence, explicit disconnect/quit behavior, secure update handling and accessibility.

## Observability, privacy and operations

Local diagnostics report connectivity stage, route decision, DNS test, packet counters and cryptographic session status **without logging secrets or user traffic**. Telemetry must not be silently enabled. Key rotation, backups, recovery, service restart, upgrade and uninstall must preserve network protections and avoid corrupting host routes/firewalls.

## Explicit v1 non-goals

No Layer 2 bridge, custom symmetric primitives, anonymous-network claims, mandatory cloud, exotic multipath, customer billing engine, and no “quantum-proof” or FIPS-certified claim without verified basis.

Changes to trust boundaries, OS privileges, cryptography, or transport require decision register update and owner approval.
