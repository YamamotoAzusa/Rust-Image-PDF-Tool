// use宣言：必要なクレートやモジュールをスコープに取り込む

use image::{self, GenericImageView}; // 画像のデコードと寸法取得のために利用
use std::fmt; // エラーメッセージのフォーマットのために fmt モジュールを利用

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
    image_height: u32,
    image_width: u32,
}

// --- エラー定義 ---

/// ` ` のインスタンス化時に発生する可能性のある検証エラー。
#[derive(Debug, PartialEq)]
pub enum ImageValidationError {
    /// 提供されたデータが空の場合に返されるエラー。
    EmptyData,
    /// データ内に画像として認識できない要素が含まれていた場合に返されるエラー。
    /// `index` フィールドには、問題が検出されたデータのインデックスが格納されます。
    NotAnImage { index: usize },
}

// --- 実装ブロック ---

impl ImageDataList {
    /// 画像のバイナリデータから幅と高さを取得するヘルパー関数。
    #[inline]
    fn get_dimensions(bytes: &[u8]) -> Result<(u32, u32), image::ImageError> {
        Ok(image::load_from_memory(bytes)?.dimensions())
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
            let (w, h) = Self::get_dimensions(bytes)
                .map_err(|_| ImageValidationError::NotAnImage { index: i })?;
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
            image_height: max_height,
            image_width: max_width,
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
        (self.image_width, self.image_height)
    }

    // --- ゲッターメソッド ---

    pub fn images(&self) -> &Vec<Vec<u8>> {
        &self.images
    }
    pub fn data_name(&self) -> &str {
        &self.data_name
    }
    pub fn image_height(&self) -> u32 {
        self.image_height
    }
    pub fn image_width(&self) -> u32 {
        self.image_width
    }
}

// --- トレイト実装 ---

impl fmt::Display for ImageValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageValidationError::EmptyData => {
                write!(f, "データが空です。画像データを1つ以上渡してください。")
            }
            ImageValidationError::NotAnImage { index } => {
                write!(
                    f,
                    "インデックス {} の要素が有効な画像データではありません。",
                    index
                )
            }
        }
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
        let mut buf = vec![color; (width * height * 3) as usize];
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
        assert_eq!(res, Err(ImageValidationError::EmptyData));
    }

    #[test]
    fn new_rejects_non_image_and_reports_index() {
        let valid_png = create_dummy_png(1, 1, 0);
        let not_an_image = b"this is not an image".to_vec();
        let data = vec![valid_png, not_an_image];
        let res = ImageDataList::new(data, "test_data");
        assert_eq!(res, Err(ImageValidationError::NotAnImage { index: 1 }));
    }

    /// `new` 関数が、サイズの揃った画像データから正しい寸法を取得することをテストします。
    #[test]
    fn new_accepts_valid_images_with_same_dimensions() {
        let img1 = create_dummy_png(10, 20, 0);
        let img2 = create_dummy_png(10, 20, 255);
        let data = vec![img1, img2];
        let res = ImageDataList::new(data, "correct_data").unwrap();
        assert_eq!(res.image_width(), 10);
        assert_eq!(res.image_height(), 20);
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
        assert_eq!(image_list.image_width(), 100);
        assert_eq!(image_list.image_height(), 200);
    }

    /// 単一の画像でも正しく動作することをテストします。
    #[test]
    fn new_works_with_single_image() {
        let img = create_dummy_png(123, 456, 0);
        let res = ImageDataList::new(vec![img], "single_image").unwrap();
        assert_eq!(res.image_width(), 123);
        assert_eq!(res.image_height(), 456);
    }
}
