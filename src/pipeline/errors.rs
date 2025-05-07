use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("non supported type")]
    NonSupportedType,
    #[error("invalid port direction")]
    InvalidPortDirection,
    #[error("invalid data type")]
    InvalidDataType,
}

#[derive(Error, Debug)]
pub enum TopologyError {
    #[error("topology source (entrypoint) is not defined")]
    UndefinedSourceEntryPoint,
    #[error("more than one topology source (entrypoint)")]
    SourceIsNotUnique,
    #[error("parent name is not defined")]
    UndefinedParentName,
    #[error("routing non feasible: unknown parent \"{0}\"")]
    ParentDoesNotExist(String),
    #[error("routing non feasible: node already exists \"{0}\"")]
    NodeAlreadyExists(String),
    #[error("internal error: routing should have been feasible")]
    InternalRoutingError,
}