mod generator;
mod review_preparer;
mod service;
mod support;
mod types;

pub use generator::{
    AgentSolutionCritiqueGenerator, DeterministicSolutionCritiqueGenerator,
    SolutionCritiqueGenerator,
};
pub use service::SolutionCritiqueService;
pub use review_preparer::SolutionCritiqueReviewPreparer;
pub use types::{
    CompileContextRequest, CompileContextResult, CompiledContextCandidate,
    CompiledContextReadResult, ContextTargetRequest, CritiqueArtifactRequest,
    CritiqueArtifactResult, EvidenceRef, RawContextBundle, SolutionCritiqueCandidate,
    SolutionCritiqueReadResult, SourceLimits,
};
