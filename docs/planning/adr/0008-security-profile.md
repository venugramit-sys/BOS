# ADR 0008: `security_profile` matrix and resolution order
Status: Accepted. Date: 2026-10-24. Plan ref: sec 2 (matrix 2.2, approved decisions sec 7).

## Decision
- `enum SecurityProfile { Development, Staging, Production }`, written as `development|staging|production` (case-insensitive on input).
- **Resolution order:** CLI `--security-profile` (or `--dev`) > env `BOS_SECURITY_PROFILE` > `bos-node.toml` `[security] profile` > **default `production`**. An invalid value is a startup error (exit 78), never a fallback.
- Resolved **once** at startup into an immutable value. There's no API, Studio, package or tenant way to change it, and packages can only read it.
- Matrix (summary; full in plan sec 2.2):

| Control | development | staging | production |
|---|---|---|---|
| Bind | 127.0.0.1 / ::1 only | any | any + TLS |
| Login | dev stand-in auto-login (ADR 0004) | required | required |
| TOTP | off (stand-in) | **required for everyone incl. QA** (owner decision) | required |
| Emergency/fallback login | off | CLI break-glass only | CLI break-glass only |
| Packages | unsigned allowed | signed | signed, trusted, in scope |
| Secrets | file/env | OpenBao | OpenBao (no cloud auto-unseal) |
| DB role | superuser allowed | `bos_app` | D9 roles |
| CORS | localhost origins | allow-list | allow-list, no `*` |
| DB marker | must be `development` | must not be `development` | must be `production` |
| Banner | `X-BOS-Profile: development` + red UI bar | header + amber bar | none |

## Consequences
One switch with well-defined behaviour. A missing setting gives production, so a forgotten config can't expose a dev node.

## Alternatives considered
Separate feature flags per control (too many combinations); default development (rejected by owner).

## Reversibility
REVERSIBLE for the matrix rows (new ADR); the resolution order is fixed.

## Phase / enforcement
P0: enum, resolution, banners, bind rule, DB marker, startup-check framework. Later phases turn on each row's controls.