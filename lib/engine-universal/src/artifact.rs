//! Define `UniversalArtifact` to allow compiling and instantiating to be
//! done as separate steps.

use loupe::MemoryUsage;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use wasmer_compiler::Triple;
use wasmer_engine::{
    Artifact, GlobalFrameInfoRegistration, InstanceHandle, InstantiationError, RuntimeError,
};
use wasmer_types::entity::{BoxedSlice, PrimaryMap};
use wasmer_types::{
    DataIndex, ElemIndex, EntityCounts, FunctionIndex, GlobalInit, GlobalType, LocalFunctionIndex,
    MemoryType, OwnedDataInitializer, OwnedTableInitializer, SignatureIndex, TableType, LocalGlobalIndex,
};
use wasmer_vm::{
    FunctionBodyPtr, MemoryStyle, TableStyle, VMImport, VMLocalFunction, VMOffsets,
    VMSharedSignatureIndex, VMTrampoline,
};

use crate::instance::UniversalInstance;

#[derive(MemoryUsage)]
pub(crate) struct Data {
    // TODO: figure out how to allocate fewer structures onto heap. Maybe have an arena…?
    pub(crate) engine: crate::UniversalEngine,
    pub(crate) import_counts: EntityCounts,
    pub(crate) start_function: Option<FunctionIndex>,
    pub(crate) vmoffsets: VMOffsets,

    #[loupe(skip)] // TODO(0-copy): support loupe...
    pub(crate) imports: Vec<VMImport>,

    #[loupe(skip)]
    pub(crate) function_call_trampolines: BoxedSlice<SignatureIndex, VMTrampoline>,
    pub(crate) dynamic_function_trampolines: BoxedSlice<FunctionIndex, FunctionBodyPtr>,
    pub(crate) frame_info_registration: Mutex<Option<GlobalFrameInfoRegistration>>,
    pub(crate) functions: BoxedSlice<LocalFunctionIndex, VMLocalFunction>,

    pub(crate) local_memories: Vec<(MemoryType, MemoryStyle)>,
    pub(crate) data_segments: Vec<OwnedDataInitializer>,
    #[loupe(skip)] // TODO(0-copy): loupe skip...
    pub(crate) passive_data: BTreeMap<DataIndex, Arc<[u8]>>,

    pub(crate) local_tables: Vec<(TableType, TableStyle)>,
    pub(crate) element_segments: Vec<OwnedTableInitializer>,
    #[loupe(skip)] // TODO(0-copy): loupe skip...
    pub(crate) passive_elements: BTreeMap<ElemIndex, Box<[FunctionIndex]>>,

    pub(crate) local_globals: Vec<(GlobalType, GlobalInit)>,
}

/// A compiled wasm module, containing everything necessary for instantiation.
#[derive(MemoryUsage, Clone)]
pub struct UniversalArtifact {
    data: Arc<Data>,
}

impl UniversalArtifact {
    /// Get the default extension when serializing this artifact
    pub fn get_default_extension(_triple: &Triple) -> &'static str {
        // `.wasmu` is the default extension for all the triples. It
        // stands for “Wasm Universal”.
        "wasmu"
    }
}

impl Artifact for UniversalArtifact {
    unsafe fn instantiate(
        &self,
        tunables: &dyn wasmer_engine::Tunables,
        resolver: &dyn wasmer_engine::Resolver,
        host_state: Box<dyn std::any::Any>,
        config: wasmer_types::InstanceConfig,
    ) -> Result<wasmer_engine::InstanceHandle, wasmer_engine::InstantiationError> {
        let data = &*self.data;
        let (imports, import_function_envs) = {
            let mut imports = wasmer_engine::resolve_imports(
                &data.engine,
                resolver,
                &data.import_counts,
                &data.imports,
                &data.dynamic_function_trampolines,
            )
            .map_err(InstantiationError::Link)?;

            // Get the `WasmerEnv::init_with_instance` function pointers and the pointers
            // to the envs to call it on.
            let import_function_envs = imports.get_imported_function_envs();

            (imports, import_function_envs)
        };

        let (allocator, memory_definition_locations, table_definition_locations) =
            wasmer_vm::InstanceAllocator::new(data.vmoffsets.clone());

        // Memories
        let mut memories: PrimaryMap<wasmer_types::LocalMemoryIndex, _> =
            PrimaryMap::with_capacity(data.local_memories.len());
        for (idx, (ty, style)) in (data.import_counts.memories..).zip(data.local_memories.iter()) {
            let memory = tunables
                .create_vm_memory(&ty, &style, memory_definition_locations[idx])
                .map_err(|e| {
                    InstantiationError::Link(wasmer_engine::LinkError::Resource(format!(
                        "Failed to create memory: {}",
                        e
                    )))
                })?;
            memories.push(memory);
        }

        // Tables
        let mut tables: PrimaryMap<wasmer_types::LocalTableIndex, _> =
            PrimaryMap::with_capacity(data.local_tables.len());
        for (idx, (ty, style)) in (data.import_counts.tables..).zip(data.local_tables.iter()) {
            let table = tunables
                .create_vm_table(ty, style, table_definition_locations[idx])
                .map_err(|e| InstantiationError::Link(wasmer_engine::LinkError::Resource(e)))?;
            tables.push(table);
        }

        // Globals
        let mut globals =
            PrimaryMap::<LocalGlobalIndex, _>::with_capacity(data.local_globals.len());
        for (ty, _) in data.local_globals.iter() {
            globals.push(Arc::new(wasmer_vm::Global::new(*ty)));
        }

        let instance = Arc::new(UniversalInstance {
            artifact: self.data.clone(),
            config,
            // memories: memories.into_boxed_slice(),
            // tables: tables.into_boxed_slice(),
            // globals: globals.into_boxed_slice(),
        }) as Arc<_>;

        Ok(InstanceHandle { instance })

        // TODO(0-copy): avoid the clones here, just keep reference to the artifact in the
        // instance.
        // let handle = wasmer_vm::InstanceHandle::new(
        //     allocator,
        //     self.finished_functions.clone(),
        //     self.finished_function_call_trampolines.clone(),
        //     vmctx_memories.into_boxed_slice(),
        //     vmctx_tables.into_boxed_slice(),
        //     vmctx_globals.into_boxed_slice(),
        //     imports,
        //     self.signatures.clone(),
        //     self.passive_data.clone(),
        //     host_state,
        //     import_function_envs,
        //     config,
        // )
        // .map_err(|trap| InstantiationError::Start(RuntimeError::from_trap(trap)))?;
        // Ok(handle)
    }
}
