// Domain services - business logic that doesn't fit in entities
//
// Services coordinate repositories and entities to implement
// use cases and business rules.

pub mod workflow_service;

pub use workflow_service::{
    AppliedColumn, AppliedWorkflow, ColumnMappingError, ValidationResult, WorkflowService,
};
