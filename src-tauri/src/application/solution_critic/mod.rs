mod generator;
mod service;
mod support;
mod types;

pub use generator::{
    AgentSolutionCritiqueGenerator, DeterministicSolutionCritiqueGenerator,
    SolutionCritiqueGenerator,
};
pub use service::SolutionCritiqueService;
pub use types::{
    CompileContextRequest, CompileContextResult, CompiledContextCandidate,
    CompiledContextReadResult, CritiqueArtifactRequest, CritiqueArtifactResult, EvidenceRef,
    RawContextBundle, SolutionCritiqueCandidate, SolutionCritiqueReadResult, SourceLimits,
};
