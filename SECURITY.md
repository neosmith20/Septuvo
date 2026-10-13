# Septuvo security policy

## Project state

Septuvo is **pre-implementation**. There is no installable VPN, audited SVP protocol, production release, or supported version. The repository's tagline is an aspiration, **not an assurance of security**.

## Reporting a potential vulnerability

If you discover an issue affecting future Septuvo code, cryptographic design, dependencies, enrollment, credentials, or network policy:

1. **Do not post exploitable details, live credentials, private keys, access tokens, or affected user data in public GitHub issues, PRs, or discussions.**
2. Contact the repository maintainer through a previously established private channel and request a confidential disclosure process. No dedicated security mailbox has been verified; **do not assume one exists**.
3. If GitHub Security Advisories/private reporting are enabled for the repository, follow the reporting path shown on the repository's Security tab.
4. Provide a minimal description, affected commit/version/platform, impact, reproduction in a private lab, and any proposed mitigation.

If there is no established private contact route, **do not disclose secrets publicly**; request a secure reporting channel without including exploit details.

## Response expectations

The maintainer will triage reported issues according to impact and whether an implementation exists. There is **no promised response SLA** yet. Security issues should be tracked privately, investigated reproducibly, remediated, and retested before public disclosure as appropriate.

## Release and support rules

No version is supported or production-ready until GitHub release approval gate **G3** in [the engineering roadmap](docs/ROADMAP.md). A passing CI job is not an independent audit. Material handshake or protocol security changes require qualified external review and owner approval under the [agent rules](AGENTS.md).
