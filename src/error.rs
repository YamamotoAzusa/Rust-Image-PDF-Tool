use crate::domain::image_data_list::ImageValidationError;
use crate::domain::input_source::path_error::PathError;
use crate::domain::pdf_file::create_pdf::PdfError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/Oエラーが発生しました")]
    Io(#[from] std::io::Error),

    #[error("パス関連のエラー")]
    Path(#[from] PathError),

    #[error("画像検証エラー")]
    ImageValidation(#[from] ImageValidationError),

    #[error("PDF生成エラー")]
    Pdf(#[from] PdfError),

    #[error("処理対象が見つかりませんでした: {0}")]
    NoItemsProcessed(String),
}
