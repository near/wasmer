//! Engine trait and associated types.

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use wasmer_compiler::{CompileError, Target};
use wasmer_types::{FunctionType, FunctionTypeRef};
use wasmer_vm::{VMCallerCheckedAnyfunc, VMFuncRef, VMSharedSignatureIndex};

mod private {
    pub struct Internal(pub(super) ());
}

/// A unimplemented Wasmer `Engine`.
///
/// This trait is used by implementors to implement custom engines
/// such as: Universal or Native.
///
/// The product that an `Engine` produces and consumes is the [`Artifact`].
pub trait Engine {
    /// Gets the target
    fn target(&self) -> &Target;

    /// Register a signature
    fn register_signature(&self, func_type: FunctionTypeRef<'_>) -> VMSharedSignatureIndex;

    /// Register a function's data.
    fn register_function_metadata(&self, func_data: VMCallerCheckedAnyfunc) -> VMFuncRef;

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType>;

    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError>;

    /// A unique identifier for this object.
    ///
    /// This exists to allow us to compare two Engines for equality. Otherwise,
    /// comparing two trait objects unsafely relies on implementation details
    /// of trait representation.
    fn id(&self) -> &EngineId;

    /// Clone the engine
    fn cloned(&self) -> Arc<dyn Engine + Send + Sync>;

    /// Internal: support for downcasting `Engine`s.
    #[doc(hidden)]
    fn type_id(&self, _: private::Internal) -> std::any::TypeId
    where
        Self: 'static,
    {
        std::any::TypeId::of::<Self>()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// A unique identifier for an Engine.
pub struct EngineId {
    id: usize,
}

impl EngineId {
    /// Format this identifier as a string.
    pub fn id(&self) -> String {
        format!("{}", &self.id)
    }
}

impl Clone for EngineId {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Default for EngineId {
    fn default() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, SeqCst),
        }
    }
}

impl dyn Engine {
    /// Downcast a dynamic Executable object to a concrete implementation of the trait.
    pub fn downcast_ref<T: Engine + 'static>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == self.type_id(private::Internal(())) {
            unsafe { Some(&*(self as *const dyn Engine as *const T)) }
        } else {
            None
        }
    }

    /// Downcast a dynamic Executable object to a concrete implementation of the trait.
    pub fn downcast_arc<T: Engine + 'static>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>> {
        if std::any::TypeId::of::<T>() == Engine::type_id(&*self, private::Internal(())) {
            // SAFETY: err, its probably sound, we effectively construct a transmute here.
            unsafe {
                let ptr = Arc::into_raw(self).cast::<T>();
                Ok(Arc::from_raw(ptr))
            }
        } else {
            Err(self)
        }
    }
}

impl dyn Engine + Send + Sync {
    /// Downcast a dynamic Executable object to a concrete implementation of the trait.
    pub fn downcast_ref<T: Engine + 'static>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == self.type_id(private::Internal(())) {
            unsafe { Some(&*(self as *const dyn Engine as *const T)) }
        } else {
            None
        }
    }

    /// Downcast a dynamic Executable object to a concrete implementation of the trait.
    pub fn downcast_arc<T: Engine + 'static>(self: Arc<Self>) -> Result<Arc<T>, Arc<Self>> {
        if std::any::TypeId::of::<T>() == Engine::type_id(&*self, private::Internal(())) {
            // SAFETY: err, its probably sound, we effectively construct a transmute here.
            unsafe {
                let ptr = Arc::into_raw(self).cast::<T>();
                Ok(Arc::from_raw(ptr))
            }
        } else {
            Err(self)
        }
    }
}
