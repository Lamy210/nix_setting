_:
let
  caskConfig = import ./cask.nix;
in
{
  homebrew = {
    enable = true;
    onActivation = {
      autoUpdate = false;
      upgrade = false;
      cleanup = "none";
    };
    caskArgs = {
      appdir = "~/Applications";
    };
    casks = caskConfig.installPackages;
    inherit (caskConfig) taps;
    brews = caskConfig.tapPackages;
    masApps = {
      Xcode = 497799835;
    };
  };
}
