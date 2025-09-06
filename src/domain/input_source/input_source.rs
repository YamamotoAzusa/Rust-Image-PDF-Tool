use super::directory_path::DirectoryPath;
use super::zip_file_path::ZipFilePath;

enum InputSource {
    Directory(DirectoryPath),
    ZipFile(ZipFilePath),
}
