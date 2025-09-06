use super::path_error::PathError;
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// ZIPファイルへのパスを表現し、その妥当性を保証する構造体。
#[derive(Debug)]
pub struct ZipFilePath(PathBuf);

impl ZipFilePath {
    // --- Public Methods ---

    /// 新しい `ZipFilePath` インスタンスを生成する。
    ///
    /// パスが存在し、ファイルであり、かつ拡張子が `.zip` であることを検証する。
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PathError> {
        let path = path.as_ref();

        // 存在し、かつファイルであることを検証
        if !path.exists() {
            return Err(PathError::InvalidPath(format!(
                "パス '{}' は存在しません。",
                path.display()
            )));
        }
        if !path.is_file() {
            return Err(PathError::InvalidPath(format!(
                "パス '{}' はファイルではありません。",
                path.display()
            )));
        }

        // 拡張子が.zipであることを検証
        if path.extension().and_then(|s| s.to_str()) != Some("zip") {
            return Err(PathError::InvalidPath(format!(
                "パス '{}' は.zipファイルではありません。",
                path.display()
            )));
        }
        Ok(Self(path.to_path_buf()))
    }

    /// 内部の `Path` への参照を返す。
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// 指定したエントリの中身をバイト列で返す。
    pub fn read_entry(&self, name: &str) -> Result<Vec<u8>, PathError> {
        // ヘルパーメソッドを順番に呼び出すことで、処理の流れを明確にする
        let mut archive = self.open_archive()?;
        let mut entry = Self::find_entry_in_archive(&mut archive, name)?;
        Self::read_entry_content(&mut entry)
    }

    // --- Private Helper Methods ---

    /// ZIPファイルを開き、ZipArchiveを生成する。
    fn open_archive(&self) -> Result<ZipArchive<std::fs::File>, PathError> {
        let file = std::fs::File::open(&self.0)
            .map_err(|e| PathError::InvalidPath(format!("ZIPファイルを開けません: {}", e)))?;
        ZipArchive::new(file)
            .map_err(|e| PathError::InvalidPath(format!("無効なZIPファイル: {}", e)))
    }

    /// アーカイブから指定されたエントリを見つける。
    ///
    /// `self`に依存しないため、関連関数(associated function)として定義。
    fn find_entry_in_archive<'a>(
        archive: &'a mut ZipArchive<std::fs::File>,
        name: &str,
    ) -> Result<zip::read::ZipFile<'a>, PathError> {
        archive
            .by_name(name)
            .map_err(|e| PathError::InvalidPath(format!("エントリ '{}' を開けません: {}", name, e)))
    }

    /// エントリの内容を読み込む。
    ///
    /// `self`に依存しないため、関連関数として定義。
    fn read_entry_content(entry: &mut zip::read::ZipFile) -> Result<Vec<u8>, PathError> {
        let mut buffer = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buffer)
            .map_err(|e| PathError::InvalidPath(format!("エントリの読み取りに失敗: {}", e)))?;
        Ok(buffer)
    }
}

// Displayトレイトの実装（表示用）
impl fmt::Display for ZipFilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

// テストモジュール
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::FileOptions;
    use zip::CompressionMethod;

    // 一時ZIPファイルを作成してパスを返す。entries は (name, bytes) のタプル配列。
    fn create_temp_zip(entries: &[(&str, &[u8])]) -> PathBuf {
        let mut path = std::env::temp_dir();
        // ファイル名が一意になるようにナノ秒単位のタイムスタンプを使用
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("test_zip_{}.zip", ts));

        let file = File::create(&path).expect("failed to create temp zip file");
        let mut zip = zip::ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);

        for (name, bytes) in entries {
            zip.start_file(*name, options).expect("start_file failed");
            zip.write_all(bytes).expect("write_all failed");
        }
        zip.finish().expect("finish zip failed");
        path
    }

    #[test]
    fn test_new_and_read_entry_ok() {
        let zip_path = create_temp_zip(&[("hello.txt", b"hello world")]);
        let zfp = ZipFilePath::new(&zip_path).expect("ZipFilePath::new should succeed");
        let content = zfp
            .read_entry("hello.txt")
            .expect("read_entry should succeed");
        assert_eq!(content, b"hello world");
        // cleanup
        let _ = fs::remove_file(zip_path);
    }

    #[test]
    fn test_new_invalid_extension() {
        let mut path = std::env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("not_a_zip_{}.txt", ts));

        // 空ファイルを作成
        File::create(&path).expect("create file");

        let res = ZipFilePath::new(&path);
        assert!(res.is_err());
        if let Err(PathError::InvalidPath(msg)) = res {
            assert!(msg.contains("は.zipファイルではありません。"));
        } else {
            panic!("Expected InvalidPath error for wrong extension");
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_new_nonexistent() {
        let path = PathBuf::from("nonexistent_file_for_test.zip");
        let res = ZipFilePath::new(&path);
        assert!(res.is_err());
        if let Err(PathError::InvalidPath(msg)) = res {
            assert!(msg.contains("は存在しません。"));
        } else {
            panic!("Expected InvalidPath error for nonexistent file");
        }
    }

    #[test]
    fn test_read_missing_entry() {
        let zip_path = create_temp_zip(&[("a.txt", b"a")]);
        let zfp = ZipFilePath::new(&zip_path).expect("ZipFilePath::new should succeed");
        let res = zfp.read_entry("missing.txt");
        assert!(res.is_err());
        if let Err(PathError::InvalidPath(msg)) = res {
            assert!(msg.contains("エントリ 'missing.txt' を開けません"));
        } else {
            panic!("Expected InvalidPath error for missing entry");
        }

        let _ = fs::remove_file(zip_path);
    }

    // 新しいテスト: as_path が元の Path を返すことを確認
    #[test]
    fn test_as_path_returns_original_path() {
        let zip_path = create_temp_zip(&[("file.txt", b"content")]);
        let zfp = ZipFilePath::new(&zip_path).expect("ZipFilePath::new should succeed");
        // Path の等価性を検証
        assert_eq!(zfp.as_path(), zip_path.as_path());
        // 表示文字列も一致することを追加で確認（任意）
        assert_eq!(zfp.to_string(), zip_path.display().to_string());
        let _ = fs::remove_file(zip_path);
    }
}
