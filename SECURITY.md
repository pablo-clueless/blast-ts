# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x | ✅ |

Blast is in early development; only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not** report security vulnerabilities through public GitHub issues.

Instead, report them privately via one of:

- **GitHub Security Advisories** — use the ["Report a vulnerability"](https://github.com/pablo-clueless/blast-ts/security/advisories/new) button on the repository
- **Email** — smsnmicheal@gmail.com

Include as much of the following as you can:

- A description of the vulnerability and its impact
- Steps to reproduce (a minimal `blast.config.json` helps)
- Affected version or commit
- Any suggested fix, if you have one

You can expect an acknowledgement within **72 hours** and a status update within **7 days**. Please give us a reasonable window to ship a fix before disclosing publicly.

## Scope and usage reminder

Blast is a load-testing and traffic-generation tool. **Only point it at systems you own or have explicit permission to test.** Running load tests against third-party services without authorization may violate their terms of service and applicable law. Issues arising from unauthorized use are out of scope for this policy.

## Handling secrets

- Never commit real credentials or tokens in `blast.config.json` — use seed/test accounts.
- Redact secrets before sharing configs in bug reports.
