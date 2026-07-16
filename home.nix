{ pkgs, ... }:

let
  zshInitExtra = ''
    bindkey -e
    bindkey '^[[1;5D' backward-word
    bindkey '^[[1;5C' forward-word
    bindkey '^[[Z' reverse-menu-complete

    eval "$(mise activate zsh)"

    if command -v eza >/dev/null 2>&1; then
      alias ls='eza --icons --group-directories-first'
      alias ll='eza -la --icons --git --group-directories-first'
      alias la='eza -a --icons --group-directories-first'
      alias tree='eza --tree --icons --group-directories-first'
    fi

    command -v bat >/dev/null 2>&1 && alias cat='bat'
    command -v rg >/dev/null 2>&1 && alias grep='rg'
    command -v fd >/dev/null 2>&1 && alias fdx='fd'

    alias lg='lazygit'
    alias lzd='lazydocker'
    alias g='git'
    alias gs='git status'
    alias gb='git branch'
    alias gc='git commit'
    alias gp='git push'
    alias gl='git pull'
    alias gd='git diff'
    alias gco='git checkout'
    alias gst='git stash'
    alias grb='git rebase'
    alias dc='docker compose'
    alias col='colima'

    command -v difft >/dev/null 2>&1 && alias diffs='difft'
    command -v procs >/dev/null 2>&1 && alias pss='procs'
    command -v xh >/dev/null 2>&1 && alias http='xh'
    command -v gdu >/dev/null 2>&1 && alias duu='gdu'
    command -v csvlens >/dev/null 2>&1 && alias csv='csvlens'
    command -v broot >/dev/null 2>&1 && alias br='broot'
    command -v trashy >/dev/null 2>&1 && alias trash='trashy put'

    if command -v abbr >/dev/null 2>&1; then
      abbr k="kubectl"
      abbr h="helm"
      abbr tf="terraform"
    fi

    mkcd() {
      mkdir -p "$1" && cd "$1"
    }

    extract() {
      if command -v ouch >/dev/null 2>&1; then
        ouch decompress "$@"
      elif command -v unzip >/dev/null 2>&1; then
        case "$1" in
          *.tar.gz|*.tgz) tar xzf "$1" ;;
          *.tar.bz2|*.tbz2) tar xjf "$1" ;;
          *.tar.xz) tar xJf "$1" ;;
          *.tar) tar xf "$1" ;;
          *.gz) gunzip "$1" ;;
          *.zip) unzip "$1" ;;
          *.rar) echo "use ouch or unrar" ;;
          *) echo "unknown format: $1" ;;
        esac
      fi
    }

    cdf() {
      local dir
      dir=$(fd --type d --hidden --exclude .git | fzf) && cd "$dir"
    }

    fv() {
      local file
      file=$(fd --type f --hidden --exclude .git | fzf) && ''${EDITOR:-vim} "$file"
    }

    if command -v fzf >/dev/null 2>&1 && command -v git >/dev/null 2>&1; then
      gcb() {
        local branch
        branch=$(git branch --all --format='%(refname:short)' | \
          sed 's|^origin/||' | sort -u | \
          fzf --preview 'git log --oneline --graph --color=always {} 2>/dev/null || true') && \
          git checkout "$branch"
      }

      glog() {
        git log --oneline --graph --all --color=always | \
          fzf --ansi --preview 'echo {} | grep -o "^[a-f0-9]\{7,\}" | head -1 | xargs -I{} git show --color=always {}' \
          --no-sort --bind 'ctrl-s:toggle-sort'
      }

      gshow() {
        git stash list | fzf --preview 'echo {} | cut -d: -f1 | xargs git stash show -p --color=always' | \
          cut -d: -f1 | xargs git stash pop
      }
    fi

    export PATH="$HOME/.local/bin:$PATH"
    export PATH="$PATH:$HOME/fvm/default/bin"

    [ -s "$HOME/.bun/_bun" ] && source "$HOME/.bun/_bun"
    export BUN_INSTALL="$HOME/.bun"
    export PATH="$BUN_INSTALL/bin:$PATH"

    if command -v gh >/dev/null 2>&1; then eval "$(gh completion -s zsh)"; fi
    if command -v kubectl >/dev/null 2>&1; then source <(kubectl completion zsh) 2>/dev/null; fi
    if command -v helm >/dev/null 2>&1; then source <(helm completion zsh) 2>/dev/null; fi
    if command -v kind >/dev/null 2>&1; then source <(kind completion zsh) 2>/dev/null; fi
    if command -v rustup >/dev/null 2>&1; then source <(rustup completions zsh) 2>/dev/null; fi
    if command -v mise >/dev/null 2>&1; then eval "$(mise completion zsh)"; fi
    if command -v just >/dev/null 2>&1; then source <(just --completions zsh) 2>/dev/null; fi
    if command -v docker >/dev/null 2>&1; then source <(docker completion zsh) 2>/dev/null; fi
    if command -v colima >/dev/null 2>&1; then source <(colima completion zsh) 2>/dev/null; fi
  '';
