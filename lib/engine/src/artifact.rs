use crate::{InstanceHandle, InstantiationError, Resolver, Tunables};
use loupe::MemoryUsage;
use std::any::Any;
use wasmer_types::InstanceConfig;

/// A predecesor of a full module Instance.
///
/// This type represents parts of a compiled WASM module ([`Executable`](crate::Executable)) that
/// are pre-allocated in within some Engine's store.
///
/// Some other operations such as linking, relocating and similar may also be performed during
/// constructon of the Artifact, making this type particularly well suited for caching in-memory.
pub trait Artifact: Send + Sync + MemoryUsage {
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
}
