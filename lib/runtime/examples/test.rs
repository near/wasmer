use wabt::wat2wasm;
use wasmer_clif_backend::CraneliftCompiler;
use wasmer_runtime::{import::Imports, Instance};

fn main() {
    let mut instance = create_module_1();
    let result = instance.call("get-0", &[]);
    println!("result: {:?}", result);
}

// fn generate_imports() -> Imports {
//     // let wasm_binary = wat2wasm(IMPORT_MODULE.as_bytes()).expect("WAST not valid or malformed");
//     // let module = wasmer_runtime::compile(&wasm_binary[..], &CraneliftCompiler::new()).expect("WASM can't be compiled");
//     // let instance = module.instantiate(Rc::new(Imports::new())).expect("WASM can't be instantiated");
//     let imports = Imports::new();
//     // imports.register("spectest", instance);
//     imports
// }

fn create_module_1() -> Instance {
    let module_str = r#"(module
      (type (;0;) (func (result i32)))
      (import "spectest" "global_i32" (global (;0;) i32))
      (func (;0;) (type 0) (result i32)
        get_global 0)
      (func (;1;) (type 0) (result i32)
        get_global 1)
      (global (;1;) i32 (get_global 0))
      (export "get-0" (func 0))
      (export "get-0-ref" (func 1)))
    "#;
    let wasm_binary = wat2wasm(module_str.as_bytes()).expect("WAST not valid or malformed");
    let module = wasmer_runtime::compile(&wasm_binary[..], &CraneliftCompiler::new())
        .expect("WASM can't be compiled");
    module
        .instantiate(generate_imports())
        .expect("WASM can't be instantiated")
}

static IMPORT_MODULE: &str = r#"
(module
  (type $t0 (func (param i32)))
  (type $t1 (func))
  (func $print_i32 (export "print_i32") (type $t0) (param $lhs i32))
  (func $print (export "print") (type $t1))
  (table $table (export "table") 10 20 anyfunc)
  (memory $memory (export "memory") 1 2)
  (global $global_i32 (export "global_i32") i32 (i32.const 666)))
"#;

pub fn generate_imports() -> Imports {
    let wasm_binary = wat2wasm(IMPORT_MODULE.as_bytes()).expect("WAST not valid or malformed");
    let module = wasmer_runtime::compile(&wasm_binary[..], &CraneliftCompiler::new())
        .expect("WASM can't be compiled");
    let instance = module
        .instantiate(Imports::new())
        .expect("WASM can't be instantiated");
    let mut imports = Imports::new();
    imports.register("spectest", instance);
    imports
}
