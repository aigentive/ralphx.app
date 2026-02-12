// Memory archive job entity - re-exports from canonical memory_archive module
//
// This module provides backward-compatible type aliases for code that was
// written against an earlier version of the memory archive types.
// The canonical types live in memory_archive.rs.

pub use super::memory_archive::{
    ArchiveJobStatus as MemoryArchiveJobStatus,
    ArchiveJobType as MemoryArchiveJobType,
};
