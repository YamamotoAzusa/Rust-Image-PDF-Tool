# 使用するベースイメージを指定
# rust:latestは、Rustの開発環境がすでにセットアップされている便利なイメージです。
FROM rust:latest

# 作業ディレクトリをコンテナ内に設定
WORKDIR /usr/src/app

# ローカルのCargo.tomlとCargo.lockをコンテナにコピー
# これにより、依存関係のキャッシュが効率的に使えます
COPY Cargo.toml Cargo.lock ./

# 依存関係をビルド（変更がない場合はこのステップはスキップされます）
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# アプリケーションのソースコードをコンテナにコピー
COPY . .

# Gemini-cliをインストール
# apt updateとapt installを1つのRUN命令にまとめることで効率を向上させます
RUN apt update && apt install -y nodejs npm

# Gemini-cliをインストール
RUN npm install -g @google/gemini-cli

# ビルドしたアプリケーションを実行可能ファイルとしてコピー
# ここではアプリケーションをビルドしています
RUN cargo build --release

# コンテナが起動したときに実行されるコマンドを設定
# シェルを起動し、ユーザーがコマンドを実行できるようにします
CMD ["/bin/bash"]