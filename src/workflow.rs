//! アプリケーションのメインワークフローを定義するモジュール。
//!
//! このモジュールは、UI層（`cli`）とドメイン層（`domain`）を仲介し、
//! PDF変換の具体的な処理フローを実装します。

use crate::cli::Args;
use my_rust_gemini_app::domain::image_data_list::ImageDataList;
use my_rust_gemini_app::domain::input_source::directory_path::DirectoryPath;
use my_rust_gemini_app::domain::input_source::input_source::InputSource;
use my_rust_gemini_app::domain::input_source::zip_file_path::ZipFilePath;
use my_rust_gemini_app::domain::pdf_file::create_pdf::PdfFile;
use my_rust_gemini_app::domain::pdf_file::pdf_font::PdfFont;
use my_rust_gemini_app::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};

// --- public な main 関数 ---

/// アプリケーションのメインロジックを実行します。
///
/// # 引数
/// * `args`: コマンドラインからパースされた引数 (`cli::Args`)。
///
/// # 戻り値
/// * `Ok(())`: すべての処理が正常に完了した場合。
/// * `Err(AppError)`: 処理中に回復不可能なエラーが発生した場合。
pub fn run(args: Args) -> Result<(), AppError> {
    // 1. 入力ディレクトリの検証
    // DirectoryPath::new を使うことで、パスが存在し、かつディレクトリであることが保証される。
    let input_dir = DirectoryPath::new(&args.input_dir)?;

    // 2. 出力ディレクトリの決定
    // `args.output_dir` が指定されていればそれを使用し、
    // 指定されていなければ入力ディレクトリ (`args.input_dir`) を出力先とする。
    let output_dir = args
        .output_dir
        .as_deref()
        .unwrap_or_else(|| input_dir.as_path());
    // 出力ディレクトリが存在しない場合は作成する。
    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    // 3. フォントパスの参照を準備
    // `Option<PathBuf>` から `Option<&Path>` へ変換して、後続の関数に渡しやすくする。
    let font_path = args.font_path.as_deref();

    // 4. 入力ディレクトリ内のエントリを走査・処理
    let mut processed_item_count = 0;
    for entry_result in input_dir.entries()? {
        let entry = entry_result?;
        let path = entry.path();

        // `InputSource::new` を使って、パスが処理対象（ディレクトリ or ZIP）か判定する。
        // それ以外（ただのファイル等）の場合は `UnsupportedType` エラーとなり、ループ内で無視される。
        if let Ok(source) = InputSource::new(&path) {
            // 処理対象だったので、対応する処理関数を呼び出す。
            // `match` を使って、`InputSource` の種類に応じた処理を振り分ける。
            let result = match source {
                InputSource::Directory(dir) => {
                    println!("[ディレクトリ処理開始] {}", dir.as_path().display());
                    process_directory(&dir, output_dir, font_path)
                }
                InputSource::ZipFile(zip) => {
                    println!("[ZIP処理開始] {}", zip.as_path().display());
                    process_zip_file(&zip, output_dir, font_path)
                }
            };

            // 各アイテムの処理結果をハンドリングする。
            match result {
                Ok(_) => {
                    // 成功した場合はカウンターを増やす。
                    processed_item_count += 1;
                }
                Err(e) => {
                    // 特定のディレクトリやZIPファイルの処理に失敗しても、プログラム全体は止めずに
                    // エラーメッセージを表示して次のアイテムの処理を続ける。
                    eprintln!(
                        "[警告] '{}' の処理中にエラーが発生しました: {}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }

    // 5. 最終結果の判定
    if processed_item_count == 0 {
        // 1つも処理対象が見つからなかった場合は、その旨をエラーとして報告する。
        Err(AppError::NoItemsProcessed(
            input_dir.as_path().display().to_string(),
        ))
    } else {
        // 1つ以上処理できていれば成功とする。
        Ok(())
    }
}

// --- private なヘルパー関数 ---

/// 指定されたディレクトリ内の画像からPDFを生成します。
fn process_directory(
    dir_path: &DirectoryPath,
    output_dir: &Path,
    font_path: Option<&Path>,
) -> Result<(), AppError> {
    // 1. 画像ファイルのパスを収集してソート
    let mut image_paths: Vec<PathBuf> = Vec::new();
    for entry_result in dir_path.entries()? {
        let entry_path = entry_result?.path();
        if is_image_file(&entry_path) {
            image_paths.push(entry_path);
        }
    }
    // ファイル名の順序を安定させるため、パスをソートする。
    image_paths.sort();

    // 2. 画像データを読み込み
    if image_paths.is_empty() {
        println!("  -> 画像ファイルが見つからなかったため、スキップします。");
        return Ok(()); // 画像がないのはエラーではないので Ok で抜ける
    }
    let mut images_data: Vec<Vec<u8>> = Vec::new();
    for path in &image_paths {
        images_data.push(fs::read(path)?);
    }

    // 3. ドメインオブジェクトを生成してPDFを作成・保存
    let data_name = dir_path
        .folder_name()
        .unwrap_or("untitled_folder")
        .to_string();
    let image_list = ImageDataList::new(images_data, &data_name)?;
    let font = PdfFont::new(font_path.and_then(|p| p.to_str()))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let pdf_file = PdfFile::create_file(&image_list, &font)?;

    // 出力パスを構築 (例: /output/dir/my_photos.pdf)
    let mut output_path = output_dir.to_path_buf();
    output_path.push(format!("{}.pdf", data_name));
    pdf_file.save_to_path(&output_path)?;

    println!(
        "  -> 完了: {} 枚の画像から {} を生成しました。",
        image_paths.len(),
        output_path.display()
    );

    Ok(())
}

/// 指定されたZIPファイル内の画像からPDFを生成します。
fn process_zip_file(
    zip_path: &ZipFilePath,
    output_dir: &Path,
    font_path: Option<&Path>,
) -> Result<(), AppError> {
    // 1. ZIPアーカイブ内の画像ファイルエントリ名を収集してソート
    let file = fs::File::open(zip_path.as_path())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let mut image_entry_names: Vec<String> = archive
        .file_names()
        .filter(|name| is_image_file(Path::new(name)))
        .map(|name| name.to_string())
        .collect();
    // ファイル名の順序を安定させるため、エントリ名をソートする。
    image_entry_names.sort();

    // 2. 画像データを読み込み
    if image_entry_names.is_empty() {
        println!("  -> 画像ファイルが見つからなかったため、スキップします。");
        return Ok(());
    }

    let mut images_data: Vec<Vec<u8>> = Vec::new();
    for name in &image_entry_names {
        // ZipFilePath に実装された read_entry メソッドは使えない（ライフタイムの問題）ため、
        // ここで直接 `zip` クレートを使って読み込む。
        let mut file_in_zip = archive
            .by_name(name)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut buffer = Vec::with_capacity(file_in_zip.size() as usize);
        std::io::copy(&mut file_in_zip, &mut buffer)?;
        images_data.push(buffer);
    }

    // 3. ドメインオブジェクトを生成してPDFを作成・保存
    let data_name = zip_path
        .file_name_with_extension(false) // 拡張子なしのファイル名を取得
        .unwrap_or("untitled_zip")
        .to_string();
    let image_list = ImageDataList::new(images_data, &data_name)?;
    let font = PdfFont::new(font_path.and_then(|p| p.to_str()))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let pdf_file = PdfFile::create_file(&image_list, &font)?;

    // 出力パスを構築 (例: /output/dir/my_archive.pdf)
    let mut output_path = output_dir.to_path_buf();
    output_path.push(format!("{}.pdf", data_name));
    pdf_file.save_to_path(&output_path)?;

    println!(
        "  -> 完了: {} 枚の画像から {} を生成しました。",
        image_entry_names.len(),
        output_path.display()
    );

    Ok(())
}

/// パスがサポートされている画像ファイルであるか、拡張子で簡易的に判定します。
fn is_image_file(path: &Path) -> bool {
    // `file_stem` がないとドットファイル (`.DS_Store` など) を誤判定するためチェック
    if path.is_file() && path.file_stem().is_some() {
        // 拡張子を小文字に変換して比較する
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "bmp"
            )
        } else {
            false
        }
    } else {
        false
    }
}
