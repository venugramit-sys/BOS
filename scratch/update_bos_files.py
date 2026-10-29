for path in ['packages/bos.auth/auth.bos', 'packages/bos.auth/package.bos']:
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()

    # In RoleCard, make code editable on create
    target1 = '                field(code) {\n                    caption = "Role Code";\n                    editable = false;\n                }'
    repl1 = '                field(code) {\n                    caption = "Role Code";\n                }'
    
    # In UserCard, make username editable on create
    target2 = '                field(username) {\n                    editable = false;\n                }'
    repl2 = '                field(username) {\n                    caption = "Username";\n                }'
    
    # Handle both CRLF and LF
    content = content.replace(target1.replace('\n', '\r\n'), repl1.replace('\n', '\r\n'))
    content = content.replace(target1, repl1)
    content = content.replace(target2.replace('\n', '\r\n'), repl2.replace('\n', '\r\n'))
    content = content.replace(target2, repl2)

    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)

print('Successfully updated auth.bos and package.bos')
