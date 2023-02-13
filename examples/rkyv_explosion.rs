use wasmer_engine::Executable;

fn main() {
    let seeds = [2, 3, 5, 7, 11, 13, 17, 21];
    for seed in seeds {
        let contract = std::fs::read(format!("examples/code{seed}")).unwrap();
        let mut features = wasmer_compiler::CpuFeature::set();
        features.insert(wasmer_compiler::CpuFeature::AVX);
        let triple = "x86_64-unknown-linux-gnu".parse().unwrap();
        let target = wasmer_compiler::Target::new(triple, features);
        let compiler = wasmer_compiler_singlepass::Singlepass::new();
        let engine = wasmer_engine_universal::Universal::new(compiler)
            .target(target)
            .features(wasmer_compiler::Features {
                threads: false,
                reference_types: false,
                simd: false,
                bulk_memory: false,
                multi_value: false,
                tail_call: false,
                module_linking: false,
                multi_memory: false,
                memory64: false,
                exceptions: false,
            })
            .engine();
        let executable = engine.compile_universal(&contract, &Tunables).unwrap();
        let serialized = executable.serialize().unwrap();
        let serialized = std::hint::black_box(serialized);

        let executable = unsafe {
            wasmer_engine_universal::UniversalExecutableRef::deserialize(&serialized)
                .expect("could not deserialize?!")
        };
        let owned = executable
            .to_owned()
            .expect("could not convert to owned executable");
        println!("{:#?}", owned);
    }
}

struct Tunables;
impl wasmer_vm::Tunables for Tunables {
    fn memory_style(&self, memory: &wasmer_types::MemoryType) -> wasmer_vm::MemoryStyle {
        wasmer_vm::MemoryStyle::Static {
            bound: memory.maximum.unwrap_or(wasmer_types::Pages(10)),
            offset_guard_size: 0x10000u64,
        }
    }

    fn table_style(&self, _table: &wasmer_types::TableType) -> wasmer_vm::TableStyle {
        wasmer_vm::TableStyle::CallerChecksSignature
    }

    fn create_host_memory(
        &self,
        _: &wasmer_types::MemoryType,
        _: &wasmer_vm::MemoryStyle,
    ) -> Result<std::sync::Arc<dyn wasmer_vm::Memory>, wasmer_vm::MemoryError> {
        todo!()
    }

    unsafe fn create_vm_memory(
        &self,
        _: &wasmer_types::MemoryType,
        _: &wasmer_vm::MemoryStyle,
        _vm_definition_location: std::ptr::NonNull<wasmer_vm::VMMemoryDefinition>,
    ) -> Result<std::sync::Arc<dyn wasmer_vm::Memory>, wasmer_vm::MemoryError> {
        todo!()
    }

    fn create_host_table(
        &self,
        _: &wasmer_types::TableType,
        _: &wasmer_vm::TableStyle,
    ) -> Result<std::sync::Arc<dyn wasmer_vm::Table>, String> {
        todo!()
    }

    unsafe fn create_vm_table(
        &self,
        _: &wasmer_types::TableType,
        _: &wasmer_vm::TableStyle,
        _: std::ptr::NonNull<wasmer_vm::VMTableDefinition>,
    ) -> Result<std::sync::Arc<dyn wasmer_vm::Table>, String> {
        todo!()
    }
}
