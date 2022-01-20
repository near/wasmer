use crate::{InstantiationError, Resolver, Tunables};

use loupe::MemoryUsage;
use std::any::Any;
use wasmer_types::entity::BoxedSlice;
use wasmer_types::{FunctionIndex, InstanceConfig, LocalFunctionIndex, SignatureIndex};
use wasmer_vm::{
    FuncDataRegistry, FunctionBodyPtr, InstanceHandle, VMSharedSignatureIndex, VMTrampoline,
};

/// A predecesor of a full module Instance.
///
/// This type represents an intermediate stage within WASM module instantiation process. At the
/// time this type exists parts of an [`Executable`](crate::Executable) are pre-allocated in the
/// [`Engine`](crate::Engine) (aka. WASM Store.)
///
/// Some other operations such as linking, relocating and similar may also be performed during
/// constructon of the Artifact, making this type particularly well suited for caching in-memory.
pub trait Artifact: Send + Sync + Upcastable + MemoryUsage {
    /// Get reference to module
    ///
    /// TODO: remove
    fn module_ref(&self) -> &wasmer_types::ModuleInfo;

    /// Get reference to module
    ///
    /// TODO: remove
    fn module_mut(&mut self) -> Option<&mut wasmer_types::ModuleInfo>;

    /// The features with which this `Executable` was built.
    fn features(&self) -> &wasmer_compiler::Features;

    /// Returns the functions allocated in memory or this `Artifact`
    /// ready to be run.
    fn finished_functions(&self) -> &BoxedSlice<LocalFunctionIndex, FunctionBodyPtr>;

    /// Returns the functions code length.
    fn finished_functions_lengths(&self) -> &BoxedSlice<LocalFunctionIndex, usize>;

    /// Returns the function call trampolines allocated in memory of this
    /// `Artifact`, ready to be run.
    fn finished_function_call_trampolines(&self) -> &BoxedSlice<SignatureIndex, VMTrampoline>;

    /// Returns the dynamic function trampolines allocated in memory
    /// of this `Artifact`, ready to be run.
    fn finished_dynamic_function_trampolines(&self) -> &BoxedSlice<FunctionIndex, FunctionBodyPtr>;

    /// Returns the associated VM signatures for this `Artifact`.
    fn signatures(&self) -> &BoxedSlice<SignatureIndex, VMSharedSignatureIndex>;

    /// Get the func data registry
    fn func_data_registry(&self) -> &FuncDataRegistry;

    /// Crate an `Instance` from this `Artifact`.
    ///
    /// # Safety
    ///
    /// See [`InstanceHandle::new`].
    unsafe fn instantiate(
        &self,
        tunables: &dyn Tunables,
        resolver: &dyn Resolver,
        host_state: Box<dyn Any>,
        config: InstanceConfig,
    ) -> Result<InstanceHandle, InstantiationError>;

    /// Finishes the instantiation of a just created `InstanceHandle`.
    ///
    /// # Safety
    ///
    /// See [`InstanceHandle::finish_instantiation`].
    unsafe fn finish_instantiation(
        &self,
        handle: &InstanceHandle,
    ) -> Result<(), InstantiationError>;
}

// Implementation of `Upcastable` taken from https://users.rust-lang.org/t/why-does-downcasting-not-work-for-subtraits/33286/7 .
/// Trait needed to get downcasting of `Engine`s to work.
pub trait Upcastable {
    fn upcast_any_ref(&'_ self) -> &'_ dyn Any;
    fn upcast_any_mut(&'_ mut self) -> &'_ mut dyn Any;
    fn upcast_any_box(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any + Send + Sync + 'static> Upcastable for T {
    #[inline]
    fn upcast_any_ref(&'_ self) -> &'_ dyn Any {
        self
    }
    #[inline]
    fn upcast_any_mut(&'_ mut self) -> &'_ mut dyn Any {
        self
    }
    #[inline]
    fn upcast_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl dyn Artifact + 'static {
    /// Try to downcast the artifact into a given type.
    #[inline]
    pub fn downcast_ref<T: 'static>(&'_ self) -> Option<&'_ T> {
        self.upcast_any_ref().downcast_ref::<T>()
    }

    /// Try to downcast the artifact into a given type mutably.
    #[inline]
    pub fn downcast_mut<T: 'static>(&'_ mut self) -> Option<&'_ mut T> {
        self.upcast_any_mut().downcast_mut::<T>()
    }
}
