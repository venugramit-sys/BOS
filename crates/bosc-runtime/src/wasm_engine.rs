use dashmap::DashMap;
use std::sync::Arc;
use wasmi::{Config, Engine, Linker, Module, Store, Val};

use crate::RuntimeError;

pub struct WasmEngine {
    engine: Engine,
    modules: Arc<DashMap<String, Module>>,
}

impl Default for WasmEngine {
    fn default() -> Self {
        let mut config = Config::default();
        config.consume_fuel(true);
        Self {
            engine: Engine::new(&config),
            modules: Arc::new(DashMap::new()),
        }
    }
}

impl WasmEngine {
    pub fn new() -> Result<Self, RuntimeError> {
        let mut config = Config::default();
        config.consume_fuel(true);

        let engine = Engine::new(&config);
        Ok(Self {
            engine,
            modules: Arc::new(DashMap::new()),
        })
    }

    /// Load and compile a WebAssembly binary (WASM) or WebAssembly Text format (WAT) module
    pub fn load_module(&self, name: &str, bytes_or_wat: &[u8]) -> Result<(), RuntimeError> {
        let module = Module::new(&self.engine, bytes_or_wat).map_err(|e| {
            RuntimeError::Internal(format!("Failed to compile Wasm module '{}': {:?}", name, e))
        })?;

        self.modules.insert(name.trim().to_lowercase(), module);
        Ok(())
    }

