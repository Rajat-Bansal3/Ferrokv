use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("error initailising listner")]
    ErrorInitialisingLister,
    #[error("error binding listner")]
    ErrorBindingLister,
    #[error("max connection limit reached")]
    MaxConnections,
    #[error("error accepting connection onto listner")]
    ErrorAcceptingConnections,
}
