use wasmer::*;
use wasmer_types::InstanceConfig;
use wasmer_vm::TrapCode;

fn get_store() -> Store {
    let compiler = Singlepass::default();
    let store = Store::new(&Universal::new(compiler).engine());
    store
}

#[test]
fn stack_limit_hit() {
    /**
     * This contracts is
    (module
    (type (;0;) (func))
    (func (;0;) (type 0)
      (local f64 <many times>)
       local.get 1
       local.get 0
       f64.copysign
       call 0
       unreachable)
    (memory (;0;) 16 144)
    (export "main" (func 0)))
     */
    let wasm: [u8; 53] = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x05, 0x05, 0x01, 0x01, 0x10, 0x90, 0x01, 0x07, 0x08, 0x01, 0x04, 0x6d,
        0x61, 0x69, 0x6e, 0x00, 0x00, 0x0a, 0x10, 0x01, 0x0e, 0x01, 0xee, 0xff, 0x01, 0x7c, 0x20,
        0x01, 0x20, 0x00, 0xa6, 0x10, 0x00, 0x00, 0x0b,
    ];
    let store = get_store();
    let module = Module::new(&store, &wasm).unwrap();
    let instance = Instance::new_with_config(
        &module,
        unsafe { InstanceConfig::new_with_stack_limit(100000) },
        &imports! {},
    );
    assert!(instance.is_ok());
    let instance = instance.unwrap();
    let main_func = instance
        .exports
        .get_function("main")
        .expect("expected function main");
    match main_func.call(&[]) {
        Err(err) => {
            let trap = err.to_trap().unwrap();
            assert_eq!(trap, TrapCode::StackOverflow);
        }
        _ => assert!(false),
    }
}
