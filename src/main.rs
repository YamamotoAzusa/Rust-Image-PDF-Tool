mod cli;
mod workflow;
use crate::cli::Args;
use clap::Parser;
use std::error::Error;
use std::process;
fn main() {
    // 1. 引数を解析
    let args = Args::parse();

    // 2. メインワークフローを実行
    println!("処理を開始します: {}", args.input_dir.display());
    if let Err(e) = workflow::run(args) {
        eprintln!("エラーが発生しました: {}", e);
        // エラーの原因が複数層にわたる場合、根本原因も表示する
        let mut source = e.source();
        while let Some(s) = source {
            eprintln!("  原因: {}", s);
            source = s.source();
        }
        process::exit(1);
    }

    println!("すべての処理が正常に完了しました。");
}
