use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    // Only compile Cap'n Proto schemas if they exist
    let schema_dir = Path::new("schemas");
    if schema_dir.exists() {
        println!("cargo:rerun-if-changed=schemas");

        // Would compile .capnp files here
        // capnpc::CompilerCommand::new()
        //     .src_prefix("schemas")
        //     .file("schemas/mcp.capnp")
        //     .run()
        //     .expect("schema compilation failed");
    }

    // Generate perfect hash table for token estimation
    println!("cargo:rerun-if-changed=build.rs");
    generate_token_table();
}

fn generate_token_table() {
    let mut table = [0u16; 131072]; // 2^17 entries

    // Common code patterns and their token counts
    // In a real implementation, this would be generated from analyzing a large code corpus
    let patterns = vec![
        // Common programming tokens
        ("fn", 1),
        ("let", 1),
        ("mut", 1),
        ("const", 1),
        ("struct", 1),
        ("impl", 1),
        ("pub", 1),
        ("use", 1),
        ("mod", 1),
        ("match", 1),
        ("if", 1),
        ("else", 1),
        ("for", 1),
        ("while", 1),
        ("loop", 1),
        ("return", 1),
        ("break", 1),
        ("continue", 1),
        ("async", 1),
        ("await", 1),
        ("self", 1),
        ("Self", 1),
        ("super", 1),
        ("crate", 1),
        // Common variable names
        ("i", 1),
        ("j", 1),
        ("k", 1),
        ("x", 1),
        ("y", 1),
        ("z", 1),
        ("result", 1),
        ("error", 1),
        ("value", 1),
        ("data", 1),
        ("input", 1),
        ("output", 1),
        ("index", 1),
        ("count", 1),
        ("size", 1),
        ("length", 1),
        ("buffer", 1),
        ("stream", 1),
        ("file", 1),
        ("path", 1),
        ("name", 1),
        ("type", 1),
        // Common types
        ("String", 1),
        ("Vec", 1),
        ("HashMap", 2),
        ("Result", 1),
        ("Option", 1),
        ("Box", 1),
        ("Arc", 1),
        ("Mutex", 1),
        // Common function calls
        ("println!", 2),
        ("format!", 2),
        ("vec!", 1),
        ("unwrap", 1),
        ("expect", 1),
        ("clone", 1),
        ("to_string", 2),
        ("as_str", 2),
        ("len", 1),
        ("is_empty", 2),
        ("push", 1),
        ("pop", 1),
        // Common operators
        ("->", 1),
        ("=>", 1),
        ("::", 1),
        ("..", 1),
        ("..=", 1),
        ("&&", 1),
        ("||", 1),
        ("==", 1),
        ("!=", 1),
        ("<=", 1),
        (">=", 1),
        // Common patterns
        ("Ok(", 1),
        ("Err(", 1),
        ("Some(", 1),
        ("None", 1),
        ("true", 1),
        ("false", 1),
        ("null", 1),
        // Common English words
        ("the", 1),
        ("of", 1),
        ("to", 1),
        ("and", 1),
        ("a", 1),
        ("in", 1),
        ("is", 1),
        ("it", 1),
        ("for", 1),
        ("with", 1),
        ("as", 1),
        ("on", 1),
        ("be", 1),
        ("at", 1),
        ("by", 1),
        ("from", 1),
        ("that", 1),
        ("this", 1),
        ("have", 1),
        ("not", 1),
    ];

    // Populate the table using xxhash
    for (pattern, token_count) in patterns {
        // Simple hash function for build script (xxhash not available here)
        let hash = pattern
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
            as usize;
        let index = hash & 0x1FFFF; // Mask to 17 bits

        // Handle collisions by keeping lower token count
        if table[index] == 0 || token_count < table[index] {
            table[index] = token_count;
        }
    }

    // Write the table to a file
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("token_table.rs");
    let mut f = File::create(&dest_path).unwrap();

    write!(f, "[").unwrap();
    for (i, &count) in table.iter().enumerate() {
        if i > 0 {
            write!(f, ",").unwrap();
        }
        if i % 16 == 0 {
            write!(f, "\n    ").unwrap();
        }
        write!(f, "{}", count).unwrap();
    }
    write!(f, "\n]").unwrap();
}
