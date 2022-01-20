use crate::{InstantiationError, RuntimeError, SerializeError};
use enumset::EnumSet;
use loupe::MemoryUsage;
use std::any::Any;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use wasmer_compiler::{CpuFeature, Features};
use wasmer_types::entity::PrimaryMap;
use wasmer_types::{DataInitializer, MemoryIndex, ModuleInfo, OwnedDataInitializer, TableIndex};
use wasmer_vm::{InstanceHandle, MemoryStyle, TableStyle};

/// An `Artifact` is the product of compiling a WASM module with an `Engine`. Conceptually this can
/// be thought of as an object file or shared library that is produced as a result of a traditional
/// ahead-of-time compilation flow.
///
/// The `Artifact` contains the compiled data for a given module as well as extra information
/// needed to instantiate (load) the module and prepare the code therein to run.
pub trait Artifact: Send + Sync + Upcastable + MemoryUsage {
    /// Return a reference-counted pointer to the module
    fn module(&self) -> Arc<ModuleInfo>;

    /// Return a pointer to a module.
    fn module_ref(&self) -> &ModuleInfo;

    /// Gets a mutable reference to the info.
    ///
    /// Note: this will return `None` if the module is already instantiated.
    fn module_mut(&mut self) -> Option<&mut ModuleInfo>;

    /// Returns the features for this Artifact
    fn features(&self) -> &Features;

    /// Returns the CPU features for this Artifact
    fn cpu_features(&self) -> EnumSet<CpuFeature>;

    /// Returns the memory styles associated with this `Artifact`.
    fn memory_styles(&self) -> &PrimaryMap<MemoryIndex, MemoryStyle>;

    /// Returns the table plans associated with this `Artifact`.
    fn table_styles(&self) -> &PrimaryMap<TableIndex, TableStyle>;

    /// Returns data initializers to pass to `InstanceHandle::initialize`
    fn data_initializers(&self) -> &[OwnedDataInitializer];

    /// Serializes an artifact into bytes
    fn serialize(&self) -> Result<Vec<u8>, SerializeError>;

    /// Serializes an artifact into a file path
    fn serialize_to_file(&self, path: &Path) -> Result<(), SerializeError> {
        let serialized = self.serialize()?;
        fs::write(&path, serialized)?;
        Ok(())
    }

    /// Finishes the instantiation of a just created `InstanceHandle`.
    ///
    /// # Safety
    ///
    /// See [`InstanceHandle::finish_instantiation`].
    unsafe fn finish_instantiation(
        &self,
        handle: &InstanceHandle,
    ) -> Result<(), InstantiationError> {
        let data_initializers = self
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
