_: {
  nix = {
    enable = true;
    settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      auto-optimise-store = true;
      keep-outputs = true;
      keep-derivations = true;
    };
    gc = {
      automatic = true;
      interval = {
        Weekday = 0;
        Hour = 3;
        Minute = 0;
      };
      options = "--delete-older-than 30d";
    };
  };
}
