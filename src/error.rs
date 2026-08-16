use thiserror::Error;

/// Errors that can be returned by the excel2pdf library.
#[derive(Debug, Error)]
pub enum Excel2PdfError {
    /// A PDF operation failed.
    #[error("PDF error: {0}")]
    Pdf(String),

    /// LibreOffice is not installed or could not be found.
    #[error("LibreOffice is not installed or could not be found")]
    LibreOfficeNotInstalled,

    /// Neither LibreOffice nor Microsoft Excel is available (Windows only).
    #[error("neither Microsoft Excel nor LibreOffice is installed on this system")]
    ConverterNotFound,

    /// The Excel-to-PDF conversion process failed.
    #[error("conversion failed: {0}")]
    ConversionFailed(String),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A merge/combine operation is already in progress.
    #[error("a process is currently running using the resource")]
    AlreadyProcessing,

    /// Invalid input was provided.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
