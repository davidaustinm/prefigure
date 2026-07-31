//! Rewrite a wasm module so it imports nothing outside module `typst_env`.
//!
//! Usage: stub-imports <in.wasm> <out.wasm>
//!
//! Every imported function from any other module (in practice the `__wbindgen_*`
//! functions dragged in transitively by the RaTeX font loader) is turned into a
//! local function whose body is a single `unreachable`. Typst then accepts the
//! module, and since embedded-font math never calls those paths, the traps are
//! never hit. Non-function imports outside `typst_env`, if any, are reported and
//! left in place (the ABI only forbids them at load if referenced).

use std::process::exit;

use walrus::{ImportKind, Module};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: stub-imports <in.wasm> <out.wasm>");
        exit(2);
    }
    let (input, output) = (&args[1], &args[2]);

    let mut module = match Module::from_file(input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("stub-imports: cannot read {input}: {e}");
            exit(1);
        }
    };

    // Collect the offending function imports first (can't mutate while borrowing).
    let mut to_stub = Vec::new();
    let mut non_func = Vec::new();
    for import in module.imports.iter() {
        if import.module == "typst_env" {
            continue;
        }
        match import.kind {
            ImportKind::Function(id) => to_stub.push((id, import.name.clone())),
            _ => non_func.push(format!("{}::{}", import.module, import.name)),
        }
    }

    for (id, name) in &to_stub {
        // Replace the import with a local function body of just `unreachable`.
        if let Err(e) = module.replace_imported_func(*id, |(body, _args)| {
            body.unreachable();
        }) {
            eprintln!("stub-imports: failed to stub {name}: {e}");
            exit(1);
        }
    }

    if !non_func.is_empty() {
        eprintln!(
            "stub-imports: warning: {} non-function import(s) left in place: {}",
            non_func.len(),
            non_func.join(", ")
        );
    }

    if let Err(e) = module.emit_wasm_file(output) {
        eprintln!("stub-imports: cannot write {output}: {e}");
        exit(1);
    }
    eprintln!("stub-imports: stubbed {} import(s) -> {output}", to_stub.len());
}
