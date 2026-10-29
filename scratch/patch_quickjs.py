with open('crates/bosc-runtime/src/quickjs_engine.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target1 = """thread_local! {
    static RUNTIME_POOL: RefCell<Vec<Runtime>> = RefCell::new(Vec::new());
}"""

target2 = """fn checkout_runtime() -> Result<Runtime, RuntimeError> {
    if let Some(runtime) = RUNTIME_POOL.with(|pool| pool.borrow_mut().pop()) {
        return Ok(runtime);
    }
    Runtime::new().map_err(|e| {
        RuntimeError::Internal(format!("Failed to initialize QuickJS runtime: {:?}", e))
    })
}

impl Drop for QuickJsEngine {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.set_interrupt_handler(None);
            RUNTIME_POOL.with(|pool| {
                let mut pool = pool.borrow_mut();
                if pool.len() < 4 {
                    pool.push(runtime);
                }
            });
        }
    }
}"""

replacement2 = """fn checkout_runtime() -> Result<Runtime, RuntimeError> {
    Runtime::new().map_err(|e| {
        RuntimeError::Internal(format!("Failed to initialize QuickJS runtime: {:?}", e))
    })
}

impl Drop for QuickJsEngine {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.set_interrupt_handler(None);
        }
    }
}"""

assert target1 in content, "target1 not found"
assert target2 in content, "target2 not found"

content = content.replace(target1, "// Runtime pool removed to avoid TLS drop outside Tokio context")
content = content.replace(target2, replacement2)

with open('crates/bosc-runtime/src/quickjs_engine.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Successfully replaced!")
