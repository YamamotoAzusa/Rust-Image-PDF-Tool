use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// 変換対象のサブフォルダやZIPファイルが含まれる親フォルダのパス
    #[arg(required = true)]
    pub input_dir: PathBuf,

    /// PDFの出力先フォルダのパス (オプション: デフォルトは入力フォルダと同じ)
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,

    /// PDFに埋め込むTTF/OTFフォントファイルのパス (オプション: デフォルトは組み込みフォント)
    #[arg(short, long)]
    pub font_path: Option<PathBuf>,
}
