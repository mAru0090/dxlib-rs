use thiserror::Error;

#[derive(Debug, Error)]
pub enum DxLibError {
    #[error("Failed to DxLib_Init()")]
    InitializeError,
    #[error("Failed to DxLib_End()")]
    FinalizeError,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
