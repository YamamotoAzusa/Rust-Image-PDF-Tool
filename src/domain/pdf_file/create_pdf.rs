// --- 依存モジュール ---

// thiserror クレートを導入し、構造化されたエラーハンドリングを実現します。
// Cargo.toml の [dependencies] セクションに `thiserror = "1.0"` を追加する必要があります。
use thiserror::Error;

use super::pdf_font::PdfFont;
use crate::domain::image_data_list::ImageDataList;

// genpdf クレート
use genpdf::{elements, Alignment, Document, Rotation, Scale, SimplePageDecorator, Size};
// image クレート（寸法取得のために使用）
use image::GenericImageView;

// Rust 標準ライブラリ
use std::fs;
use std::io::Cursor;
use std::path::Path;

// --- 定数定義 ---
// マジックナンバーを排除し、可読性と保守性を向上させます。

/// PDF用紙 A4 の幅 (mm)
const A4_WIDTH_MM: f64 = 210.0;
/// PDF用紙 A4 の高さ (mm)
const A4_HEIGHT_MM: f64 = 297.0;
/// PDFのデフォルトマージン (mm)
const DEFAULT_MARGIN_MM: f64 = 10.0;
/// 画像のデフォルトDPI (印刷品質を想定)
const DEFAULT_DPI: f64 = 300.0;

/// ピクセル単位をミリメートル単位に変換します。
fn px_to_mm(px: u32, dpi: f64) -> f64 {
    (px as f64) / dpi * 25.4
}

/// PDF生成プロセスで発生する可能性のあるエラーを定義する列挙型。
/// thiserror を利用して、エラーの原因（source）を保持し、詳細な情報を提供します。
#[derive(Debug, Error)]
pub enum PdfError {
    #[error("画像 No.{index} のデコードに失敗しました")]
    ImageDecode {
        index: usize,
        #[source]
        source: image::ImageError,
    },

    #[error("画像 No.{index} のPDF要素への変換に失敗しました")]
    ImageToElement {
        index: usize,
        #[source]
        source: genpdf::error::Error,
    },

    #[error("PDFドキュメントのレンダリングに失敗しました")]
    Render(#[from] genpdf::error::Error),

    #[error("パス '{path}' へのPDFファイルの保存に失敗しました")]
    Save {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// メモリ上に生成されたPDFファイルとそのメタデータを保持する構造体。
pub struct PdfFile {
    pub file_name: String,
    pub image_data_list: ImageDataList,
    pub font: PdfFont,
    pub pdf_data: Vec<u8>,
}

impl PdfFile {
    /// 複数の画像データから、メモリ上に単一のPDFファイルを生成します。
    ///
    /// # 引数
    /// - `image_data_list`: PDFに含める画像データの集合体（借用）。
    /// - `pdf_font`: 文書に埋め込むフォント（借用）。
    ///
    /// # 戻り値
    /// - `Ok(Self)`: PDF生成に成功した場合、`PdfFile` インスタンスを返します。
    /// - `Err(PdfError)`: PDF生成中にエラーが発生した場合に返します。
    pub fn create_file(
        image_data_list: &ImageDataList,
        pdf_font: &PdfFont,
    ) -> Result<Self, PdfError> {
        let mut doc = Document::new(pdf_font.get_font_family().clone());

        doc.set_title(image_data_list.data_name().to_string());
        // genpdf 0.2.x では PaperSize::new は存在しないため Size::new を使用
        doc.set_paper_size(Size::new(A4_WIDTH_MM, A4_HEIGHT_MM));
        doc.set_minimal_conformance();

        let mut decorator = SimplePageDecorator::new();
        decorator.set_margins(DEFAULT_MARGIN_MM);
        doc.set_page_decorator(decorator);

        let usable_w = A4_WIDTH_MM - 2.0 * DEFAULT_MARGIN_MM;
        let usable_h = A4_HEIGHT_MM - 2.0 * DEFAULT_MARGIN_MM;

        for (idx, bytes) in image_data_list.images().iter().enumerate() {
            // STEP 1: 寸法取得のため、imageクレートで一度デコードする
            let dynimg = image::load_from_memory(bytes).map_err(|e| PdfError::ImageDecode {
                index: idx + 1,
                source: e,
            })?;

            // STEP 2: 最適なスケールと回転を計算する
            let (w_px, h_px) = dynimg.dimensions();
            let (scale, rotation) =
                Self::calculate_transform((w_px, h_px), (usable_w, usable_h), DEFAULT_DPI);

            // STEP 3: genpdf が扱える要素に変換（再デコードは genpdf 側に任せる）
            let mut img = elements::Image::from_reader(Cursor::new(bytes)).map_err(|e| {
                PdfError::ImageToElement {
                    index: idx + 1,
                    source: e,
                }
            })?;

            img.set_dpi(DEFAULT_DPI);
            img.set_scale(scale);
            if let Some(rot) = rotation {
                img.set_clockwise_rotation(rot);
            }
            img.set_alignment(Alignment::Center);

            // STEP 4: ドキュメントに画像要素を追加する
            doc.push(img);

            // STEP 5: 最後の画像でなければ改ページを挿入する
            if idx + 1 < image_data_list.images().len() {
                doc.push(elements::PageBreak::new());
            }
        }

        // STEP 6: PDFをメモリ上のバイト列としてレンダリングする
        let mut pdf_bytes: Vec<u8> = Vec::new();
        doc.render(&mut pdf_bytes)?; // `?` 演算子で PdfError::Render に自動変換

        // 成功した場合、PdfFile 構造体を返す
        Ok(Self {
            file_name: image_data_list.data_name().to_string(),
            // 構造体が所有権を持つため、ここで clone する
            image_data_list: image_data_list.clone(),
            font: pdf_font.clone(),
            pdf_data: pdf_bytes,
        })
    }

    /// PDFデータを指定されたパスにファイルとして保存します。
    ///
    /// # 引数
    /// - `path`: 保存先のファイルパス。`&str`, `String`, `PathBuf` などを受け入れ可能。
    ///
    /// # 戻り値
    /// - `Ok(())`: 保存に成功した場合。
    /// - `Err(PdfError::Save)`: 書き込みに失敗した場合。
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), PdfError> {
        let path_ref = path.as_ref();
        fs::write(path_ref, &self.pdf_data).map_err(|e| PdfError::Save {
            path: path_ref.to_string_lossy().into_owned(),
            source: e,
        })
    }

    /// 画像の寸法と描画可能領域から、最適な拡大率と回転を計算するヘルパー関数。
    fn calculate_transform(
        img_dims_px: (u32, u32),
        usable_area_mm: (f64, f64),
        dpi: f64,
    ) -> (Scale, Option<Rotation>) {
        let (w_px, h_px) = img_dims_px;
        let (usable_w, usable_h) = usable_area_mm;

        let w_mm = px_to_mm(w_px, dpi);
        let h_mm = px_to_mm(h_px, dpi);

        // 回転なしの場合のスケール
        let scale_no_rot = (usable_w / w_mm).min(usable_h / h_mm);
        // 90度回転した場合のスケール
        let scale_rot90 = (usable_w / h_mm).min(usable_h / w_mm);

        if scale_rot90 > scale_no_rot {
            // 回転した方が大きく表示できる場合
            let scale_val = scale_rot90.min(1.0); // 1.0 を超える拡大はしない
            (
                Scale::new(scale_val, scale_val),
                Some(Rotation::from_degrees(90.0)),
            )
        } else {
            let scale_val = scale_no_rot.min(1.0);
            (Scale::new(scale_val, scale_val), None)
        }
    }
}
