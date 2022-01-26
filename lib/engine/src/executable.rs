use enumset::EnumSet;
use wasmer_types::entity::PrimaryMap;

mod private {
    pub struct Internal(pub(super) ());
}

/// A WASM module built by some [`Engine`](crate::Engine).
///
/// Types implementing this trait are ready to be saved (to e.g. disk) for later use or loaded with
/// the `Engine` to in order to produce an [`Artifact`](crate::Artifact).
pub trait Executable {
    /// The features with which this `Executable` was built.
    fn features(&self) -> &wasmer_compiler::Features;

    /// The CPU features this `Executable` requires.
    fn cpu_features(&self) -> EnumSet<wasmer_compiler::CpuFeature>;

    /// The memory styles associated with this `Executable`.
    fn memory_styles(&self) -> &PrimaryMap<wasmer_types::MemoryIndex, wasmer_vm::MemoryStyle>;

    /// The table plans associated with this `Executable`.
    fn table_styles(&self) -> &PrimaryMap<wasmer_types::TableIndex, wasmer_vm::TableStyle>;

    /// Data initializers used during module instantiation.
    fn data_initializers(&self) -> &[wasmer_types::OwnedDataInitializer];

    /// Serializes an artifact into bytes
    fn serialize(
        &self,
        out: &mut dyn std::io::Write,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Internal: support for downcasting `Executable`s.
    #[doc(hidden)]
    fn type_id(&self, _: private::Internal) -> std::any::TypeId
    where
        Self: 'static,
    {
        std::any::TypeId::of::<Self>()
    }
}

impl dyn Executable {
    /// Downcast a dynamic Executable object to a concrete implementation of the trait.
    pub fn downcast_ref<T: Executable + 'static>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == self.type_id(private::Internal(())) {
            unsafe { Some(&*(self as *const dyn Executable as *const T)) }
        } else {
            None
        }
    }
}
