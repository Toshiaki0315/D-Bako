# D-Bako (道具箱)

macOS向けのグリッド型ランチャーアプリケーションです。

## リポジトリ
[https://github.com/Toshiaki0315/D-Bako.git](https://github.com/Toshiaki0315/D-Bako.git)

## 主な機能
- 1ページあたり縦2列×横4列の最大8アプリを登録可能
- 最大10ページ（合計80アプリ）までの拡張設定
- Drag & Dropによる直感的なアプリ登録
- アプリアイコンの自動抽出・表示と、クリックによる起動

## 技術スタック
- **Frontend**: TypeScript, React (Tauri v2)
- **Backend**: Rust (Tauri v2)
- **Architecture**: クリーンアーキテクチャ・依存関係のないコアロジック分離

## 開発プロセス (TDD)
ユニットテストカバレッジ90%以上を目標に、以下のサイクルで開発を進めます。
1. `docs/tasks.md` に基づきテストを記述 (Red)
2. 最小限の実装を実施 (Green)
3. リファクタリング (Refactor)
