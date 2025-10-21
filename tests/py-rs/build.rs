use std::env;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
struct TestCase {
    name: String,
    rust_type: String,
    type_definition: Option<String>,
    rust_decode: Option<String>,
    rust_encode: Option<String>,
    rust_to_json: Option<String>,
    rust_from_json: Option<String>,
    requires: Option<Vec<String>>,
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated.rs");

    // Parse all test case YAML files
    let cases_dir = Path::new("cases");
    let mut test_cases = Vec::new();

    if cases_dir.exists() {
        for entry in fs::read_dir(cases_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                let content = fs::read_to_string(&path).unwrap();
                let test_case: TestCase = serde_yaml::from_str(&content).unwrap();
                test_cases.push(test_case);
            }
        }
    }

    // Sort by name for deterministic output
    test_cases.sort_by(|a, b| a.name.cmp(&b.name));

    // Generate the Rust code
    let mut code = String::new();

    code.push_str("use tobytes::prelude::*;\n");
    code.push_str("use ndarray;\n");
    code.push_str("use polars::prelude::*;\n\n");

    // Output all type definitions first
    for test_case in &test_cases {
        if let Some(ref type_def) = test_case.type_definition {
            code.push_str(type_def);
            code.push_str("\n\n");
        }
    }

    // Generate enum
    code.push_str("#[derive(Debug, Clone, Copy)]\n");
    code.push_str("pub enum RustType {\n");
    for test_case in &test_cases {
        code.push_str(&format!("    {},\n", test_case.name));
    }
    code.push_str("}\n\n");

    // Generate from_name implementation
    code.push_str("impl RustType {\n");
    code.push_str("    pub fn from_name(s: &str) -> std::result::Result<Self, String> {\n");
    code.push_str("        match s {\n");
    for test_case in &test_cases {
        code.push_str(&format!("            \"{}\" => Ok(RustType::{}),\n", test_case.name, test_case.name));
    }
    code.push_str("            _ => Err(format!(\"Unsupported test case name: {}\", s)),\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Generate encode_value function
    code.push_str("pub fn encode_value(\n");
    code.push_str("    rust_type: RustType,\n");
    code.push_str("    value: &serde_json::Value,\n");
    code.push_str(") -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {\n");
    code.push_str("    let mut buf = Vec::new();\n");
    code.push_str("    match rust_type {\n");

    for test_case in &test_cases {
        code.push_str(&format!("        RustType::{} => {{\n", test_case.name));

        // Use custom from_json conversion if specified, otherwise default to serde_json::from_value
        if let Some(ref from_json) = test_case.rust_from_json {
            code.push_str(&format!("            let v: {} = {};\n", test_case.rust_type, from_json));
        } else {
            code.push_str(&format!("            let v: {} = serde_json::from_value(value.clone())?;\n", test_case.rust_type));
        }

        // Use custom encode method if specified, otherwise default to to_bytes
        if let Some(ref encode_method) = test_case.rust_encode {
            code.push_str(&format!("            let encoded = v.{}()?;\n", encode_method));
            code.push_str("            encoded.to_bytes(&mut buf)?;\n");
        } else {
            code.push_str("            v.to_bytes(&mut buf)?;\n");
        }
        code.push_str("        }\n");
    }

    code.push_str("    }\n");
    code.push_str("    Ok(buf)\n");
    code.push_str("}\n\n");

    // Generate decode_value function
    code.push_str("pub fn decode_value(\n");
    code.push_str("    rust_type: RustType,\n");
    code.push_str("    bytes: &[u8],\n");
    code.push_str(") -> std::result::Result<serde_json::Value, Box<dyn std::error::Error>> {\n");
    code.push_str("    let mut cursor = io::Cursor::new(bytes);\n");
    code.push_str("    let result = match rust_type {\n");

    for test_case in &test_cases {
        code.push_str(&format!("        RustType::{} => {{\n", test_case.name));

        // Use custom decode method if specified, otherwise default to from_bytes
        if let Some(ref decode_method) = test_case.rust_decode {
            code.push_str(&format!("            let v: {} = {}(&mut cursor)?;\n", test_case.rust_type, decode_method));
        } else {
            // For generic types, we need turbofish syntax (::< instead of <)
            if test_case.rust_type.contains('<') {
                let turbofish_type = test_case.rust_type.replacen('<', "::<", 1);
                code.push_str(&format!("            let v: {} = {}::from_bytes(&mut cursor)?;\n", test_case.rust_type, turbofish_type));
            } else {
                code.push_str(&format!("            let v = {}::from_bytes(&mut cursor)?;\n", test_case.rust_type));
            }
        }

        // Use custom to_json conversion if specified, otherwise default to serde_json::to_value
        if let Some(ref to_json) = test_case.rust_to_json {
            code.push_str(&format!("            {}\n", to_json));
        } else {
            code.push_str("            serde_json::to_value(v)?\n");
        }
        code.push_str("        }\n");
    }

    code.push_str("    };\n");
    code.push_str("    Ok(result)\n");
    code.push_str("}\n");

    fs::write(&dest_path, code).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=cases");
}
