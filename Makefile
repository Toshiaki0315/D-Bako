.PHONY: dev app dmg

BUNDLE_DIR := src-tauri/target/release/bundle

node_modules: package-lock.json
	npm ci
	@touch node_modules

# 開発モード (Vite + Tauri) で起動する
dev: node_modules
	npm run tauri dev

# .app バンドルをビルドする
app: node_modules
	npm run tauri build -- --bundles app
	@echo "=> $(BUNDLE_DIR)/macos/"

# .dmg インストーラをビルドする
dmg: node_modules
	npm run tauri build -- --bundles dmg
	@echo "=> $(BUNDLE_DIR)/dmg/"
