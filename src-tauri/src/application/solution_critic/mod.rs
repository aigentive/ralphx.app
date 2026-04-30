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
    ApplyProjectedGapActionRequest, CompileContextRequest, CompileContextResult,
    CompiledContextCandidate, CompiledContextHistoryItem, CompiledContextReadResult,
    ContextTargetRequest, CritiqueArtifactRequest, CritiqueArtifactResult, EvidenceRef,
    ProjectedCritiqueGapActionResult, RawContextBundle, SolutionCritiqueCandidate,
    SolutionCritiqueGapActionSummary, SolutionCritiqueHistoryItem, SolutionCritiqueReadResult,
    SolutionCritiqueSessionRollup, SolutionCritiqueTargetRollupItem, SourceLimits,
};
