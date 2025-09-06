// 中間生成物である「画像データ」を型で表現する
// メモリ上の画像データのリスト (ファイル名, バイナリデータ)
struct InMemoryImages {
    file_name: String,
    data: Vec<u8>,
}

impl InMemoryImages {
    pub fn new(file_name: String, data: Vec<u8>) -> Self {
        Self { file_name, data }
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }
}
