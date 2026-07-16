local wezterm = require 'wezterm'
local act = wezterm.action

local config = wezterm.config_builder()

-- Lamy Snow Night
config.colors = {
  foreground = '#EAF6FF',
  background = '#101827',
  cursor_bg = '#A8D6F5',
  cursor_fg = '#101827',
  selection_bg = '#315078',
  selection_fg = '#FDFCFA',
  ansi = {
    '#101827', '#D97888', '#69BFAF', '#C58B44',
    '#5C8EDB', '#A995B5', '#75AEE5', '#DCEAF4',
  },
  brights = {
    '#52657D', '#F3A9B5', '#A6E8D8', '#E7B96D',
    '#A8D6F5', '#D5C9DA', '#CBEAF7', '#FDFCFA',
  },
  tab_bar = {
    background = '#17243A',
    active_tab = { bg_color = '#263B5B', fg_color = '#A8D6F5' },
    inactive_tab = { bg_color = '#17243A', fg_color = '#61748E' },
    inactive_tab_hover = { bg_color = '#263B5B', fg_color = '#CBEAF7' },
  },
  compose_cursor = '#C58B44',
  split = '#4367AA',
}

config.window_background_opacity = 0.94
config.macos_window_background_blur = 30

config.window_padding = {
  left = 10,
  right = 10,
  top = 8,
  bottom = 8,
}

config.inactive_pane_hsb = {
  saturation = 0.75,
  brightness = 0.75,
}

config.font = wezterm.font_with_fallback({
  'PlemolJP Console NF',
  'JetBrainsMono Nerd Font',
  'Apple Color Emoji',
})

config.font_size = 14.0
config.line_height = 1.08

config.hide_tab_bar_if_only_one_tab = false
config.use_fancy_tab_bar = false
config.tab_bar_at_bottom = false
config.window_decorations = 'RESIZE'
config.scrollback_lines = 30000
config.enable_scroll_bar = true

config.quick_select_patterns = {
  '[0-9a-f]{7,40}',
  'https?://\\S+',
  '[/~][.\\w/-]+',
}

local bg_path = wezterm.home_dir .. '/.config/wezterm/backgrounds/Yukihana.Lamy.jpg'

if wezterm.glob(bg_path)[1] ~= nil then
  config.background = {
    {
      source = { File = bg_path },
      width = '100%',
      height = '100%',
      opacity = 0.14,
      hsb = {
        brightness = 0.32,
        saturation = 0.55,
      },
    },
    {
      source = { Color = '#101827' },
      width = '100%',
      height = '100%',
      opacity = 0.78,
    },
  }
end

config.leader = {
  key = 'a',
  mods = 'CTRL',
  timeout_milliseconds = 1000,
}

config.keys = {
  {
    key = 'LeftArrow',
    mods = 'CMD|SHIFT',
    action = act.SplitHorizontal { domain = 'CurrentPaneDomain' },
  },
  {
    key = 'UpArrow',
    mods = 'CMD|SHIFT',
    action = act.SplitVertical { domain = 'CurrentPaneDomain' },
  },

  { key = 'h', mods = 'LEADER', action = act.ActivatePaneDirection 'Left' },
  { key = 'j', mods = 'LEADER', action = act.ActivatePaneDirection 'Down' },
  { key = 'k', mods = 'LEADER', action = act.ActivatePaneDirection 'Up' },
  { key = 'l', mods = 'LEADER', action = act.ActivatePaneDirection 'Right' },

  { key = 'H', mods = 'LEADER|SHIFT', action = act.AdjustPaneSize { 'Left', 5 } },
  { key = 'J', mods = 'LEADER|SHIFT', action = act.AdjustPaneSize { 'Down', 5 } },
  { key = 'K', mods = 'LEADER|SHIFT', action = act.AdjustPaneSize { 'Up', 5 } },
  { key = 'L', mods = 'LEADER|SHIFT', action = act.AdjustPaneSize { 'Right', 5 } },

  {
    key = 'a',
    mods = 'LEADER',
    action = act.SendKey { key = 'a', mods = 'CTRL' },
  },
  {
    key = 'Z',
    mods = 'CMD|SHIFT',
    action = act.TogglePaneZoomState,
  },
  {
    key = 'w',
    mods = 'CMD',
    action = act.CloseCurrentPane { confirm = true },
  },
  {
    key = 'F',
    mods = 'CMD|SHIFT',
    action = act.Search { CaseSensitiveString = '' },
  },
  {
    key = 'K',
    mods = 'CMD|SHIFT',
    action = act.ActivateCommandPalette,
  },
  {
    key = '[',
    mods = 'LEADER',
    action = act.ActivateCopyMode,
  },
  {
    key = 't',
    mods = 'CMD',
    action = act.SpawnTab 'CurrentPaneDomain',
  },
  {
    key = 'Enter',
    mods = 'CMD',
    action = act.ToggleFullScreen,
  },
  {
    key = 'R',
    mods = 'CTRL|SHIFT',
    action = act.ReloadConfiguration,
  },
}

for i = 1, 9 do
  table.insert(config.keys, {
    key = tostring(i),
    mods = 'CMD',
    action = act.ActivateTab(i - 1),
  })
end

return config
