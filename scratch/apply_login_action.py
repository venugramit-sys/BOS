with open('packages/bos.auth/auth.bos', 'rb') as f:
    raw = f.read()

target = b'''    actions {\r
        action(SignIn) {\r
            caption = "Sign In";\r
            icon = "LogIn";\r
            primary = true;\r
            onAction(rec, ctx) {\r
                const vars = ctx.page.variables;\r
                if (!vars.username) throw new Error("Please enter your User ID.");\r
                if (!vars.password) throw new Error("Please enter your password.");\r
                const res = ctx.call("AuthManager", "Authenticate", { username: vars.username, password: vars.password });\r
                if (!res || !res.authenticated) {\r
                    throw new Error(res && res.reason === "ACCOUNT_LOCKED" ? "Account is locked due to too many failed attempts." : "Invalid User ID or Password.");\r
                }\r
                return res;\r
            }\r
        }\r
    }'''

replacement = b'''    actions {\r
        action(ForgotPassword) {\r
            caption = "Forgot Password?";\r
            icon = "KeyRound";\r
            onAction(rec, ctx) {\r
                const vars = ctx.page.variables;\r
                if (!vars.username) {\r
                    ctx.ui.notify("info", "Please enter your User ID first, then click 'Forgot Password?' to request reset instructions.");\r
                    return { requested: false };\r
                }\r
                const res = ctx.call("AuthManager", "RequestPasswordReset", { username: vars.username });\r
                const msg = res && res.message ? res.message : `Password reset instructions dispatched for '${vars.username}'.`;\r
                ctx.ui.notify("success", msg);\r
                return { requested: true, message: msg };\r
            }\r
        }\r
\r
        action(SignIn) {\r
            caption = "Sign In";\r
            icon = "LogIn";\r
            primary = true;\r
            onAction(rec, ctx) {\r
                const vars = ctx.page.variables;\r
                if (!vars.username) throw new Error("Please enter your User ID.");\r
                if (!vars.password) throw new Error("Please enter your password.");\r
                const res = ctx.call("AuthManager", "Authenticate", { username: vars.username, password: vars.password });\r
                if (!res || !res.authenticated) {\r
                    throw new Error(res && res.reason === "ACCOUNT_LOCKED" ? "Account is locked due to too many failed attempts." : "Invalid User ID or Password.");\r
                }\r
                return res;\r
            }\r
        }\r
    }'''

assert target in raw, 'target not found in raw'
raw = raw.replace(target, replacement)
with open('packages/bos.auth/auth.bos', 'wb') as f:
    f.write(raw)
print('Successfully patched action(ForgotPassword) into auth.bos')
