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
    /* This contracts is
    (module
    (type (;0;) (func))
    (func (;0;) (type 0)
      (local f64 <32750 times>)
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

#[test]
fn stack_limit_ok() {
    let wat = r#"
        (memory (;0;) 1000 10000)
        (func $foo
            (local f64)
            i32.const 0
            i32.const 1
            i32.add
            drop
        )
        (func (export "main")
            (local $v0 i32)
            i32.const 1000000
            local.set $v0
            loop $L0
                local.get $v0
                i32.const 1
                i32.sub
                local.set $v0
                call $foo
                local.get $v0
                i32.const 0
                i32.gt_s
                br_if $L0
            end
        )
    "#;

    let store = get_store();
    let module = Module::new(&store, &wat).unwrap();
    let instance = Instance::new_with_config(
        &module,
        unsafe { InstanceConfig::new_with_stack_limit(1000) },
        &imports! {},
    );
    assert!(instance.is_ok());
    let instance = instance.unwrap();
    let main_func = instance
        .exports
        .get_function("main")
        .expect("expected function main");
    let e = main_func.call(&[]);
    assert!(e.is_ok());
}

#[test]
fn stack_limit_no_args() {
    let wat = r#"
        (func $foo
            call $foo
        )
        (func (export "main")
            call $foo
        )
    "#;

    let store = get_store();
    let module = Module::new(&store, &wat).unwrap();
    let instance =
        Instance::new_with_config(&module, unsafe { InstanceConfig::default() }, &imports! {});
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

#[test]
fn deep_but_sane() {
    let wat = r#"
        (func $foo (param $p0 i32) (result i32)
            local.get $p0
            i32.const 1
            i32.sub
            local.set $p0
            block $B0
                local.get $p0
                i32.const 0
                i32.le_s
                br_if $B0
                local.get $p0
                call $foo
                drop
            end
            local.get $p0
        )
        (func (export "main")
            i32.const 1000
            call $foo
            drop
        )
    "#;

    let store = get_store();
    let module = Module::new(&store, &wat).unwrap();
    let instance =
        Instance::new_with_config(&module, unsafe { InstanceConfig::default() }, &imports! {});
    assert!(instance.is_ok());
    let instance = instance.unwrap();
    let main_func = instance
        .exports
        .get_function("main")
        .expect("expected function main");

    let e = main_func.call(&[]);
    assert!(e.is_ok());
}
