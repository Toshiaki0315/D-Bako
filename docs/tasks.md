# 開発タスク一覧

AIエージェント（Claude）は、このタスクリストを上から順にTDDサイクル（Test -> Impl -> Refactor）で実行してください。

## Phase 1: プロジェクトセットアップ
- [x] 1. Tauri v2 (TypeScript + Rust) プロジェクトの初期化 (完了)
- [x] 2. フロントエンドのテスト環境構築 (Vitest等) と初期テスト実行 (完了: Vitest + jsdom + Testing Library + v8 coverage、閾値90%)
- [x] 3. Rustバックエンドのテスト環境確認 (`cargo test`) (ベース完了)
- [x] 4. GitHub Actions等を利用したCIでのテスト自動化設定 (カバレッジ計測含む) (完了: `.github/workflows/ci.yml`、フロント/Rust とも90%ゲート。初回実行はPush後に要確認)

## Phase 2: コアロジック (Rust / Framework Independent)
*※Tauri APIに依存しない純粋なRustモジュールとして実装*
- [x] 1. [Test->Impl] ページネーション設定管理（初期1ページ、最大10ページのバリデーション）(ベース完了)
- [x] 2. [Test->Impl] 2x4グリッド上のスロット状態管理（アプリの登録）(ベース完了)
- [x] 3. [Test->Impl] 80個（8個×10ページ）の最大登録数制限ロジック (ベース完了)
- [x] 4. [Test->Impl] スロット状態管理の拡張（削除、移動）(完了: `remove_app` / `move_app`、移動先が埋まっていれば入れ替え)
- [ ] 5. [Test->Impl] 状態のシリアライズ・デシリアライズ（JSONベースの永続化モデル）

## Phase 3: macOS インフラストラクチャ層 (Rust)
- [ ] 1. [Test->Impl] macOS `.app` バンドルからアプリアイコンを抽出する処理（NSWorkspace等の利用）
- [x] 2. [Test->Impl] 特定のパスにあるアプリケーションを起動する処理（`open`コマンド）(ベース完了)
- [ ] 3. [Test->Impl] ファイルシステム操作のモックテスト

## Phase 4: フロントエンド実装 (TypeScript)
- [x] 1. [Test->Impl] 縦2列×横4列のグリッドUIコンポーネントの作成 (ベース完了)
- [x] 2. [Test->Impl] ページネーション用UI（次へ/前へ）の作成 (ベース完了)
- [x] 3. [Test->Impl] Drag & Dropによるファイルパス取得イベントの実装 (ベース完了)
- [ ] 4. [Test->Impl] 抽出したアプリアイコン画像の表示対応

## Phase 5: 統合 (Tauri IPC Bridge)
- [x] 1. Rustのコアロジック・インフラ層をTauri Commandsとしてエクスポート (一部完了)
- [x] 2. フロントエンドからTauri Commandsの呼び出し（D&D時の保存処理）(一部完了)
- [x] 3. フロントエンドからのアプリ起動コマンド呼び出し (完了)
- [ ] 4. 状態の永続化（起動時のロード処理）
- [ ] 5. E2E動作確認とコードの最終リファクタリング
