# ADR 0009: SecretProvider / KeyProvider abstraction
Status: Accepted. Date: 2026-10-24. Plan ref: sec 5.7, 5.6.

## Decision
```rust
#[async_trait]
pub trait SecretProvider: Send + Sync {
    fn name(&self) -> &'static str;                       // "file" | "env" | "openbao" | "cloud-kms"
    async fn health(&self) -> ProviderHealth;             // Ok | Sealed | Unreachable | Misconfigured
    async fn get(&self, r: &SecretRef) -> Result<SecretValue, SecretError>;
    async fn put(&self, r: &SecretRef, v: SecretValue) -> Result<u32, SecretError>;
    async fn delete(&self, r: &SecretRef) -> Result<(), SecretError>;
    async fn lease_db_credentials(&self, role: &str) -> Result<Lease<DbCredentials>, SecretError>;
    async fn renew(&self, lease_id: &str) -> Result<Duration, SecretError>;
}
#[async_trait]
pub trait KeyProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn health(&self) -> ProviderHealth;
    async fn create_key(&self, key_id: &str, kind: KeyKind) -> Result<u32, KeyError>;       // Aes256Gcm | Ed25519
    async fn encrypt(&self, key_id: &str, pt: &[u8], aad: &[u8]) -> Result<Ciphertext, KeyError>;
    async fn decrypt(&self, key_id: &str, ct: &Ciphertext, aad: &[u8]) -> Result<Zeroizing<Vec<u8>>, KeyError>;
    async fn rewrap(&self, key_id: &str, ct: &Ciphertext) -> Result<Ciphertext, KeyError>;
    async fn sign(&self, key_id: &str, digest: &[u8; 32]) -> Result<(Vec<u8>, u32), KeyError>;   // (sig, key_version)
    async fn public_key(&self, key_id: &str, version: u32) -> Result<Vec<u8>, KeyError>;
    async fn rotate(&self, key_id: &str) -> Result<u32, KeyError>;
}
```
- `SecretRef` syntax: `env:NAME`, `file:<relative path>#<key>`, `bao:<mount>/<path>#<field>`. Config holds references only, never values.
- Implementations: `FileProvider` (development only, under `.bos/dev-secrets/`; staging/production refuse it at startup), `OpenBaoProvider` (KV v2, Transit, database engine), `CloudKmsProvider` (optional for keys only; **no cloud auto-unseal** per owner decision).
- Selected with `[secrets] provider = "file|openbao|cloud-kms"`. Packages use only `ctx.kernel.secrets.get(ref)` behind a capability check.
- Failure behaviour per function is as in plan sec 5.6: fail closed in staging/production.

## Consequences
OpenBao can be swapped out without touching callers, and dev needs no OpenBao.

## Alternatives considered
Calling OpenBao directly everywhere (lock-in, and dev would need it).

## Reversibility
REVERSIBLE (internal trait), but changing the `SecretRef` syntax is COSTLY once configs use it.

## Phase / enforcement
P0: traits + `FileProvider`. P2: `OpenBaoProvider` required in staging/production.