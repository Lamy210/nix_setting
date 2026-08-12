# nix_setting

Nix + Home Manager によるポータブルターミナル環境。Mac / Ubuntu / VPS で再現可能。

## 構成

```
nix_setting/
  flake.nix              # Nix flake (formatter + homeConfig + 7 devShells)
  home.nix               # 8 programs modules + 10 dotfiles + 81 packages
  bootstrap.sh           # 初回セットアップ (user-options.nix 自動生成)
  Brewfile               # 推奨 Homebrew casks
  .github/workflows/     # CI (flake check + secret scan)
  config/
    zsh/zshrc            starship/starship.toml  wezterm/wezterm.lua
    git/gitconfig        tmux/tmux.conf          lazygit/config.yml
    yazi/yazi.toml       broot/conf.toml         mise/config.toml
    openspec/config.json just/justfile
```

## 導入手順

```bash
# 1. Nix インストール
curl -L https://nixos.org/nix/install | sh

# 2. クローン & 適用
git clone https://github.com/Lamy210/nix_setting.git "$HOME/nix_setting"
cd "$HOME/nix_setting"
./bootstrap.sh

# 3. 背景画像 (任意 — なければ暗色背景にフォールバック)
mkdir -p ~/.config/wezterm/backgrounds
cp Yukihana.Lamy.jpg ~/.config/wezterm/backgrounds/

# 4. 個人 Git 設定
cat > ~/.gitconfig.local <<EOF
[user]
	name = Your Name
	email = your@example.com
EOF
```

## Homebrew casks (nix-darwin管理)

```bash
nix run nix-darwin -- switch --flake .
```

| アプリ | 用途 |
|--------|------|
| WezTerm | ターミナルエミュレータ |
| VS Code | エディタ |
| PlemolJP Console NF | 日本語 + Nerd Font |
| JetBrains Mono Nerd Font | 英字等幅 + アイコン |
| Loop | ウィンドウ管理 |
| FlashSpace | 高速ワークスペース切替 |
| Tameo | クリップボード履歴 + OCR |

## 導入されるツール (81 packages + 8 modules)

### Shell (programs.zsh)
zsh + autosuggestions + syntax-highlighting + zsh-abbr (fish-style abbreviations) + zsh-completions

### Prompt
Starship — 11 modules: git / nodejs / golang / rust / python / bun / docker / kubernetes / terraform / cmd_duration / hostname(SSH時のみ)

### ファイル操作
| コマンド | 用途 |
|----------|------|
| `ls` / `ll` / `la` | → eza (アイコン・git表示付き) |
| `cat` | → bat (シンタックスハイライト) |
| `fdx` | → fd (高速ファイル検索) |
| `grep` | → ripgrep (高速テキスト検索) |
| `duu` | → gdu (高速ディスク使用量) |
| `tree` | → eza tree (アイコン付きツリー) |
| `diffs` | → difftastic (構造ベースdiff) |

### Git
| コマンド | 用途 |
|----------|------|
| `lg` | lazygit (Git TUI) |
| `g` / `gs` / `gb` / `gc` / `gp` / `gl` / `gd` / `gco` / `gst` / `grb` | エイリアス |
| `git undo` | 直前のコミット取消 |
| `git amend` | 直前コミットに追記 |
| `git unstage` | ステージ取り消し |
| `git ds` | ステージ済みdiff |
| `git lg` | グラフ付きログ (全ブランチ) |
| `git who` | コントリビュータ一覧 |
| delta | Git ページャー (side-by-side diff) |

### fzf + Git 関数
| 関数 | 動作 |
|------|------|
| `gcb` | ブランチ一覧 → fzf選択 → checkout |
| `glog` | git log → fzf → 選択コミットをshow |
| `gshow` | stash 一覧 → fzf → pop |
| `cdf` | ディレクトリをfzf検索 → cd |
| `fv` | ファイルをfzf検索 → エディタで開く |

### シェル関数
| 関数 | 動作 |
|------|------|
| `mkcd <dir>` | mkdir + cd |
| `extract <file>` | アーカイブ展開 (ouch経由, tar/zip/gz/bz2/xz対応) |
| `trash <file>` | ゴミ箱へ安全削除 (trashy) |

