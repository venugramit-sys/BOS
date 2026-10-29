with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    text = f.read()

has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

old_part_1 = """      this.fieldErrors = errMap;
      this.error = `Fill in the required fields: ${missingControls.map((c) => c.caption || c.name).join(", ")}`;"""

new_part_1 = """      this.fieldErrors = errMap;
      // Single channel: in-field errors highlight the missing controls directly; suppress top banner
      this.error = "";"""

assert old_part_1 in text_lf, "old_part_1 not found"
text_lf = text_lf.replace(old_part_1, new_part_1, 1)

old_part_2 = """    else if (r.kind === "fields") {
      this.fieldErrors = { ...this.fieldErrors, ...r.fields };
      const firstKey = Object.keys(r.fields)[0];
      if (firstKey && typeof document !== "undefined") {
        setTimeout(() => {
          const el = document.querySelector(
            `[data-field="${firstKey}"] input, [data-field="${firstKey}"] textarea, [data-field="${firstKey}"] select, [data-field="${firstKey}"] [role="combobox"], [data-key="${firstKey}"] input, input[name="${firstKey}"]`
          );
          el?.focus?.();
          el?.scrollIntoView?.({ behavior: "smooth", block: "center" });
        }, 50);
      }
    }
    this.error = r.message;"""

new_part_2 = """    else if (r.kind === "fields") {
      this.fieldErrors = { ...this.fieldErrors, ...r.fields };
      // Single channel: in-field errors highlight the fields directly; suppress top banner
      this.error = "";
      const firstKey = Object.keys(r.fields)[0];
      if (firstKey && typeof document !== "undefined") {
        setTimeout(() => {
          const el = document.querySelector(
            `[data-field="${firstKey}"] input, [data-field="${firstKey}"] textarea, [data-field="${firstKey}"] select, [data-field="${firstKey}"] [role="combobox"], [data-key="${firstKey}"] input, input[name="${firstKey}"]`
          );
          el?.focus?.();
          el?.scrollIntoView?.({ behavior: "smooth", block: "center" });
        }, 50);
      }
    } else {
      this.error = r.message;
    }"""

assert old_part_2 in text_lf, "old_part_2 not found"
text_lf = text_lf.replace(old_part_2, new_part_2, 1)

if has_crlf:
    text_lf = text_lf.replace('\n', '\r\n')

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8') as f:
    f.write(text_lf)

print("page.svelte.js updated successfully")
