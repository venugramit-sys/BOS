with open('packages/bos.auth/auth.bos', 'rb') as f:
    data = f.read()

# 1. Add RequestPasswordReset to AuthManager before the closing brace of AuthManager
auth_manager_end = b'        return { success: true, configuration: updateData };\r\n    }\r\n}'
assert auth_manager_end in data or auth_manager_end.replace(b'\r\n', b'\n') in data

reset_proc = b'''        return { success: true, configuration: updateData };
    }

    RequestPasswordReset(username: String) {
        const u = (rec && rec.username !== undefined) ? rec.username : username;
        if (!u || String(u).trim().length === 0) {
            throw new Error("Username must not be empty.");
        }
        const targetUser = String(u).trim();
        const userRec = User.get(targetUser);

        SecurityAuditLog.insert({
            username: targetUser,
            action: "PasswordResetRequested",
            details: userRec ? `Password reset requested for '${targetUser}'. Reset instructions dispatched.` : `Password reset requested for non-existent account '${targetUser}'.`,
            isSuccess: !!userRec
        });

        return {
            success: true,
            username: targetUser,
            message: `If an account exists for '${targetUser}', password reset instructions have been dispatched to the registered address.`
        };
    }
}'''

if auth_manager_end in data:
    data = data.replace(auth_manager_end, reset_proc.replace(b'\n', b'\r\n'))
else:
    data = data.replace(auth_manager_end.replace(b'\r\n', b'\n'), reset_proc)

# 2. Add action(ForgotPassword) to LoginPage
login_page_action = b'''    actions {
        action(SignIn) {
            caption = "Sign In";
            icon = "LogIn";
            primary = true;
            onAction(rec, ctx) {
                const vars = ctx.page.variables;
                if (!vars.username) throw new Error("Please enter your User ID.");
                if (!vars.password) throw new Error("Please enter your password.");
                const res = ctx.call("AuthManager", "Authenticate", { username: vars.username, password: vars.password });
                if (!res || !res.authenticated) {
                    throw new Error(res && res.reason === "ACCOUNT_LOCKED" ? "Account is locked due to too many failed attempts." : "Invalid User ID or Password.");
                }
                return res;
            }
        }
    }'''

new_login_page_action = b'''    actions {
        action(ForgotPassword) {
            caption = "Forgot Password?";
            icon = "KeyRound";
            onAction(rec, ctx) {
                const vars = ctx.page.variables;
                if (!vars.username) {
                    ctx.ui.notify("info", "Please enter your User ID first, then click 'Forgot Password?' to request reset instructions.");
                    return { requested: false };
                }
                const res = ctx.call("AuthManager", "RequestPasswordReset", { username: vars.username });
                const msg = res && res.message ? res.message : `Password reset instructions dispatched for '${vars.username}'.`;
                ctx.ui.notify("success", msg);
                return { requested: true, message: msg };
            }
        }

        action(SignIn) {
            caption = "Sign In";
            icon = "LogIn";
            primary = true;
            onAction(rec, ctx) {
                const vars = ctx.page.variables;
                if (!vars.username) throw new Error("Please enter your User ID.");
                if (!vars.password) throw new Error("Please enter your password.");
                const res = ctx.call("AuthManager", "Authenticate", { username: vars.username, password: vars.password });
                if (!res || !res.authenticated) {
                    throw new Error(res && res.reason === "ACCOUNT_LOCKED" ? "Account is locked due to too many failed attempts." : "Invalid User ID or Password.");
                }
                return res;
            }
        }
    }'''

if login_page_action in data:
    data = data.replace(login_page_action, new_login_page_action.replace(b'\n', b'\r\n'))
else:
    data = data.replace(login_page_action.replace(b'\r\n', b'\n'), new_login_page_action)

with open('packages/bos.auth/auth.bos', 'wb') as f:
    f.write(data)
print('Successfully patched packages/bos.auth/auth.bos')