in
{
  home.stateVersion = "24.11";

  programs.home-manager.enable = true;

  home.packages = with pkgs; [
    zsh
    git
    curl
    wget
    unzip

    starship
    fzf
    eza
    bat
    fd
    ripgrep
    zoxide
    atuin

    gh
    ghq
    lazygit
    delta
    difftastic

    yazi
    btop
    dust
    duf
    gdu
    jq
    yq
    fx
    csvlens
    just
    direnv
    mise
    tmux
    hyperfine
    tealdeer
    sd
    broot
    watchexec
    xh
    procs
    tokei
    glow
    trashy
    lazydocker
    navi
    rustup
    htop
    watchman
    ruby
    python3
    cocoapods

    colima
    docker
    docker-compose
    kubectl
    helm
    kind
    pgcli
    mycli
    usql
    redis
    sqlite
    protobuf
    grpcurl
    websocat
    bruno
    pandoc
    ouch
    bandwhich
    termscp
    zsh-abbr
    vim
    terraform
    zsh-completions
  ];

  programs.zsh = {
    enable = true;
    autosuggestion.enable = true;
    syntaxHighlighting.enable = true;
    shellOptions = [
      "AUTO_CD"
      "CORRECT"
      "HIST_IGNORE_DUPS"
      "HIST_IGNORE_SPACE"
    ];
    initExtra = zshInitExtra;
    initExtraBeforeCompInit = ''
      [ -f "${pkgs.zsh-abbr}/share/zsh-abbr/zsh-abbr.zsh" ] && source "${pkgs.zsh-abbr}/share/zsh-abbr/zsh-abbr.zsh"
    '';
  };

  programs.starship = {
    enable = true;
    settings = builtins.fromTOML (builtins.readFile ./config/starship/starship.toml);
  };

  programs.fzf.enable = true;

  programs.direnv = {
    enable = true;
    nix-direnv.enable = true;
    config = {
      global = {
        load_dotenv = false;
        strict_env = true;
        warn_timeout = "5s";
      };
    };
  };

  programs.zoxide.enable = true;

  programs.atuin = {
    enable = true;
    settings = {
      enter_accept = true;
      search_mode = "fuzzy";
      filter_mode = "global";
      workspaces = true;
      show_preview = true;
      style = "compact";
      inline_height = 40;
      sync.records = true;
      keys = { scroll_exits = false; };
      search = { filters = ["global", "host", "workspace", "directory"]; };
    };
  };

  programs.bat = {
    enable = true;
    config = {
      theme = "Catppuccin Mocha";
      style = "plain,header,changes";
    };
  };

  programs.tmux = {
    enable = true;
    extraConfig = builtins.readFile ./config/tmux/tmux.conf;
  };

  home.file.".gitconfig".source = ./config/git/gitconfig;

  xdg.configFile."wezterm/wezterm.lua".source = ./config/wezterm/wezterm.lua;

  xdg.configFile."mise/config.toml".source = ./config/mise/config.toml;

  xdg.configFile."openspec/config.json".source = ./config/openspec/config.json;

  xdg.configFile."lazygit/config.yml".source = ./config/lazygit/config.yml;

  xdg.configFile."yazi/yazi.toml".source = ./config/yazi/yazi.toml;

  xdg.configFile."broot/conf.toml".source = ./config/broot/conf.toml;

  xdg.configFile."just/justfile".source = ./config/just/justfile;
}
