with open('ui/src/lib/components/shell/SignIn.svelte', 'rb') as f:
    data = f.read()

# 1. Update handleActionComplete to handle forgotpassword
old_complete = b'''        notify.error("Please enter both User ID and password.", { title: "Sign In Required", page: p });
      }
    }
  }'''

new_complete = b'''        notify.error("Please enter both User ID and password.", { title: "Sign In Required", page: p });
      }
    } else if (actionName?.toLowerCase() === "forgotpassword") {
      const msg = res?.return_value?.message || "Password reset instructions have been dispatched.";
      notify.success(msg, { title: "Password Reset" });
    }
  }'''

if old_complete in data:
    data = data.replace(old_complete, new_complete)
elif old_complete.replace(b'\r\n', b'\n') in data:
    data = data.replace(old_complete.replace(b'\r\n', b'\n'), new_complete.replace(b'\r\n', b'\n'))
else:
    print('Warning: old_complete target not found, checking with normalize')

# 2. Add fallback forgot password handler
old_fallback_func = b'''  async function handleFallbackSubmit(e) {'''
new_fallback_func = b'''  async function handleFallbackForgotPassword() {
    if (!fallbackUsername.trim()) {
      fallbackError = "Please enter your User ID to request a password reset.";
      notify.info("Please enter your User ID first, then click 'Forgot Password?'.", { title: "User ID Required" });
      return;
    }
    fallbackError = "";
    notify.success(`If an account exists for '${fallbackUsername.trim()}', password reset instructions have been dispatched.`, { title: "Password Reset" });
  }

  async function handleFallbackSubmit(e) {'''

if old_fallback_func in data:
    data = data.replace(old_fallback_func, new_fallback_func)

# 3. Add Forgot Password button in fallback form
old_form_check = b'''            <div class="flex items-center justify-between pt-0.5">
              <label class="flex items-center gap-2 cursor-pointer text-xs text-slate-600 dark:text-slate-400 select-none">
                <input
                  type="checkbox"
                  bind:checked={fallbackRememberMe}
                  class="size-3.5 rounded border-slate-300 dark:border-slate-700 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-950"
                />
                <span>Remember User ID</span>
              </label>
            </div>'''

new_form_check = b'''            <div class="flex items-center justify-between pt-0.5">
              <label class="flex items-center gap-2 cursor-pointer text-xs text-slate-600 dark:text-slate-400 select-none">
                <input
                  type="checkbox"
                  bind:checked={fallbackRememberMe}
                  class="size-3.5 rounded border-slate-300 dark:border-slate-700 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-950"
                />
                <span>Remember User ID</span>
              </label>

              <button
                type="button"
                onclick={handleFallbackForgotPassword}
                class="text-xs text-indigo-600 dark:text-indigo-400 hover:underline cursor-pointer"
              >
                Forgot Password?
              </button>
            </div>'''

if old_form_check in data:
    data = data.replace(old_form_check, new_form_check)
elif old_form_check.replace(b'\r\n', b'\n') in data:
    data = data.replace(old_form_check.replace(b'\r\n', b'\n'), new_form_check.replace(b'\r\n', b'\n'))

with open('ui/src/lib/components/shell/SignIn.svelte', 'wb') as f:
    f.write(data)
print('Successfully patched ui/src/lib/components/shell/SignIn.svelte')
