use std::ptr;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use wasmer::*;
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;
use wasmer_types::InstanceConfig;

fn instantiate(
    hits: Arc<AtomicUsize>,
    wasm: &[u8],
    config: InstanceConfig,
) -> (Store, Module, Result<Instance, InstantiationError>) {
    let compiler = Singlepass::default();
    let store = Store::new(&Universal::new(compiler).engine());
    let module = Module::new(&store, wasm).unwrap();
    let hit = Function::new(&store, FunctionType::new(vec![], vec![]), move |_values| {
        hits.fetch_add(1, SeqCst);
        Ok(vec![])
    });
    let gas = Function::new(
        &store,
        FunctionType::new(vec![ValType::I32], vec![]),
        |_| {
            assert!(false, "gas must always be intrisicified");
            Ok(vec![])
        },
    );
    let gas64 = Function::new(
        &store,
        FunctionType::new(vec![ValType::I64], vec![]),
        |_| {
            assert!(false, "gas must always be intrisicified");
            Ok(vec![])
        },
    );
    let instance = Instance::new_with_config(
        &module,
        config,
        &imports! { "host" => { "hit" => hit, "gas" => gas, "gas64" => gas64 } },
    );
    (store, module, instance)
}

#[test]
fn test_gas_limiting_in_start() {
    let test_cases = [
        (
            "(call $gas (i32.const 126))",
            "(call $gas (i32.const 300))",
            300,
            2,
        ),
        (
            "(call $gas (i32.const 149))",
            "(call $gas (i32.const 150))",
            300,
            3,
        ),
        (
            "(call $gas (i32.const 0))",
            "(call $gas (i32.const 150))",
            300,
            6,
        ),
        (
            // Verify things work as expeceted when the gas count exceeds i32 maximum.
            "(call $gas (i32.const 2147483647))", // 2^31-1
            "(call $gas (i32.const 2147483648))", // 2^31
            1 << 32,
            3,
        ),
        (
            // Verify things work as expeceted when the gas count exceeds i32 maximum.
            "(call $gas64 (i64.const 2147483647))", // 2^31-1
            "(call $gas64 (i64.const 2147483648))", // 2^31
            1 << 32,
            3,
        ),
        (
            // Verify things work as expeceted when the gas count exceeds u32 maximum.
            "(call $gas64 (i64.const 8589934592))", // 2^33
            "(call $gas64 (i64.const 8589934592))", // 2^33
            1 << 34,
            3,
        ),
        (
            "(call $gas64 (i64.const 9223372036854775807))", // 2^63-1
            "(call $gas64 (i64.const 0))",
            i64::MAX,
            3,
        ),
        (
            // Verify things work as expeceted when the gas count exceeds i64 maximum.
            "(call $gas64 (i64.const 9223372036854775808))", // 2^63
            "(call $gas64 (i64.const 0))",
            i64::MAX,
            1,
        ),
        (
            // Verify things work as expeceted when the gas count = u64::MAX
            "(call $gas64 (i64.const 18446744073709551615))", // 2^64 - 1
            "(call $gas64 (i64.const 0))",
            i64::MAX,
            1,
        ),
        (
            // What if the limit is 0?
            "(call $gas (i32.const 1))",
            "(call $gas (i32.const 0))",
            0,
            1,
        ),
        (
            // Mixed gas
            "(call $gas (i32.const 1))",
            "(call $gas64 (i64.const 1))",
            10,
            11,
        ),
    ];
    for (idx, (fee1, fee2, gas_limit, expected_hits)) in test_cases.iter().enumerate() {
        let hits = Arc::new(AtomicUsize::new(0));
        let wat = format!(
            r#"
            (import "host" "hit" (func))
            (func $gas (import "host" "gas") (param i32))
            (func $gas64 (import "host" "gas64") (param i64))
            (func $start
                loop
                    call 0
                    {fee1}
                    call 0
                    {fee2}
                    br 0
                end
            )
            (start $start)
            "#,
            fee1 = fee1,
            fee2 = fee2
        );
        let (_store, _module, instance) = instantiate(
            hits.clone(),
            wat.as_bytes(),
            InstanceConfig::default().with_gas_limit(*gas_limit),
        );
        match instance {
            Err(InstantiationError::Start(runtime_error)) => {
                assert_eq!(runtime_error.message(), "gas limit exceeded")
            }
            r => assert!(false, "test case {} did not fail due to gas limit: {:?}", idx, r),
        }
        // Ensure "func" was called twice.
        assert_eq!(hits.load(SeqCst), *expected_hits, "test case {} hit count mismatch", idx);
    }
}

