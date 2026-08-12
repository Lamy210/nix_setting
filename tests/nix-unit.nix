{
  testUsername = {
    expr = (import ../user-options/options.nix).username;
    expected = "lamy210";
  };

  testMacbookAirHome = {
    expr = (import ../hosts/macbook-air/options.nix).homeDirectory;
    expected = "/Users/lamy210";
  };

  testLinuxHome = {
    expr = (import ../hosts/linux-generic/options.nix).homeDirectory;
    expected = "/home/lamy210";
  };
}
