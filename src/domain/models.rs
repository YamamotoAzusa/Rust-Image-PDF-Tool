#[derive(Debug)]
enum ConversionError {
    InputPathNotFound(PathBuf),
    InputNotADirectoryOrZip,
    NoImagesFoundInSource,
    PdfGenerationFailed(String),
    FileReadFailed(String),
    ZipArchiveError(String),
}

// 入力の種類を型で表現する
// ディレクトリ or ZIPファイル
enum InputSource {
    Directory(PathBuf),
    ZipFile(PathBuf),
}

// 中間生成物である「画像データ」を型で表現する
// メモリ上の画像データのリスト (ファイル名, バイナリデータ)
struct InMemoryImages(Vec<(String, Vec<u8>)>);

// 最終生成物である「PDFデータ」を型で表現する
struct PdfDocument {
    filename_stem: String,
    data: Vec<u8>,
}
