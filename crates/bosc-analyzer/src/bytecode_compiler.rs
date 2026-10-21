use bosc_ir::{BytecodeArtifactIR, CodeunitIR, ModelIR, StateMachineIR, TableLogicIR};
use rquickjs::{Context, Module, Runtime};
use sha2::{Digest, Sha256};

/// Ahead-of-Time QuickJS Bytecode Compiler
pub struct BytecodeCompiler;

impl BytecodeCompiler {
    /// Compiles an arbitrary JavaScript/ESM string into raw QuickJS bytecode bytes
    pub fn compile_js_to_bytecode(name: &str, js_source: &str) -> Result<Vec<u8>, String> {
        let rt = Runtime::new().map_err(|e| format!("Failed to initialize QuickJS runtime: {}", e))?;
        let ctx = Context::full(&rt).map_err(|e| format!("Failed to initialize QuickJS context: {}", e))?;

        ctx.with(|ctx| {
            let module = Module::declare(ctx.clone(), name, js_source)
                .map_err(|e| format!("Module declaration failed for '{}': {}", name, e))?;

            let bytecode = module
                .write(Default::default())
                .map_err(|e| format!("Bytecode serialization failed for '{}': {}", name, e))?;

            Ok(bytecode)
        })
    }

    /// Helper to compute SHA-256 hash of source code
    pub fn compute_source_hash(source: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        format!("{:064x}", hasher.finalize())
    }

