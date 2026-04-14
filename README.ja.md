# ymux

[English](./README.md) | [한국어](./README.ko.md) | **日本語**

Windows 向けの軽量な tmux スタイルのターミナルマルチプレクサ。

Tauri 2 (Rust) + WebView2 + xterm.js で構築されています。Windows 上で軽量かつ
高速にネイティブ動作しながら、保存されるレイアウト、ペインごとの作業ディレクトリ
と起動コマンド、切り替え可能なシェル (cmd / PowerShell / pwsh / Git Bash / WSL)、
そして各々が独自のレイアウトを記憶する番号付きワークスペースを提供します。

## 機能

- **永続化されるレイアウト**: 再帰的な水平 / 垂直分割。各ペインは自分のシェル、
  `cwd`、任意の起動コマンドを記憶します。
- **シェル自動検出**: システムから `cmd.exe`、Windows PowerShell、
  PowerShell 7 (`pwsh`)、Git Bash、WSL ディストリビューションを検出し、
  選択可能なプロファイルとして提示します。
- **番号付きワークスペース**: `Ctrl+1` .. `Ctrl+9` でワークスペースを切り替え
  ます。すべてのワークスペースは独自のレイアウトを保存します。ペインは切り替え
  を越えて生存し続けるため (tmux スタイル)、REPL や tail が死ぬことはありません。
- **軽量**: Tauri バイナリ + WebView2。インストーラのターゲットは 10 MB 未満。

## 開発

必要なもの: Rust (stable)、Node 20+、pnpm (または npm)。

```sh
pnpm install
pnpm tauri dev          # 開発モードで実行
pnpm tauri build        # Windows インストーラを生成 (Windows 上で実行)
```

Windows 以外のホストでも Rust クレートは `cargo check` がクリーンに通るため、
Linux/macOS でクロスプラットフォームのロジックを開発できますが、完全な
`tauri build` とエンドツーエンドの PTY 検証は Windows 上で行う必要があります。

## 設定

`%APPDATA%\ymux\config.toml` にワークスペース、レイアウト、キャッシュされた
シェルプロファイルが保存されます。構造変更のたびに (デバウンスあり) および
アプリ終了時に書き直されます。

## キーボード

| ショートカット      | アクション                |
|---------------------|---------------------------|
| `Ctrl+Shift+D`      | 水平分割                  |
| `Ctrl+Shift+-`      | 垂直分割                  |
| `Ctrl+Shift+W`      | フォーカス中のペインを閉じる |
| `Ctrl+Tab`          | ペインフォーカスを循環    |
| `Ctrl+1` .. `Ctrl+9`| ワークスペースを切り替え  |
| `Ctrl+Shift+N`      | 新規ワークスペース        |

## ステータス

初期 MVP。ロードマップについては `docs/` (予定) を参照してください。

## サポート

ymux がお役に立ちましたら、コーヒーを 1 杯おごっていただけると嬉しいです。
プロジェクトの継続的な開発に役立ちます。

[![ko-fi](https://img.shields.io/badge/Ko--fi-支援する-FF5E5B?logo=kofi&logoColor=white)](https://ko-fi.com/youngminkim)

<https://ko-fi.com/youngminkim>