#[test]
fn test_remaining_gas() {
    let test_cases = [
        ("(call $gas (i32.const 100))", 100, 0, true),
        ("(call $gas (i32.const 0))", 100, 100, true),
        ("(call $gas (i32.const 50)) (call $gas64 (i64.const 50))", 100, 0, true),
    ];
    for (idx, (fees, gas_limit, expected_remaining, success)) in test_cases.iter().enumerate() {
        let hits = Arc::new(AtomicUsize::new(0));
        let wat = format!(
            r#"
            (func $gas (import "host" "gas") (param i32))
            (func $gas64 (import "host" "gas64") (param i64))
            (func (export "main")
                {fees}
            )
            "#,
            fees = fees,
        );
        let (_store, _module, instance) = instantiate(
            hits.clone(),
            wat.as_bytes(),
            InstanceConfig::default().with_gas_limit(*gas_limit),
        );
        let instance = instance.unwrap();
        let main = instance
            .exports
            .get_function("main")
            .expect("expected function main");
        match main.call(&[]) {
            Err(runtime_error) if !success => {
                assert_eq!(runtime_error.message(), "gas limit exceeded")
            }
            Ok(_) if *success => {}
            r => assert!(false, "test case {} produce correct result: {:?}", idx, r),
        }
        assert_eq!(
            instance.gas_limit(), *expected_remaining,
            "test case {} remaining gas limit is wrong", idx);
    }
}

// #[test]
// fn test_gas_intrinsic_tricky() {
//     let store = get_store();
//     let module = get_module_tricky_arg(&store);
//     static BURNT_GAS: AtomicUsize = AtomicUsize::new(0);
//     static HITS: AtomicUsize = AtomicUsize::new(0);
//     let instance = Instance::new(
//         &module,
//         &imports! {
//             "host" => {
//                 "func" => Function::new(&store, FunctionType::new(vec![], vec![]), |_values| {
//                     HITS.fetch_add(1, SeqCst);
//                     Ok(vec![])
//                 }),
//                 "gas" => Function::new(&store, FunctionType::new(vec![ValType::I32], vec![]), |arg| {
//                     // It shall be called, as tricky call is not intrinsified.
//                     HITS.fetch_add(1, SeqCst);
//                     match arg[0] {
//                         Value::I32(arg) => {
//                             BURNT_GAS.fetch_add(arg as usize, SeqCst);
//                         },
//                         _ => {
//                             assert!(false)
//                         }
//                     }
//                     Ok(vec![])
//                 }),
//             },
//         },
//     );
//     assert!(instance.is_ok());
//     let instance = instance.unwrap();
//     let foo_func = instance
//         .exports
//         .get_function("foo")
//         .expect("expected function foo");
//
//     let _e = foo_func.call(&[]);
//
//     assert_eq!(BURNT_GAS.load(SeqCst), 1000000001);
//     // Ensure "gas" was called.
//     assert_eq!(HITS.load(SeqCst), 1);
//
//     let zoo_func = instance
//         .exports
//         .get_function("zoo")
//         .expect("expected function zoo");
//
//     let _e = zoo_func.call(&[]);
//     // We decremented gas by two.
//     assert_eq!(BURNT_GAS.load(SeqCst), 999999999);
//     // Ensure "gas" was called.
//     assert_eq!(HITS.load(SeqCst), 2);
// }

