// --- 依存モジュール ---

// image_data_list モジュールから ImageDataList 構造体を利用します。
// この構造体は、PDFに含める複数の画像データとそのメタデータを管理するものと想定されます。
use image_data_list::ImageDataList;
// pdf_font モジュールから PdfFont 構造体を利用します。
// 日本語などのマルチバイト文字を扱うためのフォント情報を管理するものと想定されます。
use pdf_font::PdfFont;

// genpdf クレートを利用して PDF を生成します。
// elements モジュールは画像やテキストなどのPDF要素を、
// Alignment は要素の配置（中央揃えなど）を、
// Document はPDF文書全体を、
// Size は用紙サイズ（A4など）を定義します。
use genpdf::{
    self, elements, Alignment, Document, PaperSize, Rotation, Scale, SimplePageDecorator,
};
use image::DynamicImage;

// Rustの標準ライブラリである std::fs を利用して、ファイルシステムへの書き込み操作を行います。
use std::fs;

fn px_to_mm(px: u32, dpi: f64) -> f64 {
    (px as f64) / dpi * 25.4
}

/// PDFの生成やファイル保存時に発生する可能性のあるエラーを定義する列挙型。
/// これにより、呼び出し元はエラーの種類に応じた適切な処理を実装できます。
#[derive(Debug, PartialEq)]
pub enum PdfValidationError {
    /// PDFドキュメントの構築やレンダリング中にエラーが発生した場合。
    /// 例えば、画像データのデコード失敗などが該当します。
    PdfCreationError(String),
    /// 生成されたPDFデータをファイルとしてディスクに保存する際にエラーが発生した場合。
    /// 例えば、書き込み権限がないパスを指定した場合などが該当します。
    PdfSaveError(String),
}

/// メモリ上に生成されたPDFファイルとそのメタデータを保持する構造体。
/// PDFのバイトデータだけでなく、元となった画像や使用したフォント情報も保持することで、
/// 後から参照したり、再利用したりすることが容易になります。
pub struct PdfFile {
    /// PDFのファイル名やドキュメントのタイトルとして使用される文字列。
    /// 通常は元の画像データリストの名前から設定されます。
    pub file_name: String,
    /// このPDFのソースとなった元の画像データリスト。
    /// PDF生成後もこのデータを保持することで、どの画像から作られたかを追跡できます。
    pub image_data_list: ImageDataList,
    /// PDF生成時に使用されたフォント情報。
    /// 特に、テキストを含む要素を後から追加する際などに必要となる可能性があります。
    pub font: PdfFont,
    /// メモリ上にレンダリングされたPDFファイルのバイナリデータ（バイト列）。
    /// このデータを直接ファイルに保存したり、HTTPレスポンスとして返したりできます。
    pub pdf_data: Vec<u8>,
}

