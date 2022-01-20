//! Define `UniversalArtifact` to allow compiling and instantiating to be
//! done as separate steps.

use crate::UniversalExecutable;
use loupe::MemoryUsage;
use std::sync::{Arc, Mutex};
use wasmer_compiler::Triple;
use wasmer_engine::{
    Artifact, Executable, GlobalFrameInfoRegistration, InstantiationError, RuntimeError,
};
use wasmer_types::entity::BoxedSlice;
use wasmer_types::{
    DataInitializer, FunctionIndex, LocalFunctionIndex, ModuleInfo, SignatureIndex,
};
use wasmer_vm::{FuncDataRegistry, FunctionBodyPtr, VMSharedSignatureIndex, VMTrampoline};

/// A compiled wasm module, ready to be instantiated.
#[derive(MemoryUsage)]
pub struct UniversalArtifact {
    // TODO: remove this
    pub(crate) executable: UniversalExecutable,
    pub(crate) finished_functions: BoxedSlice<LocalFunctionIndex, FunctionBodyPtr>,
    #[loupe(skip)]
    pub(crate) finished_function_call_trampolines: BoxedSlice<SignatureIndex, VMTrampoline>,
    pub(crate) finished_dynamic_function_trampolines: BoxedSlice<FunctionIndex, FunctionBodyPtr>,
    pub(crate) signatures: BoxedSlice<SignatureIndex, VMSharedSignatureIndex>,
    pub(crate) func_data_registry: Arc<FuncDataRegistry>,
    pub(crate) frame_info_registration: Mutex<Option<GlobalFrameInfoRegistration>>,
    pub(crate) finished_function_lengths: BoxedSlice<LocalFunctionIndex, usize>,
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
    fn module_ref(&self) -> &ModuleInfo {
        &self.executable.compile_info.module
    }

    fn module_mut(&mut self) -> Option<&mut ModuleInfo> {
        Arc::get_mut(&mut self.executable.compile_info.module)
    }

    /// The features with which this `Executable` was built.
    fn features(&self) -> &wasmer_compiler::Features {
        &self.executable.compile_info.features
    }

    fn finished_functions(&self) -> &BoxedSlice<LocalFunctionIndex, FunctionBodyPtr> {
        &self.finished_functions
    }

    fn finished_functions_lengths(&self) -> &BoxedSlice<LocalFunctionIndex, usize> {
        &self.finished_function_lengths
    }

    fn finished_function_call_trampolines(&self) -> &BoxedSlice<SignatureIndex, VMTrampoline> {
        &self.finished_function_call_trampolines
    }

    fn finished_dynamic_function_trampolines(&self) -> &BoxedSlice<FunctionIndex, FunctionBodyPtr> {
        &self.finished_dynamic_function_trampolines
    }

    fn signatures(&self) -> &BoxedSlice<SignatureIndex, VMSharedSignatureIndex> {
        &self.signatures
    }

    fn func_data_registry(&self) -> &FuncDataRegistry {
        &self.func_data_registry
    }

    unsafe fn instantiate(
        &self,
        tunables: &dyn wasmer_engine::Tunables,
        resolver: &dyn wasmer_engine::Resolver,
        host_state: Box<dyn std::any::Any>,
        config: wasmer_types::InstanceConfig,
    ) -> Result<wasmer_vm::InstanceHandle, wasmer_engine::InstantiationError> {
        let module = Arc::clone(&self.executable.compile_info.module);
        let (imports, import_function_envs) = {
            let mut imports = wasmer_engine::resolve_imports(
                &module,
                resolver,
                &self.finished_dynamic_function_trampolines(),
                self.executable.memory_styles(),
                self.executable.table_styles(),
            )
            .map_err(InstantiationError::Link)?;

            // Get the `WasmerEnv::init_with_instance` function pointers and the pointers
            // to the envs to call it on.
            let import_function_envs = imports.get_imported_function_envs();

            (imports, import_function_envs)
        };

        // Get pointers to where metadata about local memories should live in VM memory.
        // Get pointers to where metadata about local tables should live in VM memory.

        let (allocator, memory_definition_locations, table_definition_locations) =
            wasmer_vm::InstanceAllocator::new(&*module);
        let finished_memories = tunables
            .create_memories(
                &module,
                self.executable.memory_styles(),
                &memory_definition_locations,
            )
            .map_err(InstantiationError::Link)?
            .into_boxed_slice();
        let finished_tables = tunables
            .create_tables(
                &module,
                self.executable.table_styles(),
                &table_definition_locations,
            )
            .map_err(InstantiationError::Link)?
            .into_boxed_slice();
        let finished_globals = tunables
            .create_globals(&module)
            .map_err(InstantiationError::Link)?
            .into_boxed_slice();
        let handle = wasmer_vm::InstanceHandle::new(
            allocator,
            module,
            self.finished_functions().clone(),
            self.finished_functions_lengths().clone(),
            self.finished_function_call_trampolines().clone(),
            finished_memories,
            finished_tables,
            finished_globals,
            imports,
            self.signatures().clone(),
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
        let data_initializers = self
            .executable
            .data_initializers()
            .iter()
            .map(|init| DataInitializer {
                location: init.location.clone(),
                data: &*init.data,
            })
            .collect::<Vec<_>>();
        handle
            .finish_instantiation(&data_initializers)
            .map_err(|trap| InstantiationError::Start(RuntimeError::from_trap(trap)))
    }
}
