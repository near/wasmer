//! Define `UniversalArtifact` to allow compiling and instantiating to be
//! done as separate steps.

use loupe::MemoryUsage;
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex};
use wasmer_compiler::Triple;
use wasmer_engine::{Artifact, GlobalFrameInfoRegistration, InstantiationError, RuntimeError};
use wasmer_types::entity::{BoxedSlice, PrimaryMap};
use wasmer_types::{
    EntityCounts, FunctionIndex, GlobalType, LocalFunctionIndex, MemoryType, OwnedDataInitializer,
    SignatureIndex, TableType, DataIndex,
};
use wasmer_vm::{
    FunctionBodyPtr, MemoryStyle, TableStyle, VMImport, VMLocalFunction, VMOffsets,
    VMSharedSignatureIndex, VMTrampoline,
};

/// A compiled wasm module, containing everything necessary for instantiation.
#[derive(MemoryUsage)]
pub struct UniversalArtifact {
    // TODO: figure out how to allocate fewer structures onto heap. Maybe have an arena…?
    pub(crate) engine: crate::UniversalEngine,
    pub(crate) import_counts: EntityCounts,
    #[loupe(skip)] // TODO(0-copy): support loupe...
    pub(crate) imports: Vec<VMImport>,
    pub(crate) finished_functions: BoxedSlice<LocalFunctionIndex, VMLocalFunction>,
    #[loupe(skip)]
    pub(crate) finished_function_call_trampolines: BoxedSlice<SignatureIndex, VMTrampoline>,
    pub(crate) finished_dynamic_function_trampolines: BoxedSlice<FunctionIndex, FunctionBodyPtr>,
    pub(crate) signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
    pub(crate) frame_info_registration: Mutex<Option<GlobalFrameInfoRegistration>>,
    pub(crate) data_initializers: Vec<OwnedDataInitializer>,
    pub(crate) local_memories: Vec<(MemoryType, MemoryStyle)>,
    pub(crate) local_tables: Vec<(TableType, TableStyle)>,
    pub(crate) local_globals: Vec<GlobalType>,
    #[loupe(skip)] // TODO(0-copy): loupe skip...
    pub(crate) passive_data: BTreeMap<DataIndex, Arc<[u8]>>,
    pub(crate) vmoffsets: VMOffsets,
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
    ) -> Result<wasmer_vm::InstanceHandle, wasmer_engine::InstantiationError> {
        let (imports, import_function_envs) = {
            let mut imports = wasmer_engine::resolve_imports(
                &self.engine,
                resolver,
                &self.import_counts,
                &self.imports,
                &self.finished_dynamic_function_trampolines,
            )
            .map_err(InstantiationError::Link)?;

            // Get the `WasmerEnv::init_with_instance` function pointers and the pointers
            // to the envs to call it on.
            let import_function_envs = imports.get_imported_function_envs();

            (imports, import_function_envs)
        };

        let (allocator, memory_definition_locations, table_definition_locations) =
            wasmer_vm::InstanceAllocator::new(self.vmoffsets.clone());

        // Memories
        let mut vmctx_memories: PrimaryMap<wasmer_types::LocalMemoryIndex, _> =
            PrimaryMap::with_capacity(self.local_memories.len());
        for (idx, (ty, style)) in (self.import_counts.memories..).zip(self.local_memories.iter()) {
            let memory = tunables
                .create_vm_memory(&ty, &style, memory_definition_locations[idx])
                .map_err(|e| {
                    InstantiationError::Link(wasmer_engine::LinkError::Resource(format!(
                        "Failed to create memory: {}",
                        e
                    )))
                })?;
            vmctx_memories.push(memory);
        }

        // Tables
        let mut vmctx_tables: PrimaryMap<wasmer_types::LocalTableIndex, _> =
            PrimaryMap::with_capacity(self.local_tables.len());
        for (idx, (ty, style)) in (self.import_counts.tables..).zip(self.local_tables.iter()) {
            let table = tunables
                .create_vm_table(ty, style, table_definition_locations[idx])
                .map_err(|e| InstantiationError::Link(wasmer_engine::LinkError::Resource(e)))?;
            vmctx_tables.push(table);
        }

        // Globals
        let mut vmctx_globals = PrimaryMap::with_capacity(self.local_globals.len());
        for ty in self.local_globals.iter() {
            vmctx_globals.push(Arc::new(wasmer_vm::Global::new(*ty)));
        }

        // TODO(0-copy): avoid the clones here, just keep reference to the artifact in the
        // instance.
        let handle = wasmer_vm::InstanceHandle::new(
            allocator,
            self.finished_functions.clone(),
            self.finished_function_call_trampolines.clone(),
            vmctx_memories.into_boxed_slice(),
            vmctx_tables.into_boxed_slice(),
            vmctx_globals.into_boxed_slice(),
            imports,
            self.signatures.clone(),
            self.passive_data.clone(),
            host_state,
            import_function_envs,
            config,
        )
        .map_err(|trap| InstantiationError::Start(RuntimeError::from_trap(trap)))?;
        Ok(handle)
    }

    unsafe fn finish_instantiation(
        &self,
        handle: &wasmer_vm::InstanceHandle,
    ) -> Result<(), wasmer_engine::InstantiationError> {
        handle
            .finish_instantiation(self.data_initializers.iter().map(Into::into))
            .map_err(|trap| InstantiationError::Start(RuntimeError::from_trap(trap)))
    }
}
