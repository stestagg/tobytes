use std::io::{self, Read, Write};
use serde::{Deserialize, Serialize};

// Include the generated code from build.rs
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[derive(Debug, Deserialize, Serialize)]
struct TestCase {
    name: String,
    description: String,
    rust_type: String,
    tests: Vec<serde_json::Value>,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: test_harness <encode|decode> <yaml_file>");
        std::process::exit(1);
    }

    let mode = &args[1];
    let yaml_file = &args[2];

    // Read YAML test case
    let yaml_content = std::fs::read_to_string(yaml_file)?;
    let test_case: TestCase = serde_yaml::from_str(&yaml_content)?;

    // Parse the name into the RustType enum
    let rust_type = RustType::from_name(&test_case.name)?;

    match mode.as_str() {
        "encode" => {
            // For each test value, encode it and write: [4-byte length][encoded bytes]
            for test_value in &test_case.tests {
                let encoded = encode_value(rust_type, test_value)?;
                // Write length as 4-byte big-endian integer
                let len = encoded.len() as u32;
                io::stdout().write_all(&len.to_be_bytes())?;
                // Write the encoded bytes
                io::stdout().write_all(&encoded)?;
            }
        }
        "decode" => {
            // Read encoded byte chunks from stdin: [4-byte length][encoded bytes]...
            let mut results = Vec::new();
            let mut input = Vec::new();
            io::stdin().read_to_end(&mut input)?;

            let mut cursor = io::Cursor::new(&input);
            while cursor.position() < input.len() as u64 {
                // Read 4-byte length
                let mut len_bytes = [0u8; 4];
                if cursor.read_exact(&mut len_bytes).is_err() {
                    break;
                }
                let len = u32::from_be_bytes(len_bytes) as usize;

                // Read encoded bytes
                let mut bytes = vec![0u8; len];
                cursor.read_exact(&mut bytes)?;

                // Decode
                let decoded = decode_value(rust_type, &bytes)?;
                results.push(decoded);
            }

            let output = serde_json::to_string(&results)?;
            io::stdout().write_all(output.as_bytes())?;
        }
        _ => {
            eprintln!("Invalid mode: {}. Use 'encode' or 'decode'", mode);
            std::process::exit(1);
        }
    }

    Ok(())
}
