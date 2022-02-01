// This file contains code from external sources.
// Attributions: https://github.com/wasmerio/wasmer/blob/master/ATTRIBUTIONS.md

//! Implement a registry of function signatures, for fast indirect call
//! signature checking.

use loupe::MemoryUsage;
use std::collections::{hash_map, HashMap};
use std::convert::TryFrom;
use wasmer_types::{FunctionType, FunctionTypeRef};

/// An index into the shared signature registry, usable for checking signatures
/// at indirect calls.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash, MemoryUsage)]
pub struct VMSharedSignatureIndex(u32);

impl VMSharedSignatureIndex {
    /// Create a new `VMSharedSignatureIndex`.
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

/// WebAssembly requires that the caller and callee signatures in an indirect
/// call must match. To implement this efficiently, keep a registry of all
/// signatures, shared by all instances, so that call sites can just do an
/// index comparison.
#[derive(Debug, MemoryUsage)]
pub struct SignatureRegistry {
    signature2index: HashMap<FunctionType, VMSharedSignatureIndex>,
    index2signature: Vec<FunctionType>,
}

impl SignatureRegistry {
    /// Create a new `SignatureRegistry`.
    pub fn new() -> Self {
        Self {
            signature2index: HashMap::new(),
            index2signature: Vec::new(),
        }
    }

    /// Register a signature and return its unique index.
    pub fn register(&mut self, sig: FunctionTypeRef<'_>) -> VMSharedSignatureIndex {
        let len = self.index2signature.len();
        // TODO(0-copy): this. should. not. allocate.
        //
        // This is pretty hard to avoid, however. In order to implement bijective map, we'd want
        // a `Rc<FunctionType>`, but indexing into a map keyed by `Rc<FunctionType>` with
        // `FunctionTypeRef` is… not possible given the current API either.
        //
        // Consider `transmute` or `hashbrown`'s raw_entry.
        let sig = FunctionType::new(sig.params(), sig.results());
        match self.signature2index.entry(sig.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => {
                debug_assert!(
                    u32::try_from(len).is_ok(),
                    "invariant: can't have more than 2³²-1 signatures!"
                );
                let sig_id = VMSharedSignatureIndex::new(u32::try_from(len).unwrap());
                entry.insert(sig_id);
                self.index2signature.push(sig);
                sig_id
            }
        }
    }

    /// Looks up a shared signature index within this registry.
    ///
    /// Note that for this operation to be semantically correct the `idx` must
    /// have previously come from a call to `register` of this same object.
    pub fn lookup(&self, idx: VMSharedSignatureIndex) -> Option<&FunctionType> {
        self.index2signature.get(idx.0 as usize)
    }
}
