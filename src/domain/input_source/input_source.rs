use super::directory_path::DirectoryPath;
use super::zip_file_path::ZipFilePath;

/// 入力ソースを表現する列挙型。
/// ディレクトリパスまたはZIPファイルパスのいずれかを保持する。
#[derive(Debug)]
enum InputSource {
    Directory(DirectoryPath),
    ZipFile(ZipFilePath),
}
