// use宣言：必要なクレートやモジュールをスコープに取り込む

use image::ImageReader;
// use image::{self, GenericImageView}; // 画像のデコードと寸法取得のために利用
use std::fmt; // エラーメッセージのフォーマットのために fmt モジュールを利用
use std::io::Cursor;
// --- 構造体定義 ---

/// PDF作成などで利用することを想定した、検証済みの画像データコンテナ。
///
/// 内部的に複数の画像バイナリデータ（`Vec<u8>`）をリスト（`Vec`）として保持します。
/// `new` コンストラクタを通じてのみインスタンス化でき、その際に以下の点が保証されます。
/// - データが空でないこと
/// - すべての要素がサポートされている画像フォーマットであること
/// また、すべての画像を包含できる最大の幅と高さを自動的に計算して保持します。
#[derive(Debug, PartialEq)]
pub struct ImageDataList {
    images: Vec<Vec<u8>>,
    data_name: String,
    max_height: u32,
    max_width: u32,
}

// --- エラー定義 ---
#[derive(Debug)]
pub enum ImageValidationError {
    EmptyData,
    NotAnImage {
        index: usize,
        source: image::ImageError,
    },
}
impl fmt::Display for ImageValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageValidationError::EmptyData => {
                write!(f, "データが空です。画像データを1つ以上渡してください。")
            }
            ImageValidationError::NotAnImage { index, source } => {
                write!(
                    f,
                    "インデックス {} の要素を画像として読み取れません: {}",
                    index, source
                )
            }
        }
    }
}

impl std::error::Error for ImageValidationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotAnImage { source, .. } => Some(source),
            _ => None,
        }
    }
}

// --- 実装ブロック ---

impl ImageDataList {
    /// 画像のバイナリデータから幅と高さを取得するヘルパー関数。
    #[inline]
    fn get_dimensions(bytes: &[u8]) -> Result<(u32, u32), image::ImageError> {
        ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()? // シグネチャ変わることがあるので version 固定推奨
            .into_dimensions()
    }

    /// 新しい `ImageDataList` インスタンスを作成（コンストラクタ）。
    ///
    /// 渡されたすべての画像データから、最大の幅と高さを算出して保持します。
    ///
    /// # 引数
    /// * `data`: 画像のバイナリデータ（`Vec<u8>`）を要素とするベクター。
    /// * `data_name`: この画像リストを識別するための名前。
    ///
    /// # 戻り値
    /// * `Ok(ImageDataList)`: 有効な画像データが1つ以上含まれている場合。
    /// * `Err(ImageValidationError)`: データが空か、画像でない要素が含まれている場合。
    pub fn new(
        data: Vec<Vec<u8>>,
        data_name: impl Into<String>,
    ) -> Result<Self, ImageValidationError> {
        if data.is_empty() {
            return Err(ImageValidationError::EmptyData);
        }

        let mut max_width = 0u32;
        let mut max_height = 0u32;

        // 最大寸法の集約を行う
        for (i, bytes) in data.iter().enumerate() {
            let (w, h) =
                Self::get_dimensions(bytes).map_err(|e| ImageValidationError::NotAnImage {
                    index: i,
                    source: e,
                })?;
            if w > max_width {
                max_width = w;
            }
            if h > max_height {
                max_height = h;
            }
        }

        Ok(Self {
            images: data,
            data_name: data_name.into(),
            max_height,
            max_width,
        })
    }

    // --- 便利メソッド ---

    /// 保持している画像の枚数を返します。
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// 保持している画像が空かどうか。
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// (幅, 高さ) をまとめて取得。
    pub fn dimensions(&self) -> (u32, u32) {
        (self.max_width, self.max_height)
    }

    // --- ゲッターメソッド ---

    pub fn images(&self) -> &Vec<Vec<u8>> {
        &self.images
    }
    pub fn data_name(&self) -> &str {
        &self.data_name
    }
    pub fn max_height(&self) -> u32 {
        self.max_height
    }
    pub fn max_width(&self) -> u32 {
        self.max_width
    }
}

// --- テストモジュール ---

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    // --- テスト用ヘルパー関数 ---
    fn create_dummy_png(width: u32, height: u32, color: u8) -> Vec<u8> {
        let buf = vec![color; (width * height * 3) as usize];
        let mut result = Vec::new();
        let encoder = PngEncoder::new(&mut result);
        encoder
            .write_image(&buf, width, height, ExtendedColorType::Rgb8)
            .expect("PNGのエンコードに失敗");
        result
    }

    #[test]
    fn new_empty_returns_empty_error() {
        let res = ImageDataList::new(Vec::new(), "empty_data");
        assert!(matches!(res, Err(ImageValidationError::EmptyData)));
    }

    #[test]
    fn new_rejects_non_image_and_reports_index() {
        let valid_png = create_dummy_png(1, 1, 0);
        let not_an_image = b"this is not an image".to_vec();
        let data = vec![valid_png, not_an_image];
        let res = ImageDataList::new(data, "test_data");
        assert!(matches!(
            res,
            Err(ImageValidationError::NotAnImage { index: 1, .. })
        ));
    }

    /// `new` 関数が、サイズの揃った画像データから正しい寸法を取得することをテストします。
    #[test]
    fn new_accepts_valid_images_with_same_dimensions() {
        let img1 = create_dummy_png(10, 20, 0);
        let img2 = create_dummy_png(10, 20, 255);
        let data = vec![img1, img2];
        let res = ImageDataList::new(data, "correct_data").unwrap();
        assert_eq!(res.max_width(), 10);
        assert_eq!(res.max_height(), 20);
        assert_eq!(res.dimensions(), (10, 20));
        assert_eq!(res.len(), 2);
        assert!(!res.is_empty());
    }

    /// `new` 関数が、サイズの異なる複数の画像から最大の幅と高さを正しく計算することをテストします。
    #[test]
    fn new_calculates_max_dimensions_from_varied_sizes() {
        // Arrange
        let img1 = create_dummy_png(100, 50, 0); // 幅が最大
        let img2 = create_dummy_png(80, 200, 0); // 高さが最大
        let img3 = create_dummy_png(30, 30, 0); // 幅も高さも最大ではない
        let data = vec![img1, img2, img3];

        // Act
        let res = ImageDataList::new(data, "varied_sizes");

        // Assert
        assert!(res.is_ok());
        let image_list = res.unwrap();
        // 最も大きい幅(100)と最も大きい高さ(200)が設定されていることを確認
        assert_eq!(image_list.max_width(), 100);
        assert_eq!(image_list.max_height(), 200);
    }

    /// 単一の画像でも正しく動作することをテストします。
    #[test]
    fn new_works_with_single_image() {
        let img = create_dummy_png(123, 456, 0);
        let res = ImageDataList::new(vec![img], "single_image").unwrap();
        assert_eq!(res.max_width(), 123);
        assert_eq!(res.max_height(), 456);
    }
}
