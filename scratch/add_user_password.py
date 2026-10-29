for path in ['packages/bos.auth/auth.bos', 'packages/bos.auth/package.bos']:
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()

    target = '                field(fullName);\n                field(phoneNumber);'
    repl = '                field(fullName);\n                field(passwordHash) {\n                    caption = "Password";\n                    control = password;\n                }\n                field(phoneNumber);'

    content = content.replace(target.replace('\n', '\r\n'), repl.replace('\n', '\r\n'))
    content = content.replace(target, repl)

    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)

print('Added passwordHash to UserCard')
