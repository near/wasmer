use anyhow::Result;
use wasmer::{wat2wasm, BaseTunables, Engine};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

fn compile_and_compare(wasm: &[u8]) -> Result<()> {
    let compiler = Singlepass::default();
    let engine = Universal::new(compiler).engine();
    let tunables = BaseTunables::for_target(engine.target());

    // compile for first time
    let executable = engine.compile(wasm, &tunables).unwrap();
    let mut writer1 = std::io::Cursor::new(vec![]);
    executable.serialize(&mut writer1).unwrap();

    // compile for second time
    let executable = engine.compile(wasm, &tunables).unwrap();
    let mut writer2 = std::io::Cursor::new(vec![]);
    executable.serialize(&mut writer2).unwrap();

    assert!(writer1.into_inner() == writer2.into_inner());

    Ok(())
}

#[test]
fn deterministic_empty() -> Result<()> {
    let wasm_bytes = wat2wasm(
        br#"
    (module)
    "#,
    )?;

    compile_and_compare(&wasm_bytes)
}

#[test]
fn deterministic_table() -> Result<()> {
    let wasm_bytes = wat2wasm(
        br#"
(module
  (table 2 funcref)
  (func $f1)
  (func $f2)
  (elem (i32.const 0) $f1 $f2))
"#,
    )?;

    compile_and_compare(&wasm_bytes)
}