    /// Check if a Wasm module is loaded
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.contains_key(&name.trim().to_lowercase())
    }

    /// List all loaded Wasm modules
    pub fn list_modules(&self) -> Vec<String> {
        self.modules.iter().map(|kv| kv.key().clone()).collect()
    }

    /// Execute an exported function by name with typed arguments and fuel limitation
    pub fn execute(
        &self,
        module_name: &str,
        func_name: &str,
        params: &[Val],
        fuel_limit: Option<u64>,
    ) -> Result<Vec<Val>, RuntimeError> {
        let module = self
            .modules
            .get(&module_name.trim().to_lowercase())
            .ok_or_else(|| {
                RuntimeError::NotFound("WasmModule".to_string(), module_name.to_string())
            })?;

        let mut store = Store::new(&self.engine, ());
        let fuel = fuel_limit.unwrap_or(100_000);
        store.set_fuel(fuel).map_err(|e| {
            RuntimeError::Internal(format!("Failed to configure Wasm fuel: {:?}", e))
        })?;

        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| {
                RuntimeError::Internal(format!("Failed to instantiate Wasm module: {:?}", e))
            })?;

        let func = instance.get_func(&store, func_name).ok_or_else(|| {
            RuntimeError::NotFound(
                format!("Exported function '{}'", func_name),
                module_name.to_string(),
            )
        })?;

        let mut results = vec![Val::I32(0); func.ty(&store).results().len()];
        func.call(&mut store, params, &mut results).map_err(|e| {
            RuntimeError::Internal(format!(
                "Wasm execution error in '{}.{}': {:?}",
                module_name, func_name, e
            ))
        })?;

        Ok(results)
    }

    /// Execute a Wasm function accepting two integer arguments (e.g. tax, discount, arithmetic)
    pub fn execute_i32_op(
        &self,
        module_name: &str,
        func_name: &str,
        a: i32,
        b: i32,
    ) -> Result<i32, RuntimeError> {
        let results = self.execute(
            module_name,
            func_name,
            &[Val::I32(a), Val::I32(b)],
            Some(50_000),
        )?;
        if let Some(Val::I32(res)) = results.first() {
            Ok(*res)
        } else {
            Err(RuntimeError::Internal(format!(
                "Expected i32 return from '{}.{}'",
                module_name, func_name
            )))
        }
    }

    /// Execute a Wasm function accepting two 64-bit float arguments (e.g. compound interest, geometry, pricing matrix)
    pub fn execute_f64_op(
        &self,
        module_name: &str,
        func_name: &str,
        a: f64,
        b: f64,
    ) -> Result<f64, RuntimeError> {
        let results = self.execute(
            module_name,
            func_name,
            &[Val::F64(a.into()), Val::F64(b.into())],
            Some(50_000),
        )?;
        if let Some(Val::F64(res)) = results.first() {
            Ok(res.to_float())
        } else {
            Err(RuntimeError::Internal(format!(
                "Expected f64 return from '{}.{}'",
                module_name, func_name
            )))
        }
    }

    /// Execute a Wasm function passing an input string buffer and reading the returned string from linear memory
    pub fn execute_string_io(
        &self,
        module_name: &str,
        func_name: &str,
        input: &str,
        fuel_limit: Option<u64>,
    ) -> Result<String, RuntimeError> {
        let module = self
            .modules
            .get(&module_name.trim().to_lowercase())
            .ok_or_else(|| {
                RuntimeError::NotFound("WasmModule".to_string(), module_name.to_string())
            })?;

        let mut store = Store::new(&self.engine, ());
        let fuel = fuel_limit.unwrap_or(200_000);
        store.set_fuel(fuel).map_err(|e| {
            RuntimeError::Internal(format!("Failed to configure Wasm fuel: {:?}", e))
        })?;

        let linker = Linker::new(&self.engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .map_err(|e| {
                RuntimeError::Internal(format!("Failed to instantiate Wasm module: {:?}", e))
            })?;

        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| RuntimeError::Internal("Wasm module must export 'memory'".to_string()))?;

        // Write input to linear memory starting at offset 0
        let input_bytes = input.as_bytes();
        memory.write(&mut store, 0, input_bytes).map_err(|e| {
            RuntimeError::Internal(format!("Failed to write Wasm memory: {:?}", e))
        })?;

        let func = instance.get_func(&store, func_name).ok_or_else(|| {
            RuntimeError::NotFound(
                format!("Exported function '{}'", func_name),
                module_name.to_string(),
            )
        })?;

        let mut results = vec![Val::I32(0); func.ty(&store).results().len()];
        func.call(
            &mut store,
            &[Val::I32(0), Val::I32(input_bytes.len() as i32)],
            &mut results,
        )
        .map_err(|e| {
            RuntimeError::Internal(format!(
                "Wasm execution error in '{}.{}': {:?}",
                module_name, func_name, e
            ))
        })?;

        // Output length is returned by the export
        let out_len = if let Some(Val::I32(len)) = results.first() {
            (*len).max(0) as usize
        } else {
            0
        };

        if out_len == 0 {
            return Ok(String::new());
        }

        let mut out_buf = vec![0u8; out_len];
        memory.read(&store, 0, &mut out_buf).map_err(|e| {
            RuntimeError::Internal(format!("Failed to read Wasm memory: {:?}", e))
        })?;

        String::from_utf8(out_buf)
            .map_err(|e| RuntimeError::Internal(format!("Invalid UTF-8 from Wasm: {:?}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_wat_compilation_and_execution() {
        let engine = WasmEngine::new().expect("Failed to create WasmEngine");

        // WebAssembly Text representation of enterprise tax calculation module
        let wat_source = r#"
            (module
                (func (export "calculate_tax") (param f64 f64) (result f64)
                    local.get 0
                    local.get 1
                    f64.mul
                )
                (func (export "add_ints") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )
            )
        "#;

        engine
            .load_module("tax_engine", wat_source.as_bytes())
            .expect("Compile WAT");

        assert!(engine.has_module("tax_engine"));

        // Test integer op: 150 + 250 = 400
        let sum = engine
            .execute_i32_op("tax_engine", "add_ints", 150, 250)
            .expect("Add");
        assert_eq!(sum, 400);

        // Test float op: $1500.0 * 0.18 (18% tax) = 270.0
        let tax = engine
            .execute_f64_op("tax_engine", "calculate_tax", 1500.0, 0.18)
            .expect("Tax");
        assert!((tax - 270.0).abs() < 1e-6);
    }

    #[test]
    fn test_wasm_fuel_metering_prevents_infinite_loop() {
        let engine = WasmEngine::new().expect("Failed to create WasmEngine");

        // Module with an infinite loop
        let wat_source = r#"
            (module
                (func (export "infinite_loop") (result i32)
                    (loop $l
                        br $l
                    )
                    i32.const 0
                )
            )
        "#;

        engine
            .load_module("loop_module", wat_source.as_bytes())
            .expect("Compile loop");

        // Execution must fail cleanly with fuel exhaustion, avoiding freezing the thread!
        let res = engine.execute("loop_module", "infinite_loop", &[], Some(10_000));
        assert!(res.is_err(), "Expected fuel exhaustion failure");
    }

    #[test]
    fn test_wasm_string_io_roundtrip() {
        let engine = WasmEngine::new().expect("Failed to create WasmEngine");

        let wat_source = r#"
            (module
                (memory (export "memory") 1)
                (func (export "echo") (param i32 i32) (result i32)
                    local.get 1
                )
            )
        "#;

        engine
            .load_module("echo_module", wat_source.as_bytes())
            .expect("Compile echo");

        let echo_res = engine
            .execute_string_io("echo_module", "echo", "BOS_ENTERPRISE_PAYLOAD", Some(50_000))
            .expect("Echo string execution");
        assert_eq!(echo_res, "BOS_ENTERPRISE_PAYLOAD");
    }
}
