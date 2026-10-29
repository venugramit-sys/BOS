import re

def remove_group_secondary(filepath):
    with open(filepath, "r", encoding="utf-8") as f:
        lines = f.readlines()
    
    new_lines = []
    in_secondary = False
    depth = 0
    
    for line in lines:
        if "group(Secondary)" in line:
            in_secondary = True
            depth = 1
            continue
        
        if in_secondary:
            depth += line.count("{") - line.count("}")
            if depth <= 0:
                in_secondary = False
            continue
        
        # Also remove any leftover diagnostics/feedback items if outside group
        if "item(Diagnostics)" in line or "item(Documentation)" in line or "item(Feedback)" in line or "item(Support)" in line:
            in_secondary = True
            depth = 1
            continue

        new_lines.append(line)

    with open(filepath, "w", encoding="utf-8", newline="\n") as f:
        f.writelines(new_lines)
    print(f"Processed {filepath}")

remove_group_secondary("packages/bos.auth/auth.bos")
remove_group_secondary("packages/bos.auth/package.bos")
