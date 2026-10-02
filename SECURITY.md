# Security policy

Vox Trust is **pre-1.0**: specification version 0.2 (file mode a release candidate, in-band parts experimental), software v0.x. The code has not yet been reviewed or audited by anyone but the author. Do not rely on it to protect anyone.

**Supported versions:** only the latest release on `main`. **Disclosure:** we aim to acknowledge within 7 days and to publish a fix and advisory within 90 days of a valid report; this is a volunteer project, so these are goals, not guarantees. **Scope:** the code and specification in this repository and the demo at vox-trust.github.io/demo/. Out of scope: attacks that need the victim's key or device (see the threat model).

## Reporting a vulnerability

Please report vulnerabilities **privately** using GitHub's private vulnerability reporting:

<https://github.com/vox-trust/vox-trust/security/advisories/new>

If that page is not available to you, open a minimal public issue titled "security contact request" **without any details** and a maintainer will arrange a private channel.

Do not open a public issue for a vulnerability. Include what you found, how to reproduce it, and which part of the spec or code is affected. This is a volunteer project: there is no response-time guarantee and no bug bounty yet, but reports are read and credited (if you wish).

## What is a vulnerability, and what is design feedback

- **Vulnerability (report privately):** a way to forge a seal that verifies, a replay or splice that passes the policy, a flaw in the reference code that breaks a stated guarantee.
- **Design feedback (open a public issue):** criticism of the threat model, of the specification, or of a claim in the documentation. Public scrutiny of the design is welcome.

The attackers and limits we already know about are listed in [spec/THREAT-MODEL.md](spec/THREAT-MODEL.md).
