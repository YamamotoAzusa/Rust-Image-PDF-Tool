use std::{fmt, path::Path};
// エラー型を定義
#[derive(Debug)]
pub enum PathError {
    InvalidPath(String),
    IoError(std::io::Error),
    UnsupportedType(std::path::PathBuf),
    NotFound(std::path::PathBuf),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::InvalidPath(s) => write!(f, "無効なパスです: {}", s),
            PathError::IoError(e) => write!(f, "I/Oエラー: {}", e),
            PathError::UnsupportedType(p) => {
                write!(f, "サポートされていないパスの種類です: {}", p.display())
            }
            PathError::NotFound(p) => write!(f, "パスが存在しません: {}", p.display()),
        }
    }
}

impl std::error::Error for PathError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PathError::IoError(e) => Some(e),
            _ => None,
        }
    }
}