### 略語展開 (zsh-abbr)
| 略語 | 展開先 |
|------|--------|
| `k` | `kubectl` |
| `h` | `helm` |
| `tf` | `terraform` |

### コンテナ / K8s (すべて OSS)
| コマンド | 用途 |
|----------|------|
| `col` / colima | コンテナランタイム (Docker Desktop代替, MIT) |
| docker / `dc` | Docker CLI + Compose |
| `lzd` | lazydocker (Docker TUI) |
| kubectl / `k` | Kubernetes CLI |
| helm / `h` | K8s パッケージマネージャ |
| kind | ローカルK8sクラスタ |
| terraform | IaC (Infrastructure as Code) |

### データベース
| コマンド | 用途 |
|----------|------|
| pgcli | PostgreSQL CLI (補完・ハイライト) |
| mycli | MySQL CLI (補完・ハイライト) |
| usql | ユニバーサルSQLクライアント |
| redis-cli | Redis CLI |
| sqlite3 | SQLite |

### ネットワーク / API
| コマンド | 用途 |
|----------|------|
| `http` → xh | HTTPクライアント (HTTPie互換, Rust) |
| websocat | WebSocketクライアント |
| grpcurl | gRPCリクエストテスト |
| bruno | APIクライアントTUI (Postman代替) |
| bandwhich | ネットワーク帯域モニタ |
| termscp | SCP/SFTP (TUI, プログレスバー) |

### システム / ユーティリティ
| コマンド | 用途 |
|----------|------|
| `pss` → procs | モダンps |
| btop | システムモニタ (CPU/RAM/IO) |
| htop | プロセスモニタ |
| dust | ディレクトリ容量 |
| duf | ディスク使用量 |
| `csv` → csvlens | CSVビューア |
| pandoc | ドキュメント変換 |
| ouch | 圧縮/展開 (tar/zip/gz/xz/bz2対応) |
| tokei | コード行数カウント |
| glow | Markdownビューア |
| navi | インタラクティブチートシート |
| tealdeer | tldr (コマンド用例) |

### 開発ツール
| コマンド | 用途 |
|----------|------|
| direnv | ディレクトリ別環境変数 (nix-direnv統合) |
| mise | ランタイム管理 (node lts) |
| just | タスクランナー |
| tmux | 端末マルチプレクサ |
| watchexec | ファイル監視 + コマンド実行 |
| hyperfine | ベンチマーク |
| `br` → broot | ツリーファイラ |
| yazi | 端末ファイラ |
| sd | 置換 (sed代替) |
| watchman | ファイル監視デーモン |
| cocoapods | iOS依存管理 (pod) |

### 言語ランタイム
| ツール | 管理方法 |
|--------|----------|
| rustup (Rust) | Nix (cargo/rustc/rust-analyzer/clippy) |
| python3 | Nix |
| ruby | Nix |
| vim | Nix ($EDITOR fallback) |
| node (LTS) | mise |
| Flutter/Dart | fvm ($HOME/fvm, PATH設定済み) |
| bun | $HOME/.bun (PATH設定済み) |

### 補完 (zsh completions)
gh / kubectl / helm / kind / rustup / mise / just / docker / colima + zsh-completions (追加定義)

## 日常利用

### シェル操作

```bash
z ~/project          # zoxide: 曖昧パスで高速cd (履歴から学習)
zi                   # zoxide: fzfで対話的cd
Ctrl+R               # atuin: 履歴をfuzzy検索 (↑キーもatuin化)

..                   # AUTO_CD: cd .. と同義
~/Downloads          # AUTO_CD: ディレクトリ名だけでcd

ll                   # eza: 詳細ファイル一覧 (git status付き)
tree -L 2            # eza: 2階層ツリー表示
cat config.toml      # bat: シンタックスハイライト + 行番号

fdx compose          # fd: ファイル名を高速検索
rg "func Main"       # ripgrep: ファイル中身を高速検索
rg -l "TODO"         # ripgrep: マッチしたファイル名のみ表示
```

### ファイル操作