// #[test]
// fn test_gas_intrinsic_regular() {
//     let store = get_store();
//     let mut gas_counter = FastGasCounter::new(200);
//     let module = get_module(&store);
//     static HITS: AtomicUsize = AtomicUsize::new(0);
//     let instance = Instance::new_with_config(
//         &module,
//         unsafe { InstanceConfig::default().with_counter(ptr::addr_of_mut!(gas_counter)) },
//         &imports! {
//             "host" => {
//                 "func" => Function::new(&store, FunctionType::new(vec![], vec![]), |_values| {
//                     HITS.fetch_add(1, SeqCst);
//                     Ok(vec![])
//                 }),
//                 "has" => Function::new(&store, FunctionType::new(vec![ValType::I32], vec![]), |_| {
//                     HITS.fetch_add(1, SeqCst);
//                     Ok(vec![])
//                 }),
//                 "gas" => Function::new(&store, FunctionType::new(vec![ValType::I32], vec![]), |_| {
//                     // It shall be never called, as call is intrinsified.
//                     assert!(false);
//                     Ok(vec![])
//                 }),
//             },
//         },
//     );
//     assert!(instance.is_ok());
//     let instance = instance.unwrap();
//     let foo_func = instance
//         .exports
//         .get_function("foo")
//         .expect("expected function foo");
//     let bar_func = instance
//         .exports
//         .get_function("bar")
//         .expect("expected function bar");
//     let zoo_func = instance
//         .exports
//         .get_function("zoo")
//         .expect("expected function zoo");
//     // Ensure "func" was not called.
//     assert_eq!(HITS.load(SeqCst), 0);
//     let e = bar_func.call(&[]);
//     assert!(e.is_ok());
//     // Ensure "func" was called.
//     assert_eq!(HITS.load(SeqCst), 1);
//     assert_eq!(gas_counter.burnt(), 100);
//     let _e = foo_func.call(&[]).err().expect("error calling function");
//     // Ensure "func" and "has" was called again.
//     assert_eq!(HITS.load(SeqCst), 4);
//     assert_eq!(gas_counter.burnt(), 242);
//     // Finally try to exhaust rather large limit.
//     gas_counter.gas_limit += 100_000_000;
//     gas_counter.initial_gas_limit += 100_000_000;
//     let _e = zoo_func.call(&[]).err().expect("error calling function");
//     assert_eq!(gas_counter.burnt(), 100_000_242);
// }

// #[test]
// fn test_gas_intrinsic_default() {
//     let store = get_store();
//     let module = get_module(&store);
//     static HITS: AtomicUsize = AtomicUsize::new(0);
//     let instance = Instance::new(
//         &module,
//         &imports! {
//             "host" => {
//                 "func" => Function::new(&store, FunctionType::new(vec![], vec![]), |_values| {
//                     HITS.fetch_add(1, SeqCst);
//                     Ok(vec![])
//                 }),
//                 "has" => Function::new(&store, FunctionType::new(vec![ValType::I32], vec![]), |_| {
//                     HITS.fetch_add(1, SeqCst);
//                     Ok(vec![])
//                 }),
//                 "gas" => Function::new(&store, FunctionType::new(vec![ValType::I32], vec![]), |_| {
//                     // It shall be never called, as call is intrinsified.
//                     assert!(false);
//                     Ok(vec![])
//                 }),
//             },
//         },
//     );
//     assert!(instance.is_ok());
//     let instance = instance.unwrap();
//     let foo_func = instance
//         .exports
//         .get_function("foo")
//         .expect("expected function foo");
//     let bar_func = instance
//         .exports
//         .get_function("bar")
//         .expect("expected function bar");
//     // Ensure "func" was called.
//     assert_eq!(HITS.load(SeqCst), 0);
//     let e = bar_func.call(&[]);
//     assert!(e.is_ok());
//     // Ensure "func" was called.
//     assert_eq!(HITS.load(SeqCst), 1);
//     let _e = foo_func.call(&[]);
//     // Ensure "func" and "has" was called.
//     assert_eq!(HITS.load(SeqCst), 5);
// }






// fn get_module(store: &Store) -> Module {
//     let wat = r#"
//         (import "host" "func" (func))
//         (import "host" "has" (func (param i32)))
//         (import "host" "gas" (func (param i32)))
//         (memory $mem 1)
//         (export "memory" (memory $mem))
//         (func (export "foo")
//             call 0
//             i32.const 442
//             call 1
//             i32.const 42
//             call 2
//             call 0
//             i32.const 100
//             call 2
//             call 0
//         )
//         (func (export "bar")
//             call 0
//             i32.const 100
//             call 2
//         )
//         (func (export "zoo")
//             loop
//                 i32.const 100
//                 call 2
//                 br 0
//             end
//         )
//     "#;
//
//     Module::new(&store, &wat).unwrap()
// }

// fn get_module_tricky_arg(store: &Store) -> Module {
//     let wat = r#"
//         (import "host" "func" (func))
//         (import "host" "gas" (func (param i32)))
//         (memory $mem 1)
//         (export "memory" (memory $mem))
//         (func $get_gas (param i32) (result i32)
//          i32.const 1
//          get_local 0
//          i32.add)
//         (func (export "foo")
//             i32.const 1000000000
//             call $get_gas
//             call 1
//         )
//         (func (export "zoo")
//             i32.const -2
//             call 1
//         )
//     "#;
//
//     Module::new(&store, &wat).unwrap()
// }
