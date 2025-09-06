// // src/main.rs
// mod cli;
// mod domain;
// mod infrastructure;
// mod workflow;

// use clap::Parser;
// use cli::Args;
// fn main() {
//     // 1. 引数を解析
//     let args = Args::parse();
//     println!("処理を開始します: {}", args.input_path.display());

//     // 2. メインワークフローを実行
//     match workflow::run_conversion(args.input_path) {
//         Ok(_) => println!("すべての処理が正常に完了しました。"),
//         Err(e) => eprintln!("エラーが発生しました: {:?}", e),
//     }
// }
mod domain;