impl PdfFile {
    /// 複数の画像データを含む `ImageDataList` から、メモリ上に単一のPDFファイルを生成します。
    /// 各画像は、それぞれPDFの1ページとして配置されます。
    ///
    /// # 引数
    /// - `image_data_list`: PDFの各ページに配置する画像データの集合体 (`ImageDataList`)。
    /// - `pdf_font`: 文書に埋め込むデフォルトフォント。日本語などのマルチバイト文字を正しく表示するために必要です。
    ///
    /// # 戻り値
    /// - `Ok(Self)`: PDFの生成に成功した場合、`PdfFile` インスタンスを返します。
    /// - `Err(PdfValidationError)`: 画像のデコード失敗など、PDF生成中にエラーが発生した場合に返します。
    pub fn create_file(
        image_data_list: ImageDataList,
        pdf_font: PdfFont,
    ) -> Result<Self, PdfValidationError> {
        // genpdf の Document::new は FontFamily 型の所有権を要求するため、
        // pdf_font から clone() して新しい所有権を持つ値を渡します。
        let mut doc = Document::new(pdf_font.get_font_family().clone());

        // PDFのメタデータを設定します。
        // data_name() メソッドから取得した名前をドキュメントのタイトルに設定します。
        doc.set_title(image_data_list.data_name().to_string());
        // 用紙サイズをA4に明示的に設定します。（genpdfのデフォルトもA4です）
        doc.set_paper_size(Size::A4);
        // PDFの互換性を最小限に設定します。
        // これにより、ICCカラープロファイルやXMPメタデータなどが省略され、ファイルサイズを削減できます。
        doc.set_minimal_conformance();

        // 余白 10mm を付ける
        let mut decorator = SimplePageDecorator::new();
        decorator.set_margins(10); // mm 指定。:contentReference[oaicite:6]{index=6}
        doc.set_page_decorator(decorator);

        // A4 (mm)
        let page_w_mm = 210.0;
        let page_h_mm = 297.0;
        let margin_mm = 10.0;
        let usable_w = page_w_mm - 2.0 * margin_mm;
        let usable_h = page_h_mm - 2.0 * margin_mm;

        // 印刷を見据え DPI は 300 を既定（画像が大きすぎる場合は自動縮小）
        let dpi = 300.0;

        // 画像リスト内の各画像データをループ処理し、1つずつPDFページとして追加します。
        // .iter().enumerate() を使うことで、インデックス（何番目の画像か）と画像データの両方を取得できます。
        for (idx, bytes) in image_data_list.images().iter().enumerate() {
            // STEP 1: 画像のバイトデータをデコードする
            // image クレートの load_from_memory 関数を使い、メモリ上のバイト列から動的な画像オブジェクトを生成します。
            // map_err を使って、image クレートのエラーを独自のエラー型 PdfValidationError に変換します。
            // これにより、どの画像で問題が発生したかを示す詳細なエラーメッセージを生成できます。
            let dynimg = image::load_from_memory(bytes).map_err(|e| {
                PdfValidationError::PdfCreationError(format!(
                    "画像 No.{} の読み込みに失敗しました: {}", // ユーザーフレンドリーなエラーメッセージ
                    idx + 1, // インデックスは0から始まるため、+1して表示
                    e
                ))
            })?;

            let (w_px, h_px) = dynimg.dimensions();
            let (w_mm, h_mm) = (px_to_mm(w_px, dpi), px_to_mm(h_px, dpi));

            // 回転なし／90°回転の両案でスケールを計算
            let s0 = (usable_w / w_mm).min(usable_h / h_mm);
            let s90 = (usable_w / h_mm).min(usable_h / w_mm);

            let (scale, rotate_deg) = if s90 > s0 {
                (s90.min(1.0), Some(90.0))
            } else {
                (s0.min(1.0), None)
            };

            // STEP 2: genpdf が扱える要素に変換する
            // デコードした動的画像オブジェクトを、genpdf の Image 要素に変換します。
            // この処理でもエラーが発生する可能性があるため、同様にエラーハンドリングを行います。
            let mut img = elements::Image::from_dynamic_image(dynimg).map_err(|e| {
                PdfValidationError::PdfCreationError(format!(
                    "画像 No.{} のPDF要素への変換に失敗しました: {}",
                    idx + 1,
                    e
                ))
            })?;

            img.set_dpi(dpi); // 物理サイズ基準を明示。:contentReference[oaicite:7]{index=7}
            img.set_scale(Scale::new(scale, scale)); // 等比縮小。:contentReference[oaicite:8]{index=8}
            if let Some(deg) = rotate_deg {
                img.set_clockwise_rotation(Rotation::from_degrees(deg)); // 90°回して収まりを最適化。:contentReference[oaicite:9]{index=9}
            }

            // STEP 3: 画像の配置方法を設定する
            // 画像をページの中央に配置するよう設定します。
            // 他の配置（左寄せ: Left, 右寄せ: Right）も genpdf::Alignment 列挙体で指定可能です。
            // 画像のサイズをページの幅に合わせたい場合は、set_scale() や set_dpi() メソッドで調整できます。
            img.set_alignment(Alignment::Center);

            // STEP 4: ドキュメントに画像要素を追加する
            // 設定済みの画像要素をドキュメントに追加します。この時点ではまだページにはレンダリングされません。
            doc.push(img);

            // STEP 5: 次の画像のために改ページを挿入する
            // 「1画像 = 1ページ」のレイアウトを実現するため、画像の後に改ページ要素を追加します。
            // ただし、最後の画像の後には不要な空白ページができてしまうため、改ページは挿入しません。
            if idx + 1 < image_data_list.images().len() {
                doc.push(elements::PageBreak::new());
            }
        }

        // STEP 6: PDFをメモリ上のバイト列としてレンダリングする
        // これまでの処理でドキュメントに追加された全ての要素を、PDF形式のバイト列に変換します。
        let mut pdf_bytes: Vec<u8> = Vec::new();
        doc.render(&mut pdf_bytes)
            .map_err(|e| PdfValidationError::PdfCreationError(e.to_string()))?;

        // 成功した場合、生成されたPDFデータと関連情報を持つ PdfFile 構造体を返します。
        Ok(Self {
            file_name: image_data_list.data_name().to_string(),
            image_data_list, // 引数で受け取った image_data_list の所有権をムーブ
            font: pdf_font,  // 引数で受け取った pdf_font の所有権をムーブ
            pdf_data: pdf_bytes,
        })
    }

    /// `self.pdf_data` に保持されているPDFのバイト列を、指定されたパスにファイルとして保存します。
    ///
    /// # 引数
    /// - `path`: PDFファイルを保存する先のファイルパス（例: `"output/document.pdf"`)。
    ///
    /// # 戻り値
    /// - `Ok(())`: ファイルの保存に成功した場合。
    /// - `Err(PdfValidationError::PdfSaveError)`: ファイルの書き込みに失敗した場合。
    pub fn save_to_path(&self, path: &str) -> Result<(), PdfValidationError> {
        // Rustの標準ライブラリ fs::write を使用して、バイト列をファイルに一括で書き込みます。
        // この関数は内部でファイルのオープン、書き込み、クローズを自動的に行います。
        // I/Oエラーが発生した場合は、map_err を使って io::Error を独自のエラー型に変換して返します。
        fs::write(path, &self.pdf_data).map_err(|e| PdfValidationError::PdfSaveError(e.to_string()))
    }
}