```bash
# ディレクトリをfzfで選んで移動
cdf

# ファイルをfzfで選んでエディタで開く
fv

# アーカイブ展開 (tar.gz/zip/bz2/xz対応)
extract archive.tar.gz
extract photos.zip

# ディレクトリ容量確認
dust                # 現在のディレクトリ
dust -d 2           # 深さ2まで

# 安全に削除 (ゴミ箱へ)
trash old-file.txt

# ディスク使用量
duf                 # 全マウントポイント一覧
duu                 # gdu: duの高速代替
```

### Git ワークフロー

```bash
lg                   # lazygit起動 (TUIで全git操作)

# lazygit内キー:
#   Space = stage/unstage,  c = commit,  P = push
#   / = 検索,  q = 終了,  ? = ヘルプ
```

```bash
# コマンドライン操作
gs                   # git status (短縮)
gc -m "fix: bug"     # git commit
gp                   # git push
gl                   # git pull
gd                   # git diff
gco main             # git checkout

git undo             # 直前のコミット取消 (変更は保持)
git amend            # 直前のコミットに追記
git ds               # ステージ済みの差分確認
git lg               # 全ブランチのグラフ付きログ
git who              # コントリビュータ一覧

# fzfでブランチ選択 → checkout
gcb

# fzfでgit log検索 → コミット内容表示
glog

# fzfでstash一覧 → pop
gshow

# diff表示 (deltaでside-by-side表示)
git diff HEAD~3        # deltaがpagerとしてside-by-side表示
diffs main feature     # difftastic: ブランチ間の構造ベースdiff
```

### コンテナ / K8s

```bash
# コンテナ起動
col start            # colima起動
dc up -d             # docker compose起動
dc ps                # コンテナ一覧
dc logs -f           # ログ追跡
ldz                  # lazydocker (TUIで管理)

# K8s操作
k get pods           # kubectl (abbr k → 展開後)
k get pods -w        # Pod監視
k logs -f deploy/app # ログ追跡
h list               # helm (abbr h → 展開後)
kind create cluster  # ローカルk8sクラスタ作成
```

### データベース

```bash
# PostgreSQL (補完・ハイライト付き)
pgcli postgres://localhost:5432/mydb

# MySQL
mycli -h localhost -u root

# ユニバーサルSQL (1つのCLIで全DB対応)
usql pg://localhost/mydb
usql my://localhost/mydb

# Redis
redis-cli

# SQLite
sqlite3 database.db
```

### API テスト

```bash
# HTTPリクエスト
http get https://api.example.com/users
http post https://api.example.com/users name=John

# WebSocket
websocat ws://localhost:8080/ws

# gRPC
grpcurl -plaintext localhost:50051 list
grpcurl -plaintext localhost:50051 describe

# APIクライアントTUI
bruno
```

### 開発ワークフロー

```bash
# ディレクトリ移動時に.envrcを自動評価
cd ~/project          # direnvが.envrcを読み込み

# ランタイム管理
mise ls               # インストール済みランタイム一覧
node --version        # mise管理のNode.js

# タスクランナー
just                  # 利用可能なレシピ一覧
just lint             # lint実行
just test             # テスト実行

# ファイル監視 + 自動実行
watchexec -e go -- go test ./...

# ベンチマーク
hyperfine 'my-command'

# コマンド例を調べる
tldr tar              # tealdeer: 実用例を表示
navi                  # インタラクティブチートシート

# Markdown表示
glow README.md

# コード行数カウント
tokei

# ネットワーク帯域
bandwhich

# ファイル転送 (SCP with progress)
termscp user@host:/path/file ./

# プロセス確認
pss                   # procs: モダンps
htop                  # 対話的プロセスモニタ

# テキスト操作
sd 'old' 'new' file   # sedライクな置換
pandoc README.md -o README.pdf  # ドキュメント変換

# CSV表示
csv data.csv          # csvlens: スクロール・検索付きビューア

# 圧縮/展開
ouch compress file.tar.gz dir/
ouch decompress archive.zip
```

### tmux (VPS/リモート)

```bash
tmux new -s work      # セッション開始

# tmux内:
#   C-a |    左右分割
#   C-a -    上下分割
#   C-a h/j/k/l  ペイン移動
#   C-a c    新規ウィンドウ
#   C-a v    コピーモード (v=選択開始, y=コピー)
#   C-a d    デタッチ

tmux attach -t work   # セッション再開
```

### broot / yazi (ファイラ)

