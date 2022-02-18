//! Define `UniversalArtifact` to allow compiling and instantiating to be
//! done as separate steps.

use loupe::MemoryUsage;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use wasmer_compiler::Triple;
use wasmer_engine::{GlobalFrameInfoRegistration, InstantiationError, RuntimeError};
use wasmer_types::entity::{BoxedSlice, PrimaryMap};
use wasmer_types::{
    DataIndex, ElemIndex, EntityCounts, FunctionIndex, GlobalInit, GlobalType, LocalFunctionIndex,
    LocalGlobalIndex, MemoryType, OwnedDataInitializer, OwnedTableInitializer, SignatureIndex,
    TableType,
};
use wasmer_vm::{
    Artifact, FunctionBodyPtr, FunctionExtent, InstanceHandle, MemoryStyle, Resolver, TableStyle,
    Tunables, VMImport, VMLocalFunction, VMOffsets, VMTrampoline,
};

/// A compiled wasm module, containing everything necessary for instantiation.
#[derive(MemoryUsage)]
pub struct UniversalArtifact {
    // TODO: figure out how to allocate fewer distinct structures onto heap. Maybe have an arena…?
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
    #[loupe(skip)] // TODO(0-copy):
    pub(crate) exported_functions: BTreeMap<String, FunctionIndex>,

    pub(crate) local_memories: Vec<(MemoryType, MemoryStyle)>,
    pub(crate) data_segments: Vec<OwnedDataInitializer>,
    #[loupe(skip)] // TODO(0-copy): loupe skip...
    pub(crate) passive_data: BTreeMap<DataIndex, Arc<[u8]>>,

    pub(crate) local_tables: Vec<(TableType, TableStyle)>,
    pub(crate) element_segments: Vec<OwnedTableInitializer>,
    #[loupe(skip)] // TODO(0-copy): loupe skip...
    // TODO: does this need to be a BTreeMap? Can it be a plain vector?
    pub(crate) passive_elements: BTreeMap<ElemIndex, Box<[FunctionIndex]>>,

    pub(crate) local_globals: Vec<(GlobalType, GlobalInit)>,
}

impl UniversalArtifact {
    /// Get the default extension when serializing this artifact
    pub fn get_default_extension(_triple: &Triple) -> &'static str {
        // `.wasmu` is the default extension for all the triples. It
        // stands for “Wasm Universal”.
        "wasmu"
    }

    /// Return the extents of the specified local function.
    pub fn function_extent(&self, index: LocalFunctionIndex) -> Option<FunctionExtent> {
        let func = self.functions.get(index)?;
        Some(FunctionExtent {
            address: func.body,
            length: usize::try_from(func.length).unwrap(),
        })
    }
}

impl Artifact for UniversalArtifact {
    unsafe fn instantiate(
        self: Arc<Self>,
        tunables: &dyn Tunables,
        resolver: &dyn Resolver,
        host_state: Box<dyn std::any::Any>,
        config: wasmer_types::InstanceConfig,
    ) -> Result<InstanceHandle, Box<dyn std::error::Error + Send + Sync>> {
        let (imports, import_function_envs) = {
            let mut imports = wasmer_engine::resolve_imports(
                &self.engine,
                resolver,
                &self.import_counts,
                &self.imports,
                &self.dynamic_function_trampolines,
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
        let mut memories: PrimaryMap<wasmer_types::LocalMemoryIndex, _> =
            PrimaryMap::with_capacity(self.local_memories.len());
        for (idx, (ty, style)) in (self.import_counts.memories..).zip(self.local_memories.iter()) {
            let memory = tunables
                .create_vm_memory(&ty, &style, memory_definition_locations[idx as usize])
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
            PrimaryMap::with_capacity(self.local_tables.len());
        for (idx, (ty, style)) in (self.import_counts.tables..).zip(self.local_tables.iter()) {
            let table = tunables
                .create_vm_table(ty, style, table_definition_locations[idx as usize])
                .map_err(|e| InstantiationError::Link(wasmer_engine::LinkError::Resource(e)))?;
            tables.push(table);
        }

        // Globals
        let mut globals =
            PrimaryMap::<LocalGlobalIndex, _>::with_capacity(self.local_globals.len());
        for (ty, _) in self.local_globals.iter() {
            globals.push(Arc::new(wasmer_vm::Global::new(*ty)));
        }

        let passive_data = self.passive_data.clone();
        Ok(InstanceHandle::new(
            self,
            allocator,
            memories.into_boxed_slice(),
            tables.into_boxed_slice(),
            globals.into_boxed_slice(),
            imports,
            passive_data,
            host_state,
            import_function_envs,
            config,
        ) // FIXME(0-copy): wrong error type, fix
        .map_err(|t| InstantiationError::Start(RuntimeError::from_trap(t)))?)
    }

    fn offsets(&self) -> &wasmer_vm::VMOffsets {
        &self.vmoffsets
    }

    fn import_counts(&self) -> &EntityCounts {
        &self.import_counts
    }

    fn functions(&self) -> &BoxedSlice<LocalFunctionIndex, VMLocalFunction> {
        &self.functions
    }

    fn passive_elements(&self) -> &BTreeMap<ElemIndex, Box<[FunctionIndex]>> {
        &self.passive_elements
    }

    fn element_segments(&self) -> &[OwnedTableInitializer] {
        &self.element_segments[..]
    }

    fn data_segments(&self) -> &[OwnedDataInitializer] {
        &self.data_segments[..]
    }

    fn globals(&self) -> &[(GlobalType, GlobalInit)] {
        &self.local_globals[..]
    }

    fn start_function(&self) -> Option<FunctionIndex> {
        self.start_function
    }

    fn function_by_export_field(&self, name: &str) -> Option<FunctionIndex> {
        self.exported_functions.get(name).copied()
    }

    fn function_trampoline(&self, idx: wasmer_vm::VMSharedSignatureIndex) -> Option<VMTrampoline> {
        self.engine.inner().signatures.lookup(idx).map(|r| *r.1)
    }
}
