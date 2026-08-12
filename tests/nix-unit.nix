{
  testUsername = {
    expr = (import ../user-options/options.nix).username;
    expected = "lamy210";
  };

  testMacbookAirHome = {
    expr = "/Users/" + (import ../user-options/options.nix).username;
    expected = "/Users/lamy210";
  };

  testLinuxHome = {
    expr = "/home/" + (import ../user-options/options.nix).username;
    expected = "/home/lamy210";
  };
}
