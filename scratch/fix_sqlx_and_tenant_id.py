# 1. Update Cargo.toml
with open("Cargo.toml", "r", encoding="utf-8") as f:
    cargo_content = f.read()

target_sqlx = 'sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json"] }'
replacement_sqlx = 'sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json", "rust_decimal"] }'

assert target_sqlx in cargo_content, "target_sqlx not found in Cargo.toml"
cargo_content = cargo_content.replace(target_sqlx, replacement_sqlx, 1)

with open("Cargo.toml", "w", encoding="utf-8") as f:
    f.write(cargo_content)
print("Updated Cargo.toml sqlx features")

# 2. Update crates/bosc-runtime/src/lib.rs
with open("crates/bosc-runtime/src/lib.rs", "r", encoding="utf-8") as f:
    lib_content = f.read()

target_t1 = """                    company_id: None,

                    tenant_id,"""

replacement_t1 = """                    company_id: None,

                    tenant_id: Some(tenant_id),"""

assert target_t1 in lib_content, "target_t1 not found"
lib_content = lib_content.replace(target_t1, replacement_t1)
print("Updated tenant_id in lib.rs")

with open("crates/bosc-runtime/src/lib.rs", "w", encoding="utf-8") as f:
    f.write(lib_content)
print("Saved lib.rs")
