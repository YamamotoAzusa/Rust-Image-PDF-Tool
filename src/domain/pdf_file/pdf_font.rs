// genpdfクレートのfontsモジュールと、エラーハンドリングに必要なError型をインポートします。
use genpdf::error::Error;
use genpdf::fonts::{FontData, FontFamily};
// ファイルシステムからフォントを読み込むために、標準ライブラリのfsモジュールをインポートします。
use std::fs;

/// PDFドキュメントで使用するフォントファミリーを管理するためのラッパー構造体。
///
/// この構造体は `genpdf::fonts::FontFamily` を保持し、
/// ファイルまたは埋め込みデータからのフォントの読み込みを簡潔に行うための
/// コンストラクタを提供します。
//
// 元のコードにあった `#[derive(PartialEq)]` は、内部の `FontFamily` 型が `PartialEq` トレイトを
// 実装していないためコンパイルエラーになります。代わりに `Clone` を追加すると、
// この構造体のインスタンスを複製できるため便利です。
#[derive(Debug, Clone)]
pub struct PdfFont(
    // フィールドを `pub` にすることで、外部モジュールから `my_font.0` のように
    // 内部の `FontFamily` に直接アクセスできます。
    pub FontFamily,
);

impl PdfFont {
    /// 新しい `PdfFont` インスタンスを作成します。
    ///
    /// 指定されたパスからフォントファイルを読み込むか、パスが指定されていない場合は
    /// デフォルトの埋め込みフォントを使用します。
    ///
    /// # 引数
    ///
    /// * `font_path`: TTFやOTFなどのフォントファイルへのパス (`&str`) を含む `Option`。
    ///   - `Some(path)`: 指定されたパスからフォントを読み込みます。
    ///   - `None`: コンパイル時にバイナリに埋め込まれたデフォルトフォント (`DejaVuSans.ttf`) を使用します。
    ///
    /// # 戻り値
    ///
    /// フォントの読み込みと解析に成功した場合は `Ok(PdfFont)` を返します。
    /// ファイルが存在しない、またはフォントデータが無効な場合は `Err(genpdf::error::Error)` を返します。
    pub fn new(font_path: Option<&str>) -> Result<Self, Error> {
        // `font_path` の有無に応じてフォントデータを読み込み、`FontData` インスタンスを生成します。
        // `if let` 式を使い、その結果を `font_data` 変数に束縛します。
        let font_data = if let Some(path) = font_path {
            // --- パスが指定されている場合 ---
            // `fs::read` でファイルの内容をバイトベクタ (`Vec<u8>`) として読み込みます。
            // ファイル読み込みは失敗する可能性があるため、`?` 演算子を使用します。
            // これにより、`fs::read` がエラーを返した場合、この `new` 関数は即座にそのエラーを返して終了します。
            let font_bytes = fs::read(path)?;

            // 読み込んだバイトデータから `FontData` を作成します。
            // フォントデータが不正な場合もエラーを返す可能性があるため、ここでも `?` を使います。
            FontData::new(font_bytes, None)?
        } else {
            // --- パスが指定されていない場合 (None) ---
            // `include_bytes!` マクロは、コンパイル時に指定されたファイルを読み込み、
            // その内容を `&'static [u8]` (静的ライフタイムを持つバイトスライス) としてバイナリに直接埋め込みます。
            // これにより、実行時にフォントファイルがなくてもプログラムは正しく動作します。
            let font_bytes = include_bytes!("../fonts/DejaVuSans.ttf");

            // 埋め込まれたバイトスライスから `FontData` を作成します。
            // `FontData::new` は `Vec<u8>` を要求するため、`.to_vec()` で変換します。
            FontData::new(font_bytes.to_vec(), None)?
        };

        // 一つの `FontData` をもとに `FontFamily` を構築します。
        // ここでは、通常(regular)、太字(bold)、斜体(italic)、太字斜体(bold_italic) の
        // 全てのスタイルに同じフォントデータを割り当てています。
        //
        // 注意: スタイルごとに異なるフォントファイル（例: `MyFont-Regular.ttf`, `MyFont-Bold.ttf`）を
        // 使用したい場合は、それぞれを個別に読み込んで `FontData` を作成し、各フィールドに設定する必要があります。
        let font_family = FontFamily {
            regular: font_data.clone(),
            bold: font_data.clone(),
            italic: font_data.clone(),
            // 最後のフィールドへの代入では、`font_data` の所有権がムーブされるため、`.clone()` は不要です。
            bold_italic: font_data,
        };

        // 構築した `FontFamily` を `PdfFont` でラップし、成功を示す `Ok` バリアントで包んで返します。
        Ok(PdfFont(font_family))
    }

    /// 内部に保持している `FontFamily` への不変参照を返します。
    ///
    /// このメソッドを使うことで、`genpdf` のドキュメントビルダーなどに
    /// フォントファミリーを直接渡すことができます。
    ///
    /// # 戻り値
    ///
    /// - `&genpdf::fonts::FontFamily`: 内部のフォントファミリーへの参照。
    pub fn get_font_family(&self) -> &FontFamily {
        // `self.0` はタプル構造体の最初の要素、つまり `FontFamily` インスタンスを指します。
        // その要素への参照を返します。
        &self.0
    }
}
