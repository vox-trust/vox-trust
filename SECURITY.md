# Security policy

Vox Trust is **pre-alpha**. The specification is a draft and the code has not been reviewed or audited. Do not rely on it to protect anyone.

## Reporting a vulnerability

Please report vulnerabilities **privately** using GitHub's private vulnerability reporting:

<https://github.com/vox-trust/vox-trust/security/advisories/new>

Do not open a public issue for a vulnerability. Include what you found, how to reproduce it, and which part of the spec or code is affected. This is a volunteer project: there is no response-time guarantee and no bug bounty yet, but reports are read and credited (if you wish).

## What is a vulnerability, and what is design feedback

- **Vulnerability (report privately):** a way to forge a seal that verifies, a replay or splice that passes the policy, a flaw in the reference code that breaks a stated guarantee.
- **Design feedback (open a public issue):** criticism of the threat model, of the draft spec, or of a claim in the documentation. Public scrutiny of the design is welcome.

The attackers and limits we already know about are listed in [spec/THREAT-MODEL.md](spec/THREAT-MODEL.md).
