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
- [x] 5. [Test->Impl] 状態のシリアライズ・デシリアライズ（JSONベースの永続化モデル）(完了: `to_json` / `from_json`、読込時に不変条件を検証)

## Phase 3: macOS インフラストラクチャ層 (Rust)
- [x] 1. [Test->Impl] macOS `.app` バンドルからアプリアイコンを抽出する処理（NSWorkspace等の利用）(完了: `icon.rs`、`.icns` を直接読んで PNG 化。`Assets.car` のみのアプリは未対応で、NSWorkspace によるフォールバックは今後の課題)
- [x] 2. [Test->Impl] 特定のパスにあるアプリケーションを起動する処理（`open`コマンド）(ベース完了)
- [x] 3. [Test->Impl] ファイルシステム操作のモックテスト (完了: `storage.rs`、`FileSystem` トレイト + インメモリモック、一時ファイル→rename の保存)

## Phase 4: フロントエンド実装 (TypeScript)
- [x] 1. [Test->Impl] 縦2列×横4列のグリッドUIコンポーネントの作成 (ベース完了)
- [x] 2. [Test->Impl] ページネーション用UI（次へ/前へ）の作成 (ベース完了)
- [x] 3. [Test->Impl] Drag & Dropによるファイルパス取得イベントの実装 (ベース完了)
- [x] 4. [Test->Impl] 抽出したアプリアイコン画像の表示対応 (完了: `AppIcon` コンポーネント + `get_app_icon` コマンド。取得失敗時は絵文字で代替。実アプリでの目視確認は未実施)

## Phase 5: 統合 (Tauri IPC Bridge)
- [x] 1. Rustのコアロジック・インフラ層をTauri Commandsとしてエクスポート (一部完了)
- [x] 2. フロントエンドからTauri Commandsの呼び出し（D&D時の保存処理）(一部完了)
- [x] 3. フロントエンドからのアプリ起動コマンド呼び出し (完了)
- [x] 4. 状態の永続化（起動時のロード処理）(完了: `LauncherService` で変更ごとに保存、起動時に復元。壊れたファイルは `.bak` に退避。保存先は `app_data_dir/state.json`。GUI での再起動確認は未実施)
- [x] 5. E2E動作確認とコードの最終リファクタリング (完了: 実機で起動・状態復元・アイコン表示・クリック起動を確認。`.app` 以外のドロップ拒否とエラー表示を追加。Finder からの実ドロップは未確認)

## Phase 6: URL ショートカット
- [x] 1. [Test->Impl] アイテムモデルの拡張（`kind` / `icon`、旧保存ファイルとの互換、空きスロット指定の登録）
- [x] 2. [Test->Impl] URL の正規化・表示名・`.webloc` / `.url` の解析（`link.rs`、http(s) のみ・認証情報つきは拒否）
- [x] 3. [Test->Impl] favicon 取得（`favicon.rs`、HTTP をトレイト化してモックテスト。実通信は `net.rs` で `cargo test -- --ignored live`）
- [x] 4. [Test->Impl] Tauri コマンド（`add_url_to_first_empty` / `add_url_to_slot` / `add_url_file_to_first_empty`）と起動対象の検証
- [x] 5. [Test->Impl] フロントエンド（貼り付け Cmd+V、空きスロットのクリックで入力、`.webloc` / `.url` のドロップ、favicon・頭文字アイコン）(完了。実機でのキー入力による登録は未確認)

## Phase 7: メニュー・設定・表示の調整
- [x] 1. [Test->Impl] 設定の永続化（`settings.rs`、`settings.json`。欠けた項目・壊れたファイルは既定値、保存失敗時はメモリも変えない）。保存処理は `storage::write_atomic` に共通化
- [x] 2. メニューバーの日本語化と「設定…」(⌘,) の追加（`menu.rs`、Tauri 依存のためカバレッジ対象外）
- [x] 3. [Test->Impl] 設定ダイアログと「常に前に表示」（`get_settings` / `set_always_on_top`、起動時にも適用）(実機でのメニュー操作・チェック切り替えは未確認)
- [x] 4. タイトルバーに「道具箱」を表示（ウィンドウはダークテーマに固定）
- [x] 5. ページ移動を固定サイズ (px) で表示し、グリッドは幅・高さの両方に収まる大きさに変更（最小サイズでの表示を実機で確認）
- [x] 6. [Test->Impl] ウィンドウサイズの記憶（`window::restore_size` で保存幅を検証・最小幅〜画面幅に制限し、高さは縦横比から決定。リサイズ中はメモリに覚えるだけで、ウィンドウを閉じる時と終了時に `settings.json` へ保存）(実機で復元と終了時の保存を確認。手でのリサイズ操作は未確認)
- [x] 7. [Test->Impl] 登録アイテムの名前変更（`rename_app`、前後の空白を除去・空/100文字超は拒否、保存失敗時は元に戻す。右クリックメニュー「名前を変更…」から入力ダイアログ。URL 入力と共通の `TextDialog` に整理）(実機での右クリック操作は未確認)
- [x] 8. [Test->Impl] 設定画面を別ウィンドウ化（メインの子ウィンドウ「設定」、`index.html?view=settings` で `SettingsView` を表示。「常に前に表示」は呼び出し元ではなくメインウィンドウに反映、サイズ記憶はメインウィンドウのみ）。入力欄の CSS がチェックボックスに効いてレイアウトが崩れていた不具合も解消 (設定ウィンドウが開くことは確認、中身の見た目は未確認)
