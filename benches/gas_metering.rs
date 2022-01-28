use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wasmer::*;
use wasmer_types::InstanceConfig;

fn instantiate(
    wasm: &[u8],
    config: InstanceConfig,
) -> (Store, Module, Result<Instance, InstantiationError>) {
    let compiler = Singlepass::default();
    let store = Store::new(&Universal::new(compiler).engine());
    let module = Module::new(&store, wasm).unwrap();
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
        &imports! { "host" => { "gas" => gas, "gas64" => gas64 } },
    );
    (store, module, instance)
}

fn gas(c: &mut Criterion) {
    let cases = [
        ("gas32", 0, "(call $gas (i32.const 0))"),
        ("gas32", 1, "(call $gas (i32.const 1))"),
    ];
    let mut group = c.benchmark_group("gas");
    for (name, size, fee) in cases {
        let wat = format!(
            r#"
            (func $gas (import "host" "gas") (param i32))
            (func $gas64 (import "host" "gas64") (param i64))
            (func (export "main") (local i32)
                loop
                    {fee}
                    (i32.add (local.get 0) (i32.const 1))
                    local.set 0
                    (br_if 0 (i32.lt_u (local.get 0) (i32.const 10000)))
                end
            )
            "#,
            fee = fee,
        );
        group.bench_with_input(BenchmarkId::new(name, size), &size, |b, _| {
            let mut gas_counter = wasmer_types::FastGasCounter::new(u64::MAX, 1);
            let (_, _, instance) = instantiate(wat.as_bytes(), unsafe {
                InstanceConfig::default().with_counter(std::ptr::addr_of_mut!(gas_counter))
            });
            let instance = instance.unwrap();
            let main = instance
                .exports
                .get_function("main")
                .expect("expected function main");
            b.iter(|| {
                main.call(&[]).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, gas);
criterion_main!(benches);
