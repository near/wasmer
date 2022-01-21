use std::sync::Arc;
use wasmer::*;
use wasmer_engine::Engine;
use wasmer_engine_universal::Universal;
use wasmer_types::Type::I64;
use wasmer_types::{InstanceConfig, NamedFunction};

fn slow_to_compile_contract(n_fns: usize, n_locals: usize) -> Vec<u8> {
    let fns = format!("(func (local {}))\n", "i32 ".repeat(n_locals)).repeat(n_fns);
    let wat = format!(r#"(module {} (func (export "main")))"#, fns);
    wat2wasm(wat.as_bytes()).unwrap().to_vec()
}

fn compile_uncached<'a>(
    store: &'a Store,
    engine: &'a dyn Engine,
    code: &'a [u8],
    time: bool,
) -> Result<Arc<dyn wasmer_engine::Artifact>, CompileError> {
    use std::time::Instant;
    let now = Instant::now();
    engine.validate(code)?;
    let validate = now.elapsed().as_millis();
    let now = Instant::now();
    let res = engine.compile(code, store.tunables());
    let compile = now.elapsed().as_millis();
    if time {
        println!("validate {}ms compile {}ms", validate, compile);
    }
    res
}

#[test]
#[ignore]
fn compilation_test() {
    let compiler = Singlepass::default();
    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    for factor in 1..1000 {
        let code = slow_to_compile_contract(3, 25 * factor);
        match compile_uncached(&store, &engine, &code, false) {
            Ok(art) => {
                let serialized = art.serialize().unwrap();
                println!(
                    "{}: artefact is compiled, size is {}",
                    factor,
                    serialized.len()
                );
            }
            Err(err) => {
                println!("err is {:?}", err);
            }
        }
    }
}

/*
Code to create perf map.

fn write_perf_profiler_map(functions: &Vec<NamedFunction>) -> Result<(), Box<dyn std::error::Error>>{
    let pid = process::id();
    let filename = format!("/tmp/perf-{}.map", pid);
    let mut file = File::create(filename).expect("Unable to create file");
    for f in functions {
        file.write_fmt(format_args!("{:x} {:x} {}\n", f.address, f.size, f.name))?;
    }
    Ok(())
}
*/

#[test]
fn profiling() {
    let wat = r#"
       (func (export "f1"))
       (func (export "f2"))
       (func (export "f3"))
    "#;
    let wasm = wat2wasm(wat.as_bytes()).unwrap();
    let compiler = Singlepass::default();
    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    match compile_uncached(&store, &engine, &wasm, false) {
        Ok(art) => unsafe {
            let serialized = art.serialize().unwrap();
            let module = wasmer::Module::deserialize(&store, serialized.as_slice()).unwrap();
            let instance =
                Instance::new_with_config(&module, InstanceConfig::default(), &imports! {});
            assert!(instance.is_ok());
            let instance = instance.unwrap();
            let named = instance.named_functions();
            assert_eq!(3, named.len());
            assert_eq!("f1", named[0].name);
            assert_eq!("f2", named[1].name);
            assert_eq!("f3", named[2].name);
        },
        Err(_) => {
            assert!(false)
        }
    }
}
