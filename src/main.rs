use clap::Parser;
use genpdf::elements::Image;
use genpdf::{fonts, Document};
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipArchive;

/// 指定された親フォルダ内の画像フォルダとZIPファイルをPDFに変換するツール
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 親フォルダのパス
    #[arg(short, long)]
    input_path: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // コマンドライン引数を解析します
    let args = Args::parse();
    let parent_folder = &args.input_path;

    // 指定されたパスが存在し、ディレクトリであることを確認します
    if !parent_folder.is_dir() {
        eprintln!("エラー: 指定されたパスはディレクトリではありません。");
        return Ok(()); // エラーメッセージを出力して正常終了
    }

    println!("処理を開始します: {}", parent_folder.display());

    // 親フォルダ内のエントリをループ処理します
    for entry in fs::read_dir(parent_folder)? {
        let entry = entry?;
        let path = entry.path();
        // エントリがディレクトリの場合
        if path.is_dir() {
            if let Err(e) = process_directory(&path) {
                eprintln!(
                    "ディレクトリ {} の処理中にエラーが発生しました: {}",
                    path.display(),
                    e
                );
            }
        }
        // エントリがZIPファイルの場合
        else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("zip") {
            if let Err(e) = process_zip_file(&path) {
                eprintln!(
                    "ZIPファイル {} の処理中にエラーが発生しました: {}",
                    path.display(),
                    e
                );
            }
        }
    }

    println!("すべての処理が完了しました。");
    Ok(())
}

/// 指定されたディレクトリ内の画像を1つのPDFにまとめます。
fn process_directory(dir_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("ディレクトリを処理中: {}", dir_path.display());

    // 画像ファイルのリストを収集します
    let image_paths = find_image_files(dir_path);

    if image_paths.is_empty() {
        println!(
            "ディレクトリ {} に画像ファイルが見つかりませんでした。",
            dir_path.display()
        );
        return Ok(());
    }

    // PDFファイル名はディレクトリ名と同じにします
    let pdf_file_name = dir_path.file_name().unwrap().to_str().unwrap();
    let pdf_path = dir_path.with_file_name(format!("{}.pdf", pdf_file_name));

    // 画像をPDFに変換します
    create_pdf_from_images(&image_paths, &pdf_path)?;

    println!("PDFを作成しました: {}", pdf_path.display());
    Ok(())
}

/// 指定されたZIPファイル内の画像を1つのPDFにまとめます。
fn process_zip_file(zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("ZIPファイルを処理中: {}", zip_path.display());

    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut images: Vec<(String, Vec<u8>)> = Vec::new();

    // ZIPアーカイブ内のファイルをループ処理します
    for i in 0..archive.len() {
        let mut file_in_zip = archive.by_index(i)?;
        let file_name = file_in_zip.name().to_string();

        // ファイルが画像であるか拡張子で簡易的にチェックします
        if is_image_file(&PathBuf::from(&file_name)) {
            let mut buffer = Vec::new();
            file_in_zip.read_to_end(&mut buffer)?;
            images.push((file_name.clone(), buffer));
        }
    }

    if images.is_empty() {
        println!(
            "ZIPファイル {} に画像ファイルが見つかりませんでした。",
            zip_path.display()
        );
        return Ok(());
    }

    // PDFファイル名はZIPファイル名（拡張子なし）と同じにします
    let pdf_file_name = zip_path.file_stem().unwrap().to_str().unwrap();
    let output_dir = zip_path.parent().unwrap_or_else(|| Path::new("."));
    let pdf_path = output_dir.join(format!("{}.pdf", pdf_file_name));

    // メモリ上の画像データからPDFを作成します
    create_pdf_from_memory_images(images, &pdf_path)?;

    println!("PDFを作成しました: {}", pdf_path.display());
    Ok(())
}

/// ディレクトリ内から画像ファイルのパスを再帰的に検索します。
fn find_image_files(dir_path: &Path) -> Vec<PathBuf> {
    WalkDir::new(dir_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() && is_image_file(e.path()))
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// ファイルパスが対応している画像フォーマットか判定します。
fn is_image_file(path: &Path) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => ["jpg", "jpeg", "png", "gif", "bmp"].contains(&ext.to_lowercase().as_str()),
        _ => false,
    }
}

/// デフォルトのフォントデータを返します。
fn get_default_font() -> Result<fonts::FontFamily<fonts::FontData>, Box<dyn std::error::Error>> {
    // let font_path = "/workspaces/my-rust-gemini-app/fonts/DejaVuSans.ttf";
    // include_bytes!マクロを使用してフォントファイルをバイナリに埋め込む
    let font_data = fonts::FontData::new(include_bytes!("../fonts/DejaVuSans.ttf").to_vec(), None)?;

    // Ok(fonts::FontFamily {
    //     regular: fonts::FontData::load(font_path, None)?,
    //     bold: fonts::FontData::load(font_path, None)?,
    //     italic: fonts::FontData::load(font_path, None)?,
    //     bold_italic: fonts::FontData::load(font_path, None)?,
    // })
    Ok(fonts::FontFamily {
        regular: font_data.clone(),
        bold: font_data.clone(),
        italic: font_data.clone(),
        bold_italic: font_data,
    })
}

/// 画像ファイルのパスのリストからPDFを作成します。
fn create_pdf_from_images(
    image_paths: &[PathBuf],
    output_pdf_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if image_paths.is_empty() {
        return Ok(());
    }

    // ドキュメントとフォントの準備
    let font_family = get_default_font()?;
    let mut doc = Document::new(font_family);
    doc.set_title(output_pdf_path.file_stem().unwrap().to_str().unwrap());

    // 各画像をPDFの新しいページに追加します
    for image_path in image_paths {
        println!("  画像を追加中: {}", image_path.display());
        let image = Image::from_path(image_path)?;
        doc.push(image);
    }

    // PDFをファイルに保存します
    doc.render_to_file(output_pdf_path)?;
    Ok(())
}

/// メモリ上の画像データのリストからPDFを作成します。
fn create_pdf_from_memory_images(
    images: Vec<(String, Vec<u8>)>,
    output_pdf_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if images.is_empty() {
        return Ok(());
    }

    // ドキュメントとフォントの準備
    let font_family = get_default_font()?;
    let mut doc = Document::new(font_family);
    doc.set_title(output_pdf_path.file_stem().unwrap().to_str().unwrap());

    // 各画像をPDFの新しいページに追加します
    for (name, data) in images {
        println!("  画像を処理中: {}", name);
        let image = Image::from_reader(Cursor::new(data))?;
        doc.push(image);
    }

    // PDFをファイルに保存します
    doc.render_to_file(output_pdf_path)?;
    Ok(())
}