```bash
br                    # broot: ツリー型ファイラ
#   e = エディタで開く
#   gd = lazygit起動
#   Ctrl+T = シェル起動
#   Alt+H = 隠しファイル切替

yazi                  # yazi: 3ペイン型ファイラ
#   j/k = 移動,  l = 開く,  h = 戻る
#   Space = 選択,  y = コピー,  p = 貼り付け
#   q = 終了
```

### nix develop (一時開発環境)

```bash
nix develop            # 全部入り開発環境
nix develop .#go       # Go専用 (gopls + golangci-lint + delve)
nix develop .#python   # Python専用 (uv + ruff + pyright)
nix develop .#node     # Node.js専用 (pnpm + bun + tsc)
nix develop .#rust     # Rust専用 (cargo + rust-analyzer + clippy)
nix develop .#k8s      # K8s専用 (kubectl + helm + kind + k9s)
nix develop .#db       # DB専用 (pgcli + mycli + usql + redis + sqlite)
```

## devShells

`nix develop .#<name>` で一時的な開発環境を起動:

| shell | 内容 |
|-------|------|
| `default` | Go + Python + Node + Rust + Git + DB + K8s 全部入り |
| `go` | Go + gopls + golangci-lint + delve + protoc |
| `python` | Python + uv + ruff + pyright |
| `node` | Node.js 24 + pnpm + bun + TypeScript LSP |
| `rust` | Rust + cargo + rust-analyzer + clippy |
| `k8s` | kubectl + helm + kind + k9s + docker + colima |
| `db` | pgcli + mycli + usql + redis + sqlite + protobuf |

## キーバインド

### WezTerm

| 操作 | キー |
|------|------|
| ペイン分割 (左右) | `Cmd+Shift+←` |
| ペイン分割 (上下) | `Cmd+Shift+↑` |
| ペイン移動 | `Ctrl+A` → `h/j/k/l` |
| ペインリサイズ | `Ctrl+A` → `Shift+H/J/K/L` |
| ペインズーム | `Cmd+Shift+Z` |
| ペイン閉じる | `Cmd+W` |
| 新規タブ | `Cmd+T` |
| タブ移動 | `Cmd+1..9` |
| 検索 | `Cmd+Shift+F` |
| コマンドパレット | `Cmd+Shift+K` |
| quick_select (URL/パス) | `Ctrl+Shift+Space` |
| フルスクリーン | `Cmd+Enter` |
| 設定再読込 | `Ctrl+Shift+R` |
| tmuxにC-a送信 | `Ctrl+A` → `A` |

### tmux (VPS/リモート用)

| 操作 | キー |
|------|------|
| prefix | `Ctrl+A` |
| ペイン分割 (横) | `prefix` → `\|` |
| ペイン分割 (縦) | `prefix` → `-` |
| ペイン移動 | `prefix` → `h/j/k/l` |
| ペインリサイズ | `prefix` → `Shift+H/J/K/L` |
| 新規ウィンドウ | `prefix` → `c` |
| コピーモード | `prefix` → `v` (選択), `y` (コピー) |
| 設定再読込 | `prefix` → `r` |

## コンテナ (colima)

```bash
colima start      # VM起動 (初回のみ)
docker ps         # 通常通り使用可能
lzd               # lazydocker
colima stop       # 停止
```

Docker Desktop が既存でも共存可能 (同時起動は避ける)。

## 運用

```bash
cd "$HOME/nix_setting"
git pull && ./bootstrap.sh    # 設定更新
nix fmt                        # Nixファイル整形
ls -d ~/hm-bak*               # バックアップ確認
```

## カスタマイズ

### パッケージの追加/削除

`home.nix` の `home.packages` に追記するだけ:

```nix
home.packages = with pkgs; [
  # ... 既存パッケージ ...
  ripgrep         # 追加
  # zsh-abbr      # 削除 (コメントアウト)
];
```

```bash
./bootstrap.sh     # 再適用
```

### 設定ファイルの変更

編集後 `Ctrl+Shift+R` (WezTerm) または `prefix + r` (tmux) で即反映。主要な設定ファイル:

