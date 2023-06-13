#[cfg(feature = "sys")]
mod sys {
    use anyhow::Result;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use wasmer::*;

    #[test]
    #[cfg_attr(feature = "singlepass", ignore)] // singlepass does not support funcref args.
    fn func_ref_passed_and_returned() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (import "env" "func_ref_identity" (func (param funcref) (result funcref)))
    (type $ret_i32_ty (func (result i32)))
    (table $table (export "table") 2 2 funcref)

    (func (export "run") (param) (result funcref)
          (call 0 (ref.null func)))
    (func (export "call_set_value") (param $fr funcref) (result i32)
          (table.set $table (i32.const 0) (local.get $fr))
          (call_indirect $table (type $ret_i32_ty) (i32.const 0)))
)"#;
        let module = Module::new(&store, wat)?;
        let func_ref_identity = Function::new(
            &store,
            FunctionType::new(vec![Type::FuncRef], vec![Type::FuncRef]),
            |values| -> Result<Vec<_>, _> { Ok(vec![values[0].clone()]) },
        );
        let imports = imports! {
            "env" => {
                "func_ref_identity" => func_ref_identity
            },
        };

        let instance = Instance::new(&module, &imports)?;

        let f: Function = instance.lookup_function("run").unwrap();
        let results = f.call(&[]).unwrap();
        if let Value::FuncRef(fr) = &results[0] {
            assert!(fr.is_none());
        } else {
            panic!("funcref not found!");
        }

        #[derive(Clone, Debug, WasmerEnv)]
        pub struct Env(Arc<AtomicBool>);
        let env = Env(Arc::new(AtomicBool::new(false)));

        let func_to_call = Function::new_native_with_env(&store, env.clone(), |env: &Env| -> i32 {
            env.0.store(true, Ordering::SeqCst);
            343
        });
        let call_set_value: Function = instance.lookup_function("call_set_value").unwrap();
        let results: Box<[Value]> = call_set_value.call(&[Value::FuncRef(Some(func_to_call))])?;
        assert!(env.0.load(Ordering::SeqCst));
        assert_eq!(&*results, &[Value::I32(343)]);

        Ok(())
    }

    #[test]
    #[cfg_attr(feature = "singlepass", ignore)] // singlepass does not support funcref args.
    fn func_ref_passed_and_called() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (func $func_ref_call (import "env" "func_ref_call") (param funcref) (result i32))
    (type $ret_i32_ty (func (result i32)))
    (table $table (export "table") 2 2 funcref)

    (func $product (param $x i32) (param $y i32) (result i32)
          (i32.mul (local.get $x) (local.get $y)))
    ;; TODO: figure out exactly why this statement is needed
    (elem declare func $product)
    (func (export "call_set_value") (param $fr funcref) (result i32)
          (table.set $table (i32.const 0) (local.get $fr))
          (call_indirect $table (type $ret_i32_ty) (i32.const 0)))
    (func (export "call_func") (param $fr funcref) (result i32)
          (call $func_ref_call (local.get $fr)))
    (func (export "call_host_func_with_wasm_func") (result i32)
          (call $func_ref_call (ref.func $product)))
)"#;
        let module = Module::new(&store, wat)?;

        fn func_ref_call(values: &[Value]) -> Result<Vec<Value>, RuntimeError> {
            // TODO: look into `Box<[Value]>` being returned breakage
            let f = values[0].unwrap_funcref().as_ref().unwrap();
            let f: NativeFunc<(i32, i32), i32> = f.native()?;
            Ok(vec![Value::I32(f.call(7, 9)?)])
        }

        let func_ref_call = Function::new(
            &store,
            FunctionType::new(vec![Type::FuncRef], vec![Type::I32]),
            func_ref_call,
        );
        let imports = imports! {
            "env" => {
                "func_ref_call" => func_ref_call,
                // TODO(reftypes): this should work
                /*
                "func_ref_call_native" => Function::new_native(&store, |f: Function| -> Result<i32, RuntimeError> {
                    let f: NativeFunc::<(i32, i32), i32> = f.native()?;
                    f.call(7, 9)
                })
                */
            },
        };

        let instance = Instance::new(&module, &imports)?;
        {
            fn sum(a: i32, b: i32) -> i32 {
                a + b
            }
            let sum_func = Function::new_native(&store, sum);

            let call_func: Function = instance.lookup_function("call_func").unwrap();
            let result = call_func.call(&[Value::FuncRef(Some(sum_func))])?;
            assert_eq!(result[0].unwrap_i32(), 16);
        }

        {
            let f: NativeFunc<(), i32> = instance
                .get_native_function("call_host_func_with_wasm_func")
                .unwrap();
            let result = f.call()?;
            assert_eq!(result, 63);
        }

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    #[cfg_attr(feature = "singlepass", ignore)] // singlepass does not support funcref args.
    fn extern_ref_passed_and_returned() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (func $extern_ref_identity (import "env" "extern_ref_identity") (param externref) (result externref))
    (func $extern_ref_identity_native (import "env" "extern_ref_identity_native") (param externref) (result externref))
    (func $get_new_extern_ref (import "env" "get_new_extern_ref") (result externref))
    (func $get_new_extern_ref_native (import "env" "get_new_extern_ref_native") (result externref))

    (func (export "run") (param) (result externref)
          (call $extern_ref_identity (ref.null extern)))
    (func (export "run_native") (param) (result externref)
          (call $extern_ref_identity_native (ref.null extern)))
    (func (export "get_hashmap") (param) (result externref)
          (call $get_new_extern_ref))
    (func (export "get_hashmap_native") (param) (result externref)
          (call $get_new_extern_ref_native))
)"#;
        let module = Module::new(&store, wat)?;
        let extern_ref_identity = Function::new(
            &store,
            FunctionType::new(vec![Type::ExternRef], vec![Type::ExternRef]),
            |values| -> Result<Vec<_>, _> { Ok(vec![values[0].clone()]) },
        );
        let extern_ref_identity_native =
            Function::new_native(&store, |er: ExternRef| -> ExternRef { er });
        let get_new_extern_ref = Function::new(
            &store,
            FunctionType::new(vec![], vec![Type::ExternRef]),
            |_| -> Result<Vec<_>, _> {
                let inner = [
                    ("hello".to_string(), "world".to_string()),
                    ("color".to_string(), "orange".to_string()),
                ]
                .iter()
                .cloned()
                .collect::<HashMap<String, String>>();
                let new_extern_ref = ExternRef::new(inner);
                Ok(vec![Value::ExternRef(new_extern_ref)])
            },
        );
        let get_new_extern_ref_native = Function::new_native(&store, || -> ExternRef {
            let inner = [
                ("hello".to_string(), "world".to_string()),
                ("color".to_string(), "orange".to_string()),
            ]
            .iter()
            .cloned()
            .collect::<HashMap<String, String>>();
            ExternRef::new(inner)
        });
        let imports = imports! {
            "env" => {
                "extern_ref_identity" => extern_ref_identity,
                "extern_ref_identity_native" => extern_ref_identity_native,
                "get_new_extern_ref" => get_new_extern_ref,
                "get_new_extern_ref_native" => get_new_extern_ref_native,
            },
        };

        let instance = Instance::new(&module, &imports)?;
        for run in &["run", "run_native"] {
            let f: Function = instance.lookup_function(run).unwrap();
            let results = f.call(&[]).unwrap();
            if let Value::ExternRef(er) = &results[0] {
                assert!(er.is_null());
            } else {
                panic!("result is not an extern ref!");
            }

            let f: NativeFunc<(), ExternRef> = instance.get_native_function(run).unwrap();
            let result: ExternRef = f.call()?;
            assert!(result.is_null());
        }

        for get_hashmap in &["get_hashmap", "get_hashmap_native"] {
            let f: Function = instance.lookup_function(get_hashmap).unwrap();
            let results = f.call(&[]).unwrap();
            if let Value::ExternRef(er) = &results[0] {
                let inner: &HashMap<String, String> = er.downcast().unwrap();
                assert_eq!(inner["hello"], "world");
                assert_eq!(inner["color"], "orange");
            } else {
                panic!("result is not an extern ref!");
            }

            let f: NativeFunc<(), ExternRef> = instance.get_native_function(get_hashmap).unwrap();

            let result: ExternRef = f.call()?;
            let inner: &HashMap<String, String> = result.downcast().unwrap();
            assert_eq!(inner["hello"], "world");
            assert_eq!(inner["color"], "orange");
        }

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    // TODO(reftypes): reenable this test
    #[ignore]
    fn extern_ref_ref_counting_basic() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (func (export "drop") (param $er externref) (result)
          (drop (local.get $er)))
)"#;
        let module = Module::new(&store, wat)?;
        let instance = Instance::new(&module, &imports! {})?;
        let f: NativeFunc<ExternRef, ()> = instance.get_native_function("drop").unwrap();

        let er = ExternRef::new(3u32);
        f.call(er.clone())?;

        assert_eq!(er.downcast::<u32>().unwrap(), &3);
        assert_eq!(er.strong_count(), 1);

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    fn refs_in_globals() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (global $er_global (export "er_global") (mut externref) (ref.null extern))
    (global $fr_global (export "fr_global") (mut funcref) (ref.null func))
    (global $fr_immutable_global (export "fr_immutable_global") funcref (ref.func $hello))
    (func $hello (param) (result i32)
          (i32.const 73))
)"#;
        let module = Module::new(&store, wat)?;
        let instance = Instance::new(&module, &imports! {})?;
        {
            let er_global = if let Some(Export::Global(g)) = instance.lookup("er_global") {
                g.from
            } else {
                panic!("no global");
            };

            if let Value::ExternRef(er) = er_global.get(&store) {
                assert!(er.is_null());
            } else {
                panic!("Did not find extern ref in the global");
            }

            unsafe {
                er_global.set(Val::ExternRef(ExternRef::new(3u32)))?;
            }

            if let Value::ExternRef(er) = er_global.get(&store) {
                assert_eq!(er.downcast::<u32>().unwrap(), &3);
                assert_eq!(er.strong_count(), 1);
            } else {
                panic!("Did not find extern ref in the global");
            }
        }

        {
            let fr_global = if let Some(Export::Global(g)) = instance.lookup("fr_immutable_global")
            {
                g.from
            } else {
                panic!("no global");
            };

            if let Value::FuncRef(Some(f)) = fr_global.get(&store) {
                let native_func: NativeFunc<(), u32> = f.native()?;
                assert_eq!(native_func.call()?, 73);
            } else {
                panic!("Did not find non-null func ref in the global");
            }
        }

        {
            let fr_global = if let Some(Export::Global(g)) = instance.lookup("fr_global") {
                g.from
            } else {
                panic!("no global");
            };

            if let Value::FuncRef(None) = fr_global.get(&store) {
            } else {
                panic!("Did not find a null func ref in the global");
            }

            let f = Function::new_native(&store, |arg1: i32, arg2: i32| -> i32 { arg1 + arg2 });

            unsafe {
                fr_global.set(Val::FuncRef(Some(f)))?;
            }

            if let Value::FuncRef(Some(f)) = fr_global.get(&store) {
                let native: NativeFunc<(i32, i32), i32> = f.native()?;
                assert_eq!(native.call(5, 7)?, 12);
            } else {
                panic!("Did not find extern ref in the global");
            }
        }

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    #[cfg_attr(feature = "singlepass", ignore)] // singlepass does not support funcref args.
    fn extern_ref_ref_counting_table_basic() -> Result<()> {
        use wasmer_vm::TableElement;

        let store = Store::default();
        let wat = r#"(module
    (global $global (export "global") (mut externref) (ref.null extern))
    (table $table (export "table") 4 4 externref)
    (func $insert (param $er externref) (param $idx i32)
           (table.set $table (local.get $idx) (local.get $er)))
    (func $intermediate (param $er externref) (param $idx i32)
          (call $insert (local.get $er) (local.get $idx)))
    (func $insert_into_table (export "insert_into_table") (param $er externref) (param $idx i32) (result externref)
          (call $intermediate (local.get $er) (local.get $idx))
          (local.get $er))
)"#;
        let module = Module::new(&store, wat)?;
        let instance = Instance::new(&module, &imports! {})?;

        let f: NativeFunc<(ExternRef, i32), ExternRef> =
            instance.get_native_function("insert_into_table").unwrap();

        let er = ExternRef::new(3usize);

        let er = f.call(er, 1)?;
        assert_eq!(er.strong_count(), 2);

        let table = if let Some(Export::Table(t)) = instance.lookup("table") {
            t.from
        } else {
            panic!("no table");
        };

        if let TableElement::ExternRef(er2) = table.get(1).unwrap() {
            assert_eq!(er2.strong_count(), 3);
        }

        assert_eq!(er.strong_count(), 2);
        table
            .set(1, TableElement::ExternRef(ExternRef::null()))
            .unwrap();

        assert_eq!(er.strong_count(), 1);

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    // TODO(reftypes): reenable this test
    #[ignore]
    fn extern_ref_ref_counting_global_basic() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (global $global (export "global") (mut externref) (ref.null extern))
    (func $get_from_global (export "get_from_global") (result externref)
          (drop (global.get $global))
          (global.get $global))
)"#;
        let module = Module::new(&store, wat)?;
        let instance = Instance::new(&module, &imports! {})?;

        let global = if let Some(Export::Global(g)) = instance.lookup("global") {
            g.from
        } else {
            panic!("not a global");
        };
        {
            let er = ExternRef::new(3usize);
            unsafe {
                global.set(Val::ExternRef(er.clone()))?;
            }
            assert_eq!(er.strong_count(), 2);
        }
        let get_from_global: NativeFunc<(), ExternRef> =
            instance.get_native_function("get_from_global").unwrap();

        let er = get_from_global.call()?;
        assert_eq!(er.strong_count(), 2);
        unsafe {
            global.set(Val::ExternRef(ExternRef::null()))?;
        }
        assert_eq!(er.strong_count(), 1);

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    // TODO(reftypes): reenable this test
    #[ignore]
    fn extern_ref_ref_counting_traps() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (func $pass_er (export "pass_extern_ref") (param externref)
          (local.get 0)
          (unreachable))
)"#;
        let module = Module::new(&store, wat)?;
        let instance = Instance::new(&module, &imports! {})?;

        let pass_extern_ref: NativeFunc<ExternRef, ()> =
            instance.get_native_function("pass_extern_ref").unwrap();

        let er = ExternRef::new(3usize);
        assert_eq!(er.strong_count(), 1);

        let result = pass_extern_ref.call(er.clone());
        assert!(result.is_err());
        assert_eq!(er.strong_count(), 1);

        Ok(())
    }

    #[cfg(feature = "experimental-reference-types-extern-ref")]
    #[test]
    #[cfg_attr(feature = "singlepass", ignore)] // singlepass does not support funcref args.
    fn extern_ref_ref_counting_table_instructions() -> Result<()> {
        use wasmer_vm::TableElement;

        let store = Store::default();
        let wat = r#"(module
    (table $table1 (export "table1") 2 12 externref)
    (table $table2 (export "table2") 6 12 externref)
    (func $grow_table_with_ref (export "grow_table_with_ref") (param $er externref) (param $size i32) (result i32)
          (table.grow $table1 (local.get $er) (local.get $size)))
    (func $fill_table_with_ref (export "fill_table_with_ref") (param $er externref) (param $start i32) (param $end i32)
          (table.fill $table1 (local.get $start) (local.get $er) (local.get $end)))
    (func $copy_into_table2 (export "copy_into_table2")
          (table.copy $table2 $table1 (i32.const 0) (i32.const 0) (i32.const 4)))
)"#;
        let module = Module::new(&store, wat)?;
        let instance = Instance::new(&module, &imports! {})?;

        let grow_table_with_ref: NativeFunc<(ExternRef, i32), i32> =
            instance.get_native_function("grow_table_with_ref").unwrap();
        let fill_table_with_ref: NativeFunc<(ExternRef, i32, i32), ()> =
            instance.get_native_function("fill_table_with_ref").unwrap();
        let copy_into_table2: NativeFunc<(), ()> =
            instance.get_native_function("copy_into_table2").unwrap();
        let (table1, table2) = if let (Some(Export::Table(t1)), Some(Export::Table(t2))) =
            (instance.lookup("table1"), instance.lookup("table2"))
        {
            (t1.from, t2.from)
        } else {
            panic!("can't get tables");
        };

        let er1 = ExternRef::new(3usize);
        let er2 = ExternRef::new(5usize);
        let er3 = ExternRef::new(7usize);
        {
            let result = grow_table_with_ref.call(er1.clone(), 0)?;
            assert_eq!(result, 2);
            assert_eq!(er1.strong_count(), 1);

            let result = grow_table_with_ref.call(er1.clone(), 10_000)?;
            assert_eq!(result, -1);
            assert_eq!(er1.strong_count(), 1);

            let result = grow_table_with_ref.call(er1.clone(), 8)?;
            assert_eq!(result, 2);
            assert_eq!(er1.strong_count(), 9);

            for i in 2..10 {
                if let TableElement::ExternRef(e) = table1.get(i).unwrap() {
                    assert_eq!(*e.downcast::<usize>().unwrap(), 3);
                    assert_eq!(&e, &er1);
                }
            }
            assert_eq!(er1.strong_count(), 9);
        }

        {
            fill_table_with_ref.call(er2.clone(), 0, 2)?;
            assert_eq!(er2.strong_count(), 3);
        }

        {
            table2.set(0, TableElement::ExternRef(er3.clone())).unwrap();
            table2.set(1, TableElement::ExternRef(er3.clone())).unwrap();
            table2.set(2, TableElement::ExternRef(er3.clone())).unwrap();
            table2.set(3, TableElement::ExternRef(er3.clone())).unwrap();
            table2.set(4, TableElement::ExternRef(er3.clone())).unwrap();
            assert_eq!(er3.strong_count(), 6);
        }

        {
            copy_into_table2.call()?;
            assert_eq!(er3.strong_count(), 2);
            assert_eq!(er2.strong_count(), 5);
            assert_eq!(er1.strong_count(), 11);
            for i in 1..5 {
                if let TableElement::ExternRef(e) = table2.get(i).unwrap() {
                    let value = e.downcast::<usize>().unwrap();
                    match i {
                        0 | 1 => assert_eq!(*value, 5),
                        4 => assert_eq!(*value, 7),
                        _ => assert_eq!(*value, 3),
                    }
                } else {
                    panic!("not extern ref");
                }
            }
        }

        {
            for i in 0..table1.size() {
                table1
                    .set(i, TableElement::ExternRef(ExternRef::null()))
                    .unwrap();
            }
            for i in 0..table2.size() {
                table2
                    .set(i, TableElement::ExternRef(ExternRef::null()))
                    .unwrap();
            }
        }

        assert_eq!(er1.strong_count(), 1);
        assert_eq!(er2.strong_count(), 1);
        assert_eq!(er3.strong_count(), 1);

        Ok(())
    }
}
