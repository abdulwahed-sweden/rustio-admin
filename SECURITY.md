# Security Policy

RustIO Admin includes authentication, sessions, roles, recovery and audit functionality. Security reports are therefore treated separately from ordinary bugs and feature requests.

## Reporting a vulnerability

Please **do not open a public GitHub issue** for a suspected security vulnerability.

Report it privately to:

**abdulwahed.sweden@gmail.com**

Include, when possible:

- the affected RustIO Admin version or commit;
- a clear description of the vulnerability;
- reproduction steps or a minimal proof of concept;
- the affected component or crate;
- the security boundary involved, such as authentication, authorization, recovery, sessions, audit, CSRF, token handling or generated admin behavior;
- any disclosure timeline you need us to consider.

Do not send real production credentials, private keys, access tokens, personal data or other secrets unless absolutely necessary. Prefer synthetic test data.

## Scope

Reports are especially welcome for issues involving:

- authentication or session bypass;
- privilege escalation or role-boundary failures;
- authorization checks that can be bypassed through an alternate route;
- account recovery or credential-reset weaknesses;
- token, password or secret handling;
- audit records that can be suppressed, rewritten or attributed incorrectly;
- cross-user or cross-tenant data exposure;
- unsafe generated admin behavior;
- injection or request-forgery vulnerabilities in framework-controlled surfaces;
- dependency or build-chain vulnerabilities that directly affect users of the published project.

Ordinary correctness bugs, documentation problems and feature requests should use the public issue tracker unless they have a security impact.

## Supported versions

RustIO Admin is pre-1.0 and evolves quickly. Security fixes are focused on the latest published release line. If you are using an older release, please confirm whether the issue also reproduces on the latest available version when practical.

## Disclosure and credit

Please allow reasonable time to reproduce, fix, test and publish a security release before public disclosure.

With your consent, we can credit security reporters in release notes or project acknowledgements.
