use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::ser::serializers::AllocSerializer;
use std::{ptr::NonNull, sync::Arc};
use wasmer_compiler::CpuFeature;
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::{Universal, UniversalExecutable};
use wasmer_types::{MemoryType, TableType};
use wasmer_vm::{
    Memory, MemoryError, MemoryStyle, Table, TableStyle, VMMemoryDefinition, VMTableDefinition,
};

fn main() {
    let seeds = [2, 3, 5, 7, 11, 13, 17, 21];
    for seed in seeds {
        let contract = std::fs::read(format!("examples/code{seed}")).unwrap();
        let mut features = CpuFeature::set();
        features.insert(CpuFeature::AVX);
        let triple = "x86_64-unknown-linux-gnu".parse().unwrap();
        let target = wasmer_compiler::Target::new(triple, features);
        let compiler = Singlepass::new();
        let engine = Universal::new(compiler)
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
        let mut serializer = AllocSerializer::<1024>::default();
        let pos = rkyv::ser::Serializer::serialize_value(&mut serializer, &executable).unwrap();
        let data = serializer.into_serializer().into_inner();
        let archive = unsafe { rkyv::archived_value::<UniversalExecutable>(&data, pos) };
        let mut deserializer = SharedDeserializeMap::new();
        let owned: UniversalExecutable = rkyv::Deserialize::deserialize(archive, &mut deserializer)
            .expect("could not convert to owned executable");
        println!("{:#?}", owned);
    }
}

struct Tunables;
impl wasmer_vm::Tunables for Tunables {
    fn memory_style(&self, memory: &MemoryType) -> MemoryStyle {
        MemoryStyle::Static {
            bound: memory.maximum.unwrap_or(wasmer_types::Pages(10)),
            offset_guard_size: 0x10000u64,
        }
    }

    fn table_style(&self, _table: &TableType) -> TableStyle {
        TableStyle::CallerChecksSignature
    }

    fn create_host_memory(
        &self,
        _: &MemoryType,
        _: &MemoryStyle,
    ) -> Result<Arc<dyn Memory>, MemoryError> {
        todo!()
    }

    unsafe fn create_vm_memory(
        &self,
        _: &MemoryType,
        _: &MemoryStyle,
        _vm_definition_location: NonNull<VMMemoryDefinition>,
    ) -> Result<Arc<dyn Memory>, MemoryError> {
        todo!()
    }

    fn create_host_table(&self, _: &TableType, _: &TableStyle) -> Result<Arc<dyn Table>, String> {
        todo!()
    }

    unsafe fn create_vm_table(
        &self,
        _: &TableType,
        _: &TableStyle,
        _: NonNull<VMTableDefinition>,
    ) -> Result<Arc<dyn Table>, String> {
        todo!()
    }
}
