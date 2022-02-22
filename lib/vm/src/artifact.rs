use crate::{InstanceHandle, Resolver, Tunables, VMLocalFunction, VMSharedSignatureIndex};
use loupe::MemoryUsage;
use std::{any::Any, collections::BTreeMap, sync::Arc};
use wasmer_types::{
    entity::BoxedSlice, ElemIndex, EntityCounts, FunctionIndex, GlobalInit, GlobalType,
    InstanceConfig, LocalFunctionIndex, OwnedDataInitializer, OwnedTableInitializer,
};

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
        self: Arc<Self>,
        tunables: &dyn Tunables,
        resolver: &dyn Resolver,
        host_state: Box<dyn Any>,
        config: InstanceConfig,
    ) -> Result<InstanceHandle, Box<dyn std::error::Error + Send + Sync>>;

    /// The information about offsets into the VM context table.
    fn offsets(&self) -> &crate::VMOffsets;

    /// The count of imported entities.
    fn import_counts(&self) -> &EntityCounts;

    /// The locally defined functions.
    ///
    /// These are published and ready to call.
    fn functions(&self) -> &BoxedSlice<LocalFunctionIndex, VMLocalFunction>;

    /// Passive table elements.
    fn passive_elements(&self) -> &BTreeMap<ElemIndex, Box<[FunctionIndex]>>;

    /// Table initializers.
    fn element_segments(&self) -> &[OwnedTableInitializer];

    /// Memory initializers.
    /// TODO: consider making it an iterator of `DataInitializer`s instead?
    fn data_segments(&self) -> &[OwnedDataInitializer];

    /// Passive table elements.
    fn globals(&self) -> &[(GlobalType, GlobalInit)];

    /// The function index to the start function.
    fn start_function(&self) -> Option<FunctionIndex>;

    /// Function by export name.
    fn function_by_export_field(&self, name: &str) -> Option<FunctionIndex>;

    /// Mapping between module SignatureIndex and VMSharedSignatureIndex.
    fn signatures(&self) -> &[VMSharedSignatureIndex];

    /// Obtain the function signature for either the import or local definition.
    fn function_signature(&self, index: FunctionIndex) -> Option<VMSharedSignatureIndex>;
}
