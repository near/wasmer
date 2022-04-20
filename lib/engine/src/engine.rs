//! Engine trait and associated types.

use crate::tunables::Tunables;

use loupe::MemoryUsage;

use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use wasmer_compiler::{CompileError, Target};
use wasmer_types::FunctionType;
use wasmer_vm::{VMCallerCheckedAnyfunc, VMFuncRef, VMSharedSignatureIndex};

/// A unimplemented Wasmer `Engine`.
///
/// This trait is used by implementors to implement custom engines
/// such as: Universal or Native.
///
/// The product that an `Engine` produces and consumes is the [`Artifact`].
pub trait Engine: MemoryUsage {
    /// Gets the target
    fn target(&self) -> &Target;

    /// Register a signature
    fn register_signature(&self, func_type: &FunctionType) -> VMSharedSignatureIndex;

    /// Register a function's data.
    fn register_function_metadata(&self, func_data: VMCallerCheckedAnyfunc) -> VMFuncRef;

    /// Lookup a signature
    fn lookup_signature(&self, sig: VMSharedSignatureIndex) -> Option<FunctionType>;

    /// Validates a WebAssembly module
    fn validate(&self, binary: &[u8]) -> Result<(), CompileError>;

    /// Compile a WebAssembly binary
    fn compile(
        &self,
        binary: &[u8],
        tunables: &dyn Tunables,
    ) -> Result<Box<dyn crate::Executable>, CompileError>;

    /// Load a compiled executable with this engine.
    fn load(
        &self,
        executable: &(dyn crate::Executable + 'static),
    ) -> Result<Arc<dyn crate::Artifact>, CompileError>;

    /// A unique identifier for this object.
    ///
    /// This exists to allow us to compare two Engines for equality. Otherwise,
    /// comparing two trait objects unsafely relies on implementation details
    /// of trait representation.
    fn id(&self) -> &EngineId;

    /// Clone the engine
    fn cloned(&self) -> Arc<dyn Engine + Send + Sync>;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, MemoryUsage)]
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
