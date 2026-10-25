# Septuvo security policy

## Project state

Septuvo is **pre-implementation**. There is no installable VPN, audited SVP specification or implementation, production release, or supported version. The repository's tagline is an aspiration, **not an assurance of security**.

## Reporting a potential vulnerability

If you discover an issue affecting future Septuvo code, cryptographic design, dependencies, enrollment, credentials, or network policy:

1. **Do not post exploitable details, live credentials, private keys, access tokens, or affected user data in public GitHub issues, PRs, or discussions.**
2. Report privately through GitHub: open the repository's **Security** tab and choose **Report a vulnerability**. GitHub private vulnerability reporting is enabled for this repository (verified 2026-10-14). No dedicated security mailbox exists; **do not assume one does**.
3. Provide a minimal description, affected commit/version/platform, impact, reproduction in a private lab, and any proposed mitigation.

If private reporting is unavailable for any reason, **do not disclose secrets or exploit details publicly**; open a minimal public issue asking for a secure reporting channel, without technical details.

## Response expectations

The maintainer will triage reported issues according to impact and whether an implementation exists. There is **no promised response SLA** yet. Security issues should be tracked privately, investigated reproducibly, remediated, and retested before public disclosure as appropriate.

## AI-assisted audits

Septuvo plans repeated AI-assisted adversarial audits with Cloudflare's `security-audit` skill, pinned to a reviewed revision ([docs/security/AI_ASSISTED_AUDIT.md](docs/security/AI_ASSISTED_AUDIT.md), D-044). Each run needs the owner's explicit authorization for a named commit. Their findings are handled through the private channel above and are never published automatically. **They supplement, and never replace, the qualified independent reviews that gates G1 and G3 require; a clean run is not evidence of security.**

## Release and support rules

No version is supported or production-ready until the production release gate **G3** in [the engineering roadmap](docs/ROADMAP.md). A passing CI job is not an independent audit. Material handshake or protocol security changes require qualified external review and owner approval under the [agent rules](AGENTS.md).
