//! A Wasm module can be compiled with multiple compilers.
//!
//! This example illustrates how to use the Singlepass compiler.
//!
//! You can run the example directly by executing in Wasmer root:
//!
//! ```shell
//! cargo run --example compiler-singlepass --release --features "singlepass"
//! ```
//!
//! Ready?

use std::fmt::Write;
use wasmer::{imports, wat2wasm, Instance, Module, Store, Value};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

pub fn many_functions_contract(function_count: u32) -> Vec<u8> {
    let mut functions = String::new();
    for i in 0..function_count {
        writeln!(
            &mut functions,
            "(func
              i32.const {}
              drop
              return)",
            i
        )
        .unwrap();
    }

    let code = format!(
        r#"(module
          (export "main" (func 0))
          {})"#,
        functions
    );
    wat2wasm(code.as_bytes()).unwrap().to_vec()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_span_tree::span_tree().aggregate(true).enable();

    // Let's declare the Wasm module with the text representation.
    let wasm_bytes = many_functions_contract(150_000);

    // Use Singlepass compiler with the default settings
    let compiler = Singlepass::default();

    // Create the store
    let store = Store::new(&Universal::new(compiler).engine());

    println!("Compiling module...");
    // Let's compile the Wasm module.
    let module = Module::new(&store, wasm_bytes)?;

    // Create an empty import object.
    let import_object = imports! {};

    println!("Instantiating module...");
    let instance = {
        // Let's instantiate the Wasm module.
        let _span = tracing::debug_span!(target: "vm", "Instance::new").entered();
        Instance::new(&module, &import_object)?
    };

    println!("Instantiating module... the second time");
    let instance = {
        // This one matches NEAR's execution model of initialization
        let _span = tracing::debug_span!(target: "vm", "Instance::new").entered();
        Instance::new(&module, &import_object)?
    };
    let main = instance.exports.get_function("main")?;

    println!("Calling `main` function...");
    let results = main.call(&[])?;

    println!("Results: {:?}", results);
    // assert_eq!(results.to_vec(), vec![Value::I32(3)]);

    Ok(())
}

#[test]
#[cfg(feature = "singlepass")]
fn test_compiler_singlepass() -> Result<(), Box<dyn std::error::Error>> {
    main()
}
