// use宣言：必要なクレートやモジュールをスコープに取り込む
use image; // 画像フォーマットの推測に image クレートを利用
use std::fmt; // エラーメッセージのフォーマットのために fmt モジュールを利用

// --- 構造体定義 ---

/// PDF作成などで利用することを想定した、検証済みの画像データコンテナ。
///
/// 内部的に複数の画像バイナリデータ（`Vec<u8>`）をリスト（`Vec`）として保持します。
/// `new` コンストラクタを通じてのみインスタンス化でき、その際にデータが空でないことと、
/// すべての要素がサポートされている画像フォーマットであることが保証されます。
#[derive(Debug, PartialEq)]
pub struct ImageDataList(Vec<Vec<u8>>);

// --- エラー定義 ---

/// `ImageDataList` のインスタンス化時に発生する可能性のある検証エラー。
/// `PartialEq` を派生させることで、テストコード内で `assert_eq!` を使った直接比較が可能になります。
#[derive(Debug, PartialEq)]
pub enum ImageValidationError {
    /// 提供されたデータが空の場合に返されるエラー。
    EmptyData,
    /// データ内に画像として認識できない要素が含まれていた場合に返されるエラー。
    /// `index` フィールドには、問題が検出されたデータのインデックスが格納されます。
    NotAnImage { index: usize },
}

// --- 実装ブロック ---

// `ImageDataList` 構造体に関連するメソッドを実装します。
impl ImageDataList {
    /// 新しい `ImageDataList` インスタンスを作成（コンストラクタ）。
    ///
    /// # 引数
    /// * `data`: 画像のバイナリデータ（`Vec<u8>`）を要素とするベクター。各要素が1つの画像ファイルに対応します。
    ///
    /// # 戻り値
    /// * `Ok(ImageDataList)`: すべてのデータが有効な画像フォーマットである場合。
    /// * `Err(ImageValidationError)`: 検証に失敗した場合。
    ///     - `ImageValidationError::EmptyData`: `data` が空の場合。
    ///     - `ImageValidationError::NotAnImage`: `data` 内に画像でない要素が含まれている場合。
    pub fn new(data: Vec<Vec<u8>>) -> Result<Self, ImageValidationError> {
        // --- 事前条件チェック ---
        // 1. データが空でないことを確認します。
        // もし空であれば、処理を続行せずに `EmptyData` エラーを返します。
        if data.is_empty() {
            return Err(ImageValidationError::EmptyData);
        }

        // --- データ検証ループ ---
        // 2. 提供されたすべてのバイト列を順にチェックします。
        // `enumerate()` を使うことで、要素のインデックスと値の両方を取得できます。
        for (i, bytes) in data.iter().enumerate() {
            // `image::guess_format` を使い、バイト列の先頭部分（マジックナンバー）から
            // 画像フォーマットを推測します。
            // この関数は非常に高速で、画像全体をデコードする必要はありません。
            // 推測に失敗した場合（`is_err()` が true）、そのデータは画像ではないと判断します。
            if image::guess_format(bytes).is_err() {
                // 問題が発見された要素のインデックス `i` を含んだ `NotAnImage` エラーを返します。
                return Err(ImageValidationError::NotAnImage { index: i });
            }
        }

        // --- 成功時の処理 ---
        // すべての検証を通過した場合、`data` を持つ `ImageDataList` インスタンスを `Ok` で包んで返します。
        Ok(ImageDataList(data))
    }

    /// 内部に保持している画像データの不変参照を返すゲッターメソッド。
    ///
    /// これにより、外部コードは `ImageDataList` の中身を読み取ることができますが、
    /// 所有権を奪ったり、直接変更したりすることはできません。
    pub fn data(&self) -> &Vec<Vec<u8>> {
        &self.0
    }
}

// --- トレイト実装 ---

// `ImageValidationError` を人間が読める文字列として表示するための `Display` トレイトを実装します。
// これにより、`println!("{}", error);` のようにしてエラーメッセージを簡単に出力できるようになります。
impl fmt::Display for ImageValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `match` 式を使って、エラーの種類ごとに異なるメッセージをフォーマットします。
        match self {
            // `EmptyData` の場合
            ImageValidationError::EmptyData => {
                write!(f, "データが空です。画像データを1つ以上渡してください。")
            }
            // `NotAnImage` の場合。`{ index }` で中の値を取り出してメッセージに埋め込みます。
            ImageValidationError::NotAnImage { index } => {
                write!(f, "データの要素 {} が画像データではありません。", index)
            }
        }
    }
}

// --- テストモジュール ---

// `#[cfg(test)]` アトリビュートにより、このモジュールは `cargo test` 実行時のみコンパイルされます。
#[cfg(test)]
mod tests {
    // 親モジュール（このファイルの外側）から必要なものをインポートします。
    use super::*;

    /// `new` 関数に空のベクターを渡した際に `EmptyData` エラーが返されることをテストします。
    #[test]
    fn new_empty_returns_empty_error() {
        // Act: `ImageDataList::new` に空のベクターを渡して呼び出します。
        let res = ImageDataList::new(Vec::new());

        // Assert: 結果が期待通り `Err(ImageValidationError::EmptyData)` であることを確認します。
        assert_eq!(res, Err(ImageValidationError::EmptyData));
    }

    /// `new` 関数に画像でないデータが含まれている場合に、
    /// 正しいインデックスを持つ `NotAnImage` エラーが返されることをテストします。
    #[test]
    fn new_rejects_non_image_and_reports_index() {
        // Arrange: テストデータを用意します。
        // 1つ目は有効な画像データ（PNGヘッダー）、2つ目は明らかに画像でないテキストデータです。
        let valid_png_header = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let not_an_image = b"this is not an image".to_vec();
        let data = vec![valid_png_header, not_an_image];

        // Act: テストデータを渡して `ImageDataList::new` を呼び出します。
        let res = ImageDataList::new(data);

        // Assert: 結果が、インデックス `1` を持つ `NotAnImage` エラーであることを確認します。
        assert_eq!(res, Err(ImageValidationError::NotAnImage { index: 1 }));
    }

    /// `new` 関数が、既知の画像フォーマット（のマジックバイト）を正しく受け入れることをテストします。
    #[test]
    fn new_accepts_known_image_magic_bytes() {
        // Arrange: 一般的な画像フォーマットの先頭バイト列（マジックナンバー）を用意します。
        // `image::guess_format` はこれらの数バイトだけでフォーマットを推測できます。
        let png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]; // PNG
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG (SOI + APP0 マーカー)
        let gif = b"GIF89a".to_vec(); // GIF
        let data = vec![png, jpg, gif];

        // Act: これらの有効なデータを渡して `ImageDataList::new` を呼び出します。
        let res = ImageDataList::new(data);

        // Assert: 結果が `Ok` であることを確認します。`is_ok()` は Result が `Ok` かどうかを bool で返します。
        assert!(res.is_ok());
    }
}
