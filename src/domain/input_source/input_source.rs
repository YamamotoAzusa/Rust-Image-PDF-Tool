use super::directory_path::DirectoryPath;
use super::zip_file_path::ZipFilePath;

/// 入力ソースを表現する列挙型。
/// ディレクトリパスまたはZIPファイルパスのいずれかを保持する。
#[derive(Debug)]
pub enum InputSource {
    Directory(DirectoryPath),
    ZipFile(ZipFilePath),
}

use super::path_error::PathError;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

impl InputSource {
    /// 新しい `InputSource` インスタンスを作成します。
    ///
    /// # 引数
    /// * `path` - ディレクトリまたはZIPファイルへのパス。
    ///
    /// # 戻り値
    /// パスが有効なディレクトリまたはZIPファイルである場合は `Ok(InputSource)` を返します。
    /// パスが存在しない、サポートされていない種類である、またはその他の理由で無効な場合は `Err(PathError)` を返します。
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PathError> {
        let path = path.as_ref();

        // メタデータの取得を試みる。これにより、パスの存在チェックを一度で行う。
        // fs::metadata はシンボリックリンクを追跡する。追跡したくない場合は symlink_metadata を使用する。
        let metadata = fs::metadata(path).map_err(|e| {
            // I/OエラーをカスタムのPathErrorに変換する。
            // ここでは、特に `NotFound` エラーを明示的に扱うことが重要。
            if e.kind() == std::io::ErrorKind::NotFound {
                PathError::NotFound(path.to_path_buf())
            } else {
                // その他のI/Oエラーは、より一般的なエラー型としてラップする
                PathError::IoError(e)
            }
        })?;

        // パスがディレクトリの場合
        if metadata.is_dir() {
            // DirectoryPath のコンストラクタを呼び出す。
            // このコンストラクタは、例えばパーミッションなどの追加チェックを行うかもしれない。
            let dir = DirectoryPath::new(path)?;
            return Ok(InputSource::Directory(dir));
        }

        // パスがファイルの場合
        if metadata.is_file() {
            // ZipFilePath のコンストラクタを呼び出す。
            // このコンストラクタは、拡張子が .zip であるかなどのチェックを行うと想定される。
            let zip = ZipFilePath::new(path)?;
            return Ok(InputSource::ZipFile(zip));
        }

        // パスがディレクトリでもファイルでもない場合 (例: シンボリックリンク、FIFOなど)
        // サポートされていないパスの種類としてエラーを返す。
        Err(PathError::UnsupportedType(path.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // tempfileクレートは、テスト中に一時的なファイルやディレクトリを作成するのに非常に便利
    use std::fs::File;
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    // --- 正常系のテスト ---

    /// 正常なディレクトリパスで InputSource::Directory が作成できることをテスト
    #[test]
    fn test_new_with_valid_directory() {
        // 一時的なディレクトリを作成
        let dir = tempdir().expect("Failed to create temp directory");
        let path = dir.path();

        // InputSource::new を呼び出す
        let result = InputSource::new(path);

        // 結果がOkであることを表明(assert)
        assert!(result.is_ok());

        // Okの中身がInputSource::Directoryであることを表明
        // matches! マクロは、enumのバリアントをチェックするのに便利
        assert!(matches!(result.unwrap(), InputSource::Directory(_)));
    }

    /// 正常な（空の）ZIPファイルパスで InputSource::ZipFile が作成できることをテスト
    #[test]
    fn test_new_with_valid_zip_file() {
        // 一時的なファイルを作成（拡張子を.zipにする）
        // ZipFilePath::new が拡張子をチェックすると仮定
        let mut file = NamedTempFile::new().unwrap();
        file.as_file_mut()
            .write_all(b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0")
            .unwrap(); // ダミーのZIPヘッダー
        let path = file.path().with_extension("zip");
        fs::rename(file.path(), &path).unwrap();

        // InputSource::new を呼び出す
        let result = InputSource::new(&path);

        // 結果がOkであり、中身がInputSource::ZipFileであることを表明
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), InputSource::ZipFile(_)));
    }

    // --- 異常系のテスト ---

    /// 存在しないパスを与えた場合に NotFound エラーが返ることをテスト
    #[test]
    fn test_new_with_non_existent_path() {
        // 存在しないことが確実なパスを作成
        let path = PathBuf::from("/path/that/definitely/does/not/exist");

        let result = InputSource::new(&path);

        // 結果がErrであり、そのエラーが PathError::NotFound であることを表明
        assert!(result.is_err());
        // is_err()でチェック済みなので、ここでは unwrap_err() を安全に使える
        assert!(matches!(result.unwrap_err(), PathError::NotFound(_)));
    }

    /// ZIPファイルではない通常のファイルを与えた場合にエラーが返ることをテスト
    /// (ZipFilePath::new が拡張子などでチェックしていると仮定)
    #[test]
    fn test_new_with_regular_file_not_zip() {
        // 一時的なテキストファイルを作成
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "this is not a zip file").unwrap();

        let result = InputSource::new(file.path());

        // 結果がErrであることを表明
        // エラーの具体的な種類は ZipFilePath の実装に依存する
        assert!(result.is_err());
    }
}