    /// Compiles a CodeunitIR into a verified BytecodeArtifactIR (.qbc)
    pub fn compile_codeunit(cu: &CodeunitIR) -> Result<BytecodeArtifactIR, String> {
        let mut js = String::new();
        js.push_str(&format!("// BOS AOT Compiled Codeunit: {}\n", cu.name));
        js.push_str(&format!("export class {} {{\n", cu.name));

        for m in &cu.methods {
            let param_list = m
                .params
                .iter()
                .map(|(p, _)| p.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let body = m.script.as_deref().unwrap_or("return null;");
            js.push_str(&format!("  static {}({}) {{\n", m.name, param_list));
            if !m.params.is_empty() {
                let first_param = &m.params[0].0;
                let unpack_checks = m
                    .params
                    .iter()
                    .map(|(p, _)| format!("'{p}' in {first_param}"))
                    .collect::<Vec<_>>()
                    .join(" || ");
                js.push_str(&format!(
                    "    if (typeof {first_param} === 'object' && {first_param} !== null && ({unpack_checks})) {{\n"
                ));
                for (p, _) in &m.params {
                    if p != first_param {
                        js.push_str(&format!("      var {p} = {first_param}.{p};\n"));
                    }
                }
                js.push_str(&format!("      {first_param} = {first_param}.{first_param};\n"));
                js.push_str("    }\n");
            }
            js.push_str(&format!("    {}\n", body));
            js.push_str("  }\n");
        }

        js.push_str("}\n");
        js.push_str(&format!("export default {};\n", cu.name));

        let source_hash = Self::compute_source_hash(&js);
        let bytecode = Self::compile_js_to_bytecode(&cu.name, &js)?;

        Ok(BytecodeArtifactIR::new(
            cu.name.clone(),
            "codeunit",
            bytecode,
            source_hash,
        ))
    }

    /// Compiles a TableLogicIR into a verified BytecodeArtifactIR (.qbc)
    pub fn compile_tablelogic(tl: &TableLogicIR) -> Result<BytecodeArtifactIR, String> {
        let mut js = String::new();
        js.push_str(&format!("// BOS AOT Compiled TableLogic: {} for {}\n", tl.name, tl.target_table));
        js.push_str(&format!("export class {} {{\n", tl.name));

        // Triggers: onInsert, onModify, onDelete
        for trig in &tl.triggers {
            let param_list = if trig.params.is_empty() {
                "rec, ctx".to_string()
            } else {
                trig.params.join(", ")
            };
            let body = trig.script.as_deref().unwrap_or("");
            js.push_str(&format!("  static {}({}) {{\n", trig.name, param_list));
            js.push_str(&format!("    {}\n", body));
            js.push_str("  }\n");
        }

        // Field Validations
        for val in &tl.field_validations {
            let old_param = val.old_param_name.as_deref().unwrap_or("xRec");
            js.push_str(&format!(
                "  static validate_{}(rec, {}, {}, ctx) {{\n",
                val.field_name, val.param_name, old_param
            ));
            js.push_str(&format!("    {}\n", val.script));
            js.push_str("  }\n");
        }

        // Methods
        for m in &tl.methods {
            let param_list = m
                .params
                .iter()
                .map(|(p, _)| p.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let body = m.script.as_deref().unwrap_or("return null;");
            js.push_str(&format!("  static {}({}) {{\n", m.name, param_list));
            js.push_str(&format!("    {}\n", body));
            js.push_str("  }\n");
        }

        js.push_str("}\n");
        js.push_str(&format!("export default {};\n", tl.name));

        let source_hash = Self::compute_source_hash(&js);
        let bytecode = Self::compile_js_to_bytecode(&tl.name, &js)?;

        Ok(BytecodeArtifactIR::new(
            tl.name.clone(),
            "tablelogic",
            bytecode,
            source_hash,
        ))
    }

    /// Compiles a StateMachineIR into a verified BytecodeArtifactIR (.qbc)
    pub fn compile_statemachine(sm: &StateMachineIR) -> Result<BytecodeArtifactIR, String> {
        let mut js = String::new();
        js.push_str(&format!("// BOS AOT Compiled StateMachine: {} for {}\n", sm.name, sm.target_table));
        js.push_str(&format!("export class {} {{\n", sm.name));
        js.push_str(&format!("  static initial = {:?};\n", sm.initial_state));

        js.push_str("  static transitions = [\n");
        for t in &sm.transitions {
            let from_json = serde_json::to_string(&t.from_states).unwrap_or_else(|_| "[]".to_string());
            js.push_str(&format!(
                "    {{ name: {:?}, from: {}, to: {:?} }},\n",
                t.name, from_json, t.to_state
            ));
        }
        js.push_str("  ];\n");

        js.push_str("}\n");
        js.push_str(&format!("export default {};\n", sm.name));

        let source_hash = Self::compute_source_hash(&js);
        let bytecode = Self::compile_js_to_bytecode(&sm.name, &js)?;

        Ok(BytecodeArtifactIR::new(
            sm.name.clone(),
            "statemachine",
            bytecode,
            source_hash,
        ))
    }

    /// Compiles all logic meta-objects in a ModelIR into an aggregated pre-linked package bundle (.qbc)
    pub fn compile_package_bundle(package_name: &str, model: &ModelIR) -> Result<BytecodeArtifactIR, String> {
        let mut js = String::new();
        js.push_str(&format!("// BOS Monolithic Pre-Linked Bytecode Bundle: {}\n", package_name));
        js.push_str("export const bundle = {\n");

        // Codeunits
        js.push_str("  codeunits: {\n");
        for cu in &model.codeunits {
            js.push_str(&format!("    {}: class {{\n", cu.name));
            for m in &cu.methods {
                let param_list = m
                    .params
                    .iter()
                    .map(|(p, _)| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let body = m.script.as_deref().unwrap_or("return null;");
                js.push_str(&format!("      static {}({}) {{\n", m.name, param_list));
                if !m.params.is_empty() {
                    let first_param = &m.params[0].0;
                    let unpack_checks = m
                        .params
                        .iter()
                        .map(|(p, _)| format!("'{p}' in {first_param}"))
                        .collect::<Vec<_>>()
                        .join(" || ");
                    js.push_str(&format!(
                        "        if (typeof {first_param} === 'object' && {first_param} !== null && ({unpack_checks})) {{\n"
                    ));
                    for (p, _) in &m.params {
                        if p != first_param {
                            js.push_str(&format!("          var {p} = {first_param}.{p};\n"));
                        }
                    }
                    js.push_str(&format!("          {first_param} = {first_param}.{first_param};\n"));
                    js.push_str("        }\n");
                }
                js.push_str(&format!("        {}\n", body));
                js.push_str("      }\n");
            }
            js.push_str("    },\n");
        }
        js.push_str("  },\n");

        // TableLogics
        js.push_str("  tablelogics: {\n");
        for tl in &model.table_logics {
            js.push_str(&format!("    {}: class {{\n", tl.name));
            for trig in &tl.triggers {
                let param_list = if trig.params.is_empty() {
                    "rec, ctx".to_string()
                } else {
                    trig.params.join(", ")
                };
                let body = trig.script.as_deref().unwrap_or("");
                js.push_str(&format!("      static {}({}) {{\n", trig.name, param_list));
                js.push_str(&format!("        {}\n", body));
                js.push_str("      }\n");
            }
            for val in &tl.field_validations {
                let old_param = val.old_param_name.as_deref().unwrap_or("xRec");
                js.push_str(&format!(
                    "      static validate_{}(rec, {}, {}, ctx) {{\n",
                    val.field_name, val.param_name, old_param
                ));
                js.push_str(&format!("        {}\n", val.script));
                js.push_str("      }\n");
            }
            js.push_str("    },\n");
        }
        js.push_str("  },\n");

        js.push_str("};\n");
        js.push_str("export default bundle;\n");

        let source_hash = Self::compute_source_hash(&js);
        let bundle_name = format!("{}.bundle", package_name);
        let bytecode = Self::compile_js_to_bytecode(&bundle_name, &js)?;

        Ok(BytecodeArtifactIR::new(
            bundle_name,
            "bundle",
            bytecode,
            source_hash,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_js_to_bytecode_and_envelope() {
        let js = "export function add(a, b) { return a + b; }\nexport default add;";
        let bytecode = BytecodeCompiler::compile_js_to_bytecode("math", js)
            .expect("QuickJS bytecode compilation must succeed");
        assert!(!bytecode.is_empty(), "Bytecode must not be empty");

        let artifact = BytecodeArtifactIR::new("math", "codeunit", bytecode.clone(), "fakehash");
        assert!(artifact.verify_integrity(), "CRC32 integrity must hold");

        let envelope = artifact.to_binary_envelope();
        assert_eq!(&envelope[0..4], &bosc_ir::BCBOS_MAGIC);

        let parsed = BytecodeArtifactIR::from_binary_envelope("math", "codeunit", "fakehash", &envelope)
            .expect("Envelope deserialization must succeed");
        assert_eq!(parsed.bytecode, bytecode);
        assert!(parsed.verify_integrity());
    }
}
