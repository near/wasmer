use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use wasmer::*;
use wasmer_engine_universal::UniversalExecutableRef;

fn call_many_functions(n: usize) -> String {
    let fndefs = (0..n)
        .map(|idx| format!(r#"(func $fn{idx} return)"#, idx = idx))
        .collect::<String>();
    let calls = (0..n)
        .map(|idx| format!("call $fn{idx}\n", idx = idx))
        .collect::<String>();
    format!(
        r#"(module {fndefs} (func (export "main") {calls} return) (func (export "single") call $fn0 return))"#,
        fndefs = fndefs,
        calls = calls
    )
}

fn nops(c: &mut Criterion) {
    for size in [1, 10, 100, 1000, 10000] {
        let wat = call_many_functions(size);
        let store = Store::new(&Universal::new(Singlepass::new()).engine());
        let mut compile = c.benchmark_group("compile");
        compile.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let module = Module::new(&store, &wat).unwrap();
                let imports = imports! {};
                let _ = Instance::new(&module, &imports).unwrap();
            })
        });
        drop(compile);

        let module = Module::new(&store, &wat).unwrap();
        let imports = imports! {};
        let instance = Instance::new(&module, &imports).unwrap();
        let mut get_main = c.benchmark_group("get_main");
        get_main.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let _: Function = instance.lookup_function("main").unwrap();
            })
        });
        drop(get_main);
        let main: Function = instance.lookup_function("main").unwrap();
        let mut call_main = c.benchmark_group("call_main");
        call_main.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(main.call(&[]).unwrap());
            })
        });
        drop(call_main);

        let single: Function = instance.lookup_function("single").unwrap();
        let mut call_single = c.benchmark_group("call_single");
        call_single.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(single.call(&[]).unwrap());
            })
        });
        drop(call_single);

        let mut serialize = c.benchmark_group("serialize");
        let wasm = wat::parse_bytes(wat.as_ref()).unwrap();
        let executable = store.engine().compile(&wasm, store.tunables()).unwrap();
        serialize.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(executable.serialize().unwrap());
            })
        });
        drop(serialize);

        let serialized = executable.serialize().unwrap();
        let mut deserialize = c.benchmark_group("deserialize");
        deserialize.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| unsafe {
                let deserialized = UniversalExecutableRef::deserialize(&serialized).unwrap();
                black_box(store.engine().load(&deserialized).unwrap());
            })
        });
    }
}

criterion_group!(benches, nops);

criterion_main!(benches);
