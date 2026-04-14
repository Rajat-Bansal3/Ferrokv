#[derive(Debug, thiserror::Error)]
pub enum AOFError {
    #[error("error initialising AOFWriter")]
    FailedToInitialsiseWriter,
    #[error("file doesnt exisits or have required perms")]
    ErrOpeningFile,
    #[error("error wrting to file. could be a permission issue")]
    ErrorWrittingToFile,
    #[error("error flushing the buffer to Page")]
    ErrorFlushingBuffer,
}
