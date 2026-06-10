# Email — Conventions for Framework-Emitted Messages

The contract for every email the framework dispatches on a
project's behalf — recovery emails today, MFA enrolment + security
alerts + "password changed" confirmation tomorrow. Governs the
visual shape, the vocabulary, the security envelope, and the
separation between framework identity (RustIO) and application
identity (the deployed product).

Companion to `DESIGN_CHROME.md` (which owns admin-surface chrome).
PR review for any new framework-emitted email is against this
document.

> **Status**
>
> Stabilised in 0.12.0 alongside the production recovery flow
> (real SMTP, polished HTML body, brand-identity architecture).
> The recovery-email path is the reference implementation; future
> email surfaces (MFA enrolment, password-changed confirmation,
> security alerts) inherit from this contract.


---


## 1. Purpose

### 1.1 What this document governs

Every email the framework composes and dispatches:

- Password-reset emails (`auth::recovery::issue_reset_token`).
- Future: password-changed confirmations, MFA enrolment receipts,
  forced-password-rotation notices, security alerts on
  authority transitions.

Governs:

- Subject-line vocabulary.
- HTML body structure (wordmark, descriptor, title, greeting,
  CTA, fine print, verification reference, security envelope,
  anti-phishing warning, signature block, operational footer).
- Plaintext body shape (always present alongside HTML).
- Identity layering — application identity is visible, framework
  identity is invisible by default.
- The data contract between business logic (recovery flow) and
  the renderer (`RecoveryEmailParts`).

### 1.2 What this document does not cover

- SMTP transport choice — projects own the `Mailer` impl. See
  `examples/clinic-appointments/src/mailer.rs` for the canonical
  `LettreSmtpMailer` reference.
- Admin chrome surfaces — `DESIGN_CHROME.md`.
- Audit trail — `DESIGN_AUDIT.md`. Email-dispatch audit rows are
  emitted by the recovery flow, not by the renderer.
- Email-template overrides at the project level — currently the
  framework owns the HTML body verbatim. A future project-side
  override layer (out of scope for 0.12.0) will get its own
  doctrine doc.


---


## 2. Invariants

The rules every framework-emitted email must honour. PRs that
violate these are rejected on principle, not on taste.

### 2.1 Plaintext-first; HTML is an alternative

Every email carries a `text/plain` body. The HTML body is an
alternative MIME part, never a replacement. Clients that strip
HTML still get the same information — link, expiry, security
envelope, "if this wasn't you" disclaimer. The `Mail` type
enforces this shape:

```rust
pub struct Mail {
    pub text_body: String,           // required
    pub html_body: Option<String>,   // optional alternative
    ...
}
```

`Mail::with_html(...)` attaches the HTML part to an existing
`framework_envelope` plaintext. Reverse construction
(HTML-only) is not supported.

### 2.2 Application identity owns the user-visible surface

The deployed project's `app_name` ("Library Circulation",
"Stockholm Clinic", "Atlas ERP") appears in:

- The subject line, *after* the action verb:
  `Reset your password — Library Circulation`.
- The wordmark at the top of the HTML body.
- The plaintext `System:` envelope line.
- The footer "You are receiving this because … on `<app_name>`".

The framework name `RustIO` does **not** appear in any of those
positions. The optional `show_powered_by` flag enables a single
low-contrast "Powered by RustIO" footer credit; default is
`false`.

### 2.3 Single concentrated point of emphasis (HTML)

Each email has exactly one saturated colour band: the CTA button
in `--rio-accent` (the teal — value in `VISUAL-CONTRACT.md` §1). Every other
surface lives in the muted-text / surface / border range. Mirrors
`DESIGN_CHROME.md §2.2` — operator focus matters in chrome;
recipient focus matters in email.

### 2.4 Security envelope is unconditional

Every recovery / authority email surfaces the four envelope
fields when known: `System`, `When` (UTC, tabular numerals),
`From IP`, `Device` (User-Agent). Renders in plaintext as a
separator block, in HTML as a labelled table. Missing fields are
omitted, never faked. The envelope serves anti-phishing parity:
a recipient who sees a real reset email can always cross-check
the IP / device / time against their own actions.

### 2.5 Anti-phishing warning is unconditional

Every email includes the "If this wasn't you — ignore this; your
password stays unchanged" warning panel near the bottom of the
body. Wording is calm, not panicked. Tells the recipient what to
do (revoke sessions on `/admin/account/sessions`) but does not
manufacture urgency.

### 2.6 Verification reference is operationally meaningful

The 6-character uppercase reference rendered inside a
security-style panel is derived from the per-request
`correlation_id` (last 6 alphanumeric chars of the UUID v7).
Operators can match the email's reference against the audit-log
row's `correlation_id` to confirm an email is genuinely the one
generated for their attempt. This is **not** a code to type — the
label explicitly says so. Future MFA verification emails may use
the same visual block with a typed numeric code; the slot is
shared, the semantics differ.

### 2.7 Greeting falls back gracefully

`Hello {greeting_name},` resolves through the documented
fallback chain: `display_name → first_name → email-local-part →
"there"`. Never `Hello user@example.com,` — that breaks the
operational tone. Never `Hello there,` when a display_name is
known.

### 2.8 No marketing language; no urgency manufacture

Forbidden phrases in any framework email:

- "Welcome to the future of …"
- "We're excited to …"
- "Hurry / Last chance / Act now"
- "Click here" (use a verb that describes the action — "Set a
  new password", "Verify your device")
- "From the team at …"
- Generic SaaS pleasantries.

Operational tone — calm, direct, fact-stated. The recipient is
recovering their account or responding to an authority event, not
being marketed to.


---


## 3. The standard recovery email shape

The reference layout, top to bottom. Future email types adapt
this skeleton; novel sections require an extension to this
document first.

```
┌──────────────────────────────────────────────────────────────┐
│  Library Circulation                                         │
│  ACCOUNT SECURITY NOTIFICATION                               │
│                                                              │
│  Reset your password                                         │
│                                                              │
│  Hello Abdulwahed,                                           │
│  We received a request to reset the password for your        │
│  Library Circulation account. Choose a new password to       │
│  continue.                                                   │
│                                                              │
│  ┌────────────────────────────┐                              │
│  │   Set a new password       │  ← brand-accent CTA          │
│  └────────────────────────────┘                              │
│                                                              │
│  Or paste this link into your browser:                       │
│  http://127.0.0.1:3000/admin/reset-password/…                │
│                                                              │
│  This link expires in 1 hour.                                │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ VERIFICATION REFERENCE                               │    │
│  │ 14C659                                               │    │
│  │ Keep this for your security records.                 │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  ──────────────────────                                      │
│  System    Library Circulation                               │
│  When      2026-05-13 14:30 UTC                              │
│  From IP   127.0.0.1                                         │
│  Device    Mozilla/5.0 (Macintosh; …)                        │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ ⚠ If this wasn't you — ignore this email …           │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
│  ACCOUNT OWNER                                               │
│  Abdulwahed Mansour                                          │
│  Principal Administrator                                     │
│  Library Circulation                                         │
└──────────────────────────────────────────────────────────────┘
   Session-aware authentication · Audit-logged · 14:30 UTC
   You are receiving this because a password reset was
   requested for your account on Library Circulation.
   Need help? Contact support@library.example.com
```

Mandatory blocks (in order): wordmark, descriptor, title,
greeting, intro, CTA, URL fallback, fine print, verification
reference, security envelope, anti-phishing warning, signature,
operational footer.

Optional blocks: `support_email` line in footer,
"Powered by RustIO" credit (opt-in only).


---


## 4. Subject-line vocabulary

Fixed verb-first patterns for known email types:

| Email type | Subject |
|---|---|
| Self-service password reset | `Reset your password — {app_name}` |
| Admin-issued password reset (R2) | `Your password was reset by an administrator — {app_name}` |
| Password-changed confirmation (future) | `Your password was changed — {app_name}` |
| MFA enrolment receipt (future) | `Two-step verification is now active — {app_name}` |
| Security alert: suspicious login (future) | `New sign-in to your account — {app_name}` |

Rules:

- Action verb leads. The recipient should know what happened from
  the inbox preview alone.
- Em-dash separates verb from app name. Never a colon, never a
  pipe.
- `{app_name}` substitutes the project's `Admin::app_name(...)`
  value. Never the framework name.
- No emoji, no trailing exclamation, no "[Action Required]"
  brackets.


---


## 5. Greeting + signature derivation

The framework reads `rustio_users` profile columns
(`first_name`, `last_name`, `display_name`, `job_title`) and
passes them through `StoredUser::greeting_name()` and
`StoredUser::signature_lines()` helpers.

### 5.1 Greeting

```
display_name → first_name → email-local-part → "there"
```

The first non-empty value wins. Used as `Hello {x},`. The
fallback to "there" is the floor — `Hello there,` is acceptable
for accounts that haven't filled in any profile field. Never
falls all the way to the full email address.

### 5.2 Signature block

Two lines drawn from the same profile columns:

| Line | Source (first non-empty wins) |
|---|---|
| Primary | `first_name + last_name` → `display_name` → email-local-part |
| Secondary (optional, italics-suppressed) | `job_title` |

Followed by a third line that's always the project's `app_name`.
The block is suppressed entirely when no profile fields are set —
the email stays compact for legacy installs rather than rendering
a half-empty placeholder.


---


## 6. The security envelope

Four labelled rows. All optional individually, but the block
header (`System`) always renders.

| Label | Source | Format |
|---|---|---|
| System | `app_name` from `SiteBranding` | Plain text |
| When | `chrono::Utc::now()` at dispatch | `YYYY-MM-DD HH:MM UTC`, tabular numerals |
| From IP | Request's `x-forwarded-for` / `x-real-ip` / connection-peer | Raw IPv4 / IPv6 string |
| Device | Request's `User-Agent` header | Raw header value, no parsing |

No User-Agent parser. The raw header is the truth; framework
doesn't pretend to know the device model. Readers who care
inspect the raw string; readers who don't, skip it. Adding a UA
parser is rejected on the same principle as DESIGN_CHROME.md §7:
introducing a third-party "knowledge" layer over an honest
string trades truth for ergonomics.


---


## 7. The verification reference panel

A 6-character uppercase code rendered inside a security-style
inner card, with explicit labelling.

### 7.1 Derivation

```rust
let stripped = correlation_id
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .collect::<String>();
let code = stripped[stripped.len().saturating_sub(6)..]
    .to_ascii_uppercase();
```

Deterministic — the same `correlation_id` always yields the same
code. Operators can grep the audit log for that correlation_id
and match the email exactly.

### 7.2 Why 6 characters

- Enough entropy to make spoofing visible (~2³⁰ if alphanumeric).
- Short enough to read aloud over the phone for a support
  interaction.
- Long enough to look like a real verification reference.

### 7.3 Why it's not a typed code (yet)

The framework's recovery flow uses link-based consume, not code-
based. The reference panel is a *visual* security artifact today.
The slot is reserved for a future MFA verification flow that will
populate the same panel with a typed numeric code — at that
point the reader doctrine flips from "match this against the
audit log" to "type this into the verify page." The panel label
disambiguates: "Verification reference" today, "Verification
code" when MFA emails arrive.


---


## 8. Anti-phishing warning panel

Always present, always near the bottom of the body, always in the
amber warning tone (`#FFF8EB` background, `#F2D9A7` border,
`#6B4F12` text). Wording matches:

> **If this wasn't you** — ignore this email. Your password stays
> unchanged, and the link above will expire on its own. You can
> also sign in and revoke open sessions from the Sessions page.

Rules:

- Don't manufacture urgency. The user's correct response is to
  ignore the email; the framework already enforces TTL +
  single-use semantics on the token.
- Always tell the user what to do if they're concerned. "Revoke
  open sessions from the Sessions page" is the actionable
  fallback.
- No phone numbers, no support-line scripting. Direct the user
  to in-product surfaces (`/admin/account/sessions`).


---


## 9. App identity vs framework identity

The framework is invisible to end users by default. The deployed
application owns every visible surface.

### 9.1 What the project supplies

Via `Admin::app_name(...)`, `Admin::app_tagline(...)`,
`Admin::support_email(...)`, `Admin::public_url(...)`:

| Field | Used in |
|---|---|
| `app_name` | Subject, wordmark, plaintext `System:`, security envelope, footer attribution |
| `app_tagline` | Descriptor under wordmark (falls back to "Account security notification") |
| `support_email` | Footer "Need help? Contact …" line |
| `public_url` | Reset-link base URL (overrides request-header derivation) |

### 9.2 What the framework supplies

The shape, the typography, the security envelope, the anti-
phishing panel, the verification reference — every doctrine in
this document. Projects override by *passing different
`Admin::app_*` values*, not by re-templating the email.

A project-side template-override layer is **not** in 0.12.0. If
that need surfaces, the design has its own doctrine doc as a
precondition.

### 9.3 The opt-in framework credit

`Admin::show_powered_by(true)` renders a single low-contrast
"Powered by RustIO" line in the email footer (and the admin
chrome — same flag governs both). Off by default. Projects that
want to attribute the framework opt in explicitly; everyone else
gets a framework-invisible product.


---


## 10. Plumbing — `RecoveryEmailParts` + Admin builders

The framework's recovery dispatcher reads `Admin::branding()` +
the loaded `StoredUser` and assembles a `RecoveryEmailParts`
struct. The renderer is pure — same inputs always produce the
same bytes.

```rust
let parts = email::RecoveryEmailParts {
    app_name: &admin.branding().app_name,
    app_tagline: admin.branding().app_tagline.as_deref(),
    title: "Reset your password",
    greeting_name: &user.greeting_name(),
    intro: &format!("We received a request to reset the password for your {app_name} account."),
    cta_label: "Set a new password",
    cta_url: &reset_link,
    fine_print: &format!("This link expires {ttl_human}."),
    when: chrono::Utc::now(),
    request_ip: Some(&client_ip),
    ua_summary: user_agent_owned.as_deref(),
    correlation_id: req_correlation_id,
    signature_primary: Some(&sig_primary),
    signature_title: sig_title.as_deref(),
    support_email: admin.branding().support_email.as_deref(),
    show_powered_by: admin.branding().show_powered_by,
};
let html = email::render_recovery_html(parts);
let mail = Mail::framework_envelope(...).with_html(html);
```

`RecoveryEmailParts::new(app_name, title, greeting_name, intro,
cta_url, fine_print, when)` is the constructor for external
crates (the CLI's `doctor email --html-preview` uses it).
Optional fields default to `None`; callers mutate by field
assignment.

The struct is `#[non_exhaustive]` — adding fields is non-breaking
for external crates that use the constructor + field-mutation
pattern.


---


## 11. What framework-emitted emails must never do

Hard refusals. PRs that introduce any of these are sent back.

- **No tracking pixels.** Not in HTML, not in image links, not
  anywhere. Framework emails are operational, not marketing.
- **No external `<img>` tags.** Apart from data-URI embedded
  pixels (also forbidden — see above), every image would be a
  fetch from a third-party origin. Recipients' clients would leak
  the open. Use Unicode characters or CSS for any visual element.
- **No JavaScript.** Most clients strip it; the ones that don't
  are a security-attack surface.
- **No CDN-hosted CSS, fonts, or assets.** Inline every byte
  needed for the email to render. Web fonts especially — most
  clients ignore them; a few cache them poorly.
- **No reply-to addresses that lead nowhere.** Either omit the
  `Reply-To` header (the framework default) or set it to the
  project's `support_email`. Never `noreply@…` — that's
  hostile communication.
- **No conditional content based on the client.** The email is
  the same for Gmail / Apple Mail / Outlook / Thunderbird. Best-
  effort rendering, not split brand.
- **No "click here to unsubscribe".** Framework emails are
  authority transitions on accounts the user owns — they cannot
  unsubscribe from a password reset they didn't ask for.
- **No deceptive sender names.** `From:` is always
  `{app_name} <{mail_from}>` exactly. Never a person's name that
  isn't the actor on the action (impersonation surface).
- **No localisation in this commit.** Email body is currently
  English-only. A future localisation pass will need its own
  doctrine — translating the security envelope without losing
  meaning is non-trivial.


---


## 12. Open work

Anchored here so future email surfaces stay coherent with this
document.

- **Password-changed confirmation email** — emitted on consume
  of a reset token. Mirrors the issue email's structure with a
  different subject ("Your password was changed — {app_name}")
  and no CTA. Reuses the security envelope + anti-phishing
  panel verbatim. Out of scope for 0.12.0; doctrine sanctioned.
- **MFA enrolment receipt** — emitted when a user adds a TOTP
  factor. Same shape, includes a backup-code summary block
  (count + last-rotated date, NOT the codes themselves).
- **Security alert: suspicious login** — emitted when login
  succeeds from a new IP / new UA / new geolocation. Same shape;
  CTA points to `/admin/account/sessions` with a `?revoke=N`
  pre-fill for the suspicious session.
- **Per-organisation email customisation** — for multi-tenant
  projects that want different brands per org. Requires extending
  `SiteBranding` lookup to be tenant-scoped. Larger architectural
  decision; out of scope until the multi-tenant story stabilises.
- **Email-template project override** — letting projects ship
  their own HTML templates that the framework renders. Bigger
  framework change (requires a template-loader precedence design
  + a public template-context contract). Has its own doctrine
  prerequisite before any code lands.

Each item above is a separate doctrine-light commit on top of
this contract — same shape, different content, all four
invariants (§2) preserved.
