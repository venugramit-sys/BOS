file_path = "crates/bosc-codegen-db/src/lib.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 3. Update AOT prepared statements GetById, Exists, UpdateOptimistic, Delete, SIFT to IS NOT DISTINCT FROM
target_getbyid_comp = 'WHERE "{}" = $1 AND "_tenant_id" = $2 AND "_company_id" = $3 LIMIT 1;'
replacement_getbyid_comp = 'WHERE "{}" = $1 AND "_tenant_id" IS NOT DISTINCT FROM $2 AND "_company_id" IS NOT DISTINCT FROM $3 LIMIT 1;'
assert target_getbyid_comp in content, "target_getbyid_comp not found"
content = content.replace(target_getbyid_comp, replacement_getbyid_comp)
print("Updated GetById company statement")

target_getbyid_tbl = 'WHERE "{}" = $1 AND "_tenant_id" = $2 LIMIT 1;'
replacement_getbyid_tbl = 'WHERE "{}" = $1 AND "_tenant_id" IS NOT DISTINCT FROM $2 LIMIT 1;'
assert target_getbyid_tbl in content, "target_getbyid_tbl not found"
content = content.replace(target_getbyid_tbl, replacement_getbyid_tbl)
print("Updated GetById table statement")

target_exists = 'WHERE "{}" = $1 AND "_tenant_id" = $2);'
replacement_exists = 'WHERE "{}" = $1 AND "_tenant_id" IS NOT DISTINCT FROM $2);'
assert target_exists in content, "target_exists not found"
content = content.replace(target_exists, replacement_exists)
print("Updated Exists statement")

target_update = 'WHERE "{}" = ${} AND "_system_version" = ${} AND "_tenant_id" = ${} RETURNING *;'
replacement_update = 'WHERE "{}" = ${} AND "_system_version" = ${} AND "_tenant_id" IS NOT DISTINCT FROM ${} RETURNING *;'
assert target_update in content, "target_update not found"
content = content.replace(target_update, replacement_update)
print("Updated UpdateOptimistic statement")

target_delete = 'WHERE "{}" = $1 AND "_system_version" = $2 AND "_tenant_id" = $3 RETURNING *;'
replacement_delete = 'WHERE "{}" = $1 AND "_system_version" = $2 AND "_tenant_id" IS NOT DISTINCT FROM $3 RETURNING *;'
assert target_delete in content, "target_delete not found"
content = content.replace(target_delete, replacement_delete)
print("Updated Delete statement")

target_sift = 'WHERE "_tenant_id" = $1;'
replacement_sift = 'WHERE "_tenant_id" IS NOT DISTINCT FROM $1;'
assert target_sift in content, "target_sift not found"
content = content.replace(target_sift, replacement_sift)
print("Updated SIFT statement")

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)

print("Finished AOT prepared statements update.")
