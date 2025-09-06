use std::fmt;
// エラー型を定義
#[derive(Debug)]
pub enum PathError {
    InvalidPath(String),
    IoError(std::io::Error),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::InvalidPath(s) => write!(f, "無効なパスです: {}", s),
            PathError::IoError(e) => write!(f, "I/Oエラー: {}", e),
        }
    }
}