| ファイル | 用途 | 編集後に必要な操作 |
|----------|------|-------------------|
| `config/zsh/zshrc` | エイリアス・関数・PATH | `exec zsh` または再ログイン |
| `config/starship/starship.toml` | プロンプト表示 | 自動反映 |
| `config/wezterm/wezterm.lua` | ターミナル外観・キー | `Ctrl+Shift+R` |
| `config/tmux/tmux.conf` | tmux 設定 | `prefix + r` |
| `config/git/gitconfig` | Git エイリアス | 自動反映 |
| `config/lazygit/config.yml` | lazygit 表示 | 自動反映 |

### エイリアス・キーバインドの変更

エイリアスは `home.nix` の `zshInitExtra` セクションで管理。キーバインドは `config/wezterm/wezterm.lua` の `config.keys`。

```lua
-- wezterm: カスタムキーバインド追加例
{
  key = 'N',
  mods = 'CMD',
  action = act.SpawnCommandInNewTab {
    cwd = wezterm.home_dir,
    args = { 'btop' },
  },
},
```

```zsh
# zsh: カスタムエイリアス追加例 (home.nix の zshInitExtra 内)
alias k9s='k9s --namespace default'
```

### テーマ変更

| ツール | 設定場所 | 例 |
|--------|----------|----|
| WezTerm | `config/wezterm/wezterm.lua:6-27` | `colors` テーブル (Lamy Snow Night) |
| Starship | `config/starship/starship.toml` | 各モジュールの `style` |
| tmux | `config/tmux/tmux.conf:54-65` | `status-style`, `pane-border-style` |
| bat | `home.nix:239-241` | `config.theme = "Dracula"` |
| lazygit | `config/lazygit/config.yml:6-16` | `gui.theme` の色コード |
| broot | `config/broot/conf.toml:19-38` | `[skin]` セクション |

### devShell の追加

`flake.nix` の `devShells.${userOptions.system}` にブロックを追加:

```nix
myLang = pkgs.mkShell {
  packages = with pkgs; [ myTool myLsp ];
  shellHook = ''
    echo "ready"
  '';
};
```

```bash
nix develop .#myLang
```

### マシン固有設定

public repo にコミットしたくないマシン固有の設定は以下の方法で管理:

```bash
# 1. user-options.nix (bootstrap.sh が自動生成)
# 2. ~/.gitconfig.local (個人Git設定)
# 3. machine-local.nix を作成して home.nix に import

# home.nix 先頭に追加:
# imports = [ (if builtins.pathExists ./machine-local.nix
#              then ./machine-local.nix else {}) ];
```

```nix
# machine-local.nix (gitignore 対象):
{ pkgs, ... }: {
  home.packages = with pkgs; [ firefox slack ];
}
```

## justfile テンプレート

`~/.config/just/justfile` にプロジェクト用テンプレートが配置される。新規プロジェクトで `just` を実行すると利用可能なレシピが表示される。

```bash
# プロジェクトにコピーして使う
cp ~/.config/just/justfile ./justfile
just                  # レシピ一覧
just lint             # lint
just test             # test
just docker-up        # docker compose up -d
```

## トラブルシューティング

```bash
# Nix がインストールされているか
command -v nix

# flake が正しいか
nix flake check

# Home Manager のバックアップ確認
ls -d ~/hm-bak*

# バックアップから復元
rm ~/.zshrc ~/.gitconfig
cp ~/hm-bak-*/home.nix.backup ~/...

# Nix ストアのクリーンアップ
nix-store --gc

# WezTerm 設定のエラー確認
wezterm cli list

# tmux 設定のエラー確認
tmux source-file ~/.config/tmux/tmux.conf
```

## atuin 履歴同期

```bash
atuin login          # 初回のみ (atuin.sh アカウント)
atuin sync           # 手動同期
atuin status         # 同期状態確認
```

設定では `sync.records = true` が有効。`Ctrl+R` で全端末の履歴を unified 検索。

## CI

push/PR 時に GitHub Actions が以下を実行:
- `nix flake check` — Nix 構文・型チェック
- secret scan — トークン/SSH鍵の誤コミット検出
- image scan — 著作権画像の誤コミット検出

## Public repo policy

public repo に含めない:
- 背景画像 (著作権)
- SSH秘密鍵 / APIキー / Token
- `.env` / 個人情報 (`.gitconfig.local` に分離)
- `user-options.nix` (`bootstrap.sh` が自動生成)
