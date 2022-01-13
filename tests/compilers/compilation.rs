use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use wasmer::*;
use wasmer_engine::Engine;
use wasmer_engine_universal::Universal;

fn slow_to_compile_contract(n_fns: usize, n_locals: usize) -> Vec<u8> {
    let fns = format!("(func (local {}))\n", "i32 ".repeat(n_locals)).repeat(n_fns);
    let wat = format!(r#"(module {} (func (export "main")))"#, fns);
    wat2wasm(wat.as_bytes()).unwrap().to_vec()
}

fn compile_uncached<'a>(
    store: &'a Store,
    engine: &'a dyn Engine,
    code: &'a [u8],
) -> Result<Arc<dyn wasmer_engine::Artifact>, CompileError> {
    use std::time::Instant;
    let now = Instant::now();
    engine.validate(code)?;
    let validate = now.elapsed().as_millis();
    let now = Instant::now();
    let res = engine.compile(code, store.tunables());
    let compile = now.elapsed().as_millis();
    println!("validate {} compile {}", validate, compile);
    res
}

// #[test]
fn compilation_test() {
    let compiler = Singlepass::default();
    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    for factor in 1..1000 {
        let code = slow_to_compile_contract(3, 25 * factor);
        match compile_uncached(&store, &engine, &code) {
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

// #[test]
fn disassemble() {
    let contract_wasm = "/Users/Nikolay.Igotti/near/aurora-engine/mainnet-release.wasm";
    let out_code = "/Users/Nikolay.Igotti/near/aurora-engine/mainnet-release.bin";
    let out_artefact = "/Users/Nikolay.Igotti/near/aurora-engine/mainnet-release.art";

    let compiler = Singlepass::default();
    let engine = Universal::new(compiler).engine();
    let store = Store::new(&engine);
    let code = fs::read(contract_wasm).unwrap();
    match compile_uncached(&store, &engine, &code) {
        Ok(art) => unsafe {
            let serialized = art.serialize().unwrap();
            let mut bin_file = File::create(out_code).unwrap();
            for f in art.finished_functions() {
                let index = f.0;
                let ptr = f.1;
                let len = art.finished_functions_lengths()[index];
                bin_file
                    .write_all(std::slice::from_raw_parts(ptr.0 as *const u8, len))
                    .unwrap();
            }
            println!("artefact is compiled, size is {}", serialized.len());
            fs::write(out_artefact, serialized).unwrap();
        },
        Err(err) => {
            println!("err is {:?}", err);
        }
    }
}
