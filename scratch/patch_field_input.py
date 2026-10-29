with open('ui/src/lib/page-kit/FieldInput.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''      <Input
        {id}
        name={f.key || c.name}
        type={f.control === "email" ? "email" : f.control === "phone" ? "tel" : f.control === "url" ? "url" : f.control === "masked" ? "password" : "text"}
        prefixSnippet={customIcon ? inputPrefix : null}
        bind:value={() => value ?? "", (v) => write(v)}
        maxlength={f.max_length}
        errorMessage={error}
        required={f.required}
        placeholder={f.placeholder ?? (isDesignControl ? (f.caption || c.caption || label || "") : "")}
        readonly={isDesignControl}
        size={compact ? "sm" : null}
      />'''

replacement = '''      <Input
        {id}
        name={f.key || c.name}
        type={f.control === "email" ? "email" : f.control === "phone" ? "tel" : f.control === "url" ? "url" : f.control === "masked" ? "password" : "text"}
        autocomplete={(f.key || c.name) === "username" ? "username" : ((f.control === "masked" || (f.key || c.name) === "password") ? "current-password" : undefined)}
        prefixSnippet={customIcon ? inputPrefix : null}
        bind:value={() => value ?? "", (v) => write(v)}
        maxlength={f.max_length}
        errorMessage={error}
        required={f.required}
        placeholder={f.placeholder ?? (isDesignControl ? (f.caption || c.caption || label || "") : "")}
        readonly={isDesignControl}
        size={compact ? "sm" : null}
      />'''

assert target in content, "target not found in FieldInput.svelte"
content = content.replace(target, replacement)

with open('ui/src/lib/page-kit/FieldInput.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
print("FieldInput.svelte updated successfully!")
