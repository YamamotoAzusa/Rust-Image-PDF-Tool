use super::path_error::PathError;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// 構造体としてDirectoryPathを定義
#[derive(Debug)]
pub struct DirectoryPath {
    pub path: PathBuf,
}

impl DirectoryPath {
    // コンストラクタ: パスを受け取り、バリデーションを行う
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PathError> {
        let path = path.as_ref();

        // パスが存在し、かつディレクトリであることを検証
        if !path.exists() {
            return Err(PathError::InvalidPath(format!(
                "パス '{}' は存在しません。",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(PathError::InvalidPath(format!(
                "パス '{}' はディレクトリではありません。",
                path.display()
            )));
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    // 内部のPathBufへの参照を返す
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    // ディレクトリが空かどうかをチェックするメソッド
    pub fn is_empty(&self) -> Result<bool, PathError> {
        // fs::read_dirがResultを返すため、`?`演算子でエラーを伝播させる
        let mut entries = fs::read_dir(&self.path).map_err(|e| PathError::IoError(e))?;

        // entries.next()は、イテレータが空かどうかをチェックする
        Ok(entries.next().is_none())
    }

    // ディレクトリ内のすべてのエントリをイテレータとして取得
    pub fn entries(&self) -> Result<fs::ReadDir, PathError> {
        fs::read_dir(&self.path).map_err(|e| PathError::IoError(e))
    }
}

// Displayトレイトの実装（表示用）
impl fmt::Display for DirectoryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

#[cfg(test)]
mod tests {
    // 外部クレートや親モジュールをuse
    use super::*;
    use std::io::ErrorKind;
    use tempfile::tempdir;
    /// 正常なディレクトリパスでDirectoryPathが作成できるかテスト
    #[test]
    fn test_valid_directory_path() {
        // 一時的なディレクトリを作成
        let dir = tempdir().expect("Failed to create temp directory");
        let path = dir.path();

        let result = DirectoryPath::new(path);

        // 結果がOKであることを確認
        assert!(result.is_ok());

        // 内部のパスが一致するか検証
        let dir_path_instance = result.unwrap();
        assert_eq!(dir_path_instance.as_path(), path);
    }

    /// 存在しないパスでエラーが返されるかテスト
    #[test]
    fn test_non_existent_path_returns_error() {
        let path = PathBuf::from("this_directory_should_not_exist");
        let result = DirectoryPath::new(&path);

        // 結果がErrであることを確認
        assert!(result.is_err());

        // エラーの種類がPathError::InvalidPathであることを検証
        let err = result.unwrap_err();
        if let PathError::InvalidPath(msg) = err {
            assert!(msg.contains("存在しません"));
        } else {
            panic!("予期せぬエラーが返されました: {:?}", err);
        }
    }

    /// ファイルパスでエラーが返されるかテスト
    #[test]
    fn test_file_path_returns_error() {
        let file_path = PathBuf::from("Cargo.toml"); // 常に存在するファイル
        let result = DirectoryPath::new(&file_path);

        // 結果がErrであることを確認
        assert!(result.is_err());

        // エラーの種類がPathError::InvalidPathであることを検証
        let err = result.unwrap_err();
        if let PathError::InvalidPath(msg) = err {
            assert!(msg.contains("ディレクトリではありません"));
        } else {
            panic!("予期せぬエラーが返されました: {:?}", err);
        }
    }

    /// is_empty()メソッドが正しく機能するかテスト
    #[test]
    fn test_is_empty_method() {
        // (1) 空のディレクトリ
        let empty_dir = tempdir().expect("Failed to create empty temp directory");
        let empty_path = DirectoryPath::new(empty_dir.path()).unwrap();

        let is_empty = empty_path.is_empty().expect("is_empty should not fail");
        assert!(is_empty, "空のディレクトリはis_empty()でtrueを返すはずです");

        // (2) 空ではないディレクトリ
        let non_empty_dir = tempdir().expect("Failed to create non-empty temp directory");
        fs::write(non_empty_dir.path().join("test_file.txt"), "hello world")
            .expect("Failed to create file");
        let non_empty_path = DirectoryPath::new(non_empty_dir.path()).unwrap();

        let is_empty = non_empty_path.is_empty().expect("is_empty should not fail");
        assert!(
            !is_empty,
            "空ではないディレクトリはis_empty()でfalseを返すはずです"
        );
    }

    /// is_empty()がI/Oエラーを正しく返すかテスト
    #[test]
    fn test_is_empty_returns_io_error() {
        // 存在しないパスを作成し、is_emptyを呼び出す（newで検証済みのため、このパス自体はDirectoryPath型ではない）
        let non_existent_path = PathBuf::from("another_non_existent_path");
        // 強制的にDirectoryPathのインスタンスを作成（このテストのためにnewの検証をスキップ）
        let dir_path = DirectoryPath {
            path: non_existent_path,
        };

        let result = dir_path.is_empty();

        // 結果がErrであることを確認
        assert!(result.is_err());

        // エラーがPathError::IoErrorであり、かつio::ErrorKind::NotFoundを持つことを検証
        let err = result.unwrap_err();
        if let PathError::IoError(e) = err {
            assert_eq!(e.kind(), ErrorKind::NotFound);
        } else {
            panic!("予期せぬエラーが返されました: {:?}", err);
        }
    }
    /// entries()メソッドが正しく機能するかテスト
    #[test]
    fn test_entries_method() {
        // --- (1) 複数のエントリを持つディレクトリのテスト ---
        let dir = tempdir().expect("Failed to create temp directory");
        let path = dir.path();

        // テスト用のファイルとサブディレクトリを作成
        fs::write(path.join("file1.txt"), "hello").expect("Failed to create file1");
        fs::write(path.join("file2.txt"), "world").expect("Failed to create file2");
        fs::create_dir(path.join("subdir")).expect("Failed to create subdir");

        let dir_path = DirectoryPath::new(path).unwrap();
        let entries_result = dir_path.entries();

        // entries()が成功することを確認
        assert!(entries_result.is_ok());

        // 返されたイテレータからエントリ名を取得し、ソートして比較
        let mut entry_names: Vec<String> = entries_result
            .unwrap()
            .map(|res| res.map(|e| e.file_name().into_string().unwrap()).unwrap())
            .collect();
        entry_names.sort(); // 読み取り順序は保証されないためソートする

        assert_eq!(entry_names, vec!["file1.txt", "file2.txt", "subdir"]);

        // --- (2) 空のディレクトリのテスト ---
        let empty_dir = tempdir().expect("Failed to create empty directory");
        let empty_path = DirectoryPath::new(empty_dir.path()).unwrap();
        let mut entries = empty_path
            .entries()
            .expect("entries should not fail for empty dir");

        // イテレータが空であることを確認
        assert!(entries.next().is_none());
    }

    /// entries()がI/Oエラーを正しく返すかテスト
    #[test]
    fn test_entries_returns_io_error() {
        // new()のバリデーションをスキップして、存在しないパスを持つインスタンスを強制的に作成
        let non_existent_path = PathBuf::from("this_path_definitely_does_not_exist");
        let dir_path = DirectoryPath {
            path: non_existent_path,
        };

        let result = dir_path.entries();

        // 結果がErrであることを確認
        assert!(result.is_err());

        // エラーの種類がPathError::IoErrorであり、その原因がErrorKind::NotFoundであることを確認
        let err = result.unwrap_err();
        if let PathError::IoError(e) = err {
            assert_eq!(e.kind(), ErrorKind::NotFound);
        } else {
            panic!("予期せぬエラーが返されました: {:?}", err);
        }
    }
}
