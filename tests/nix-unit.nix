let
  manifest = builtins.fromTOML (builtins.readFile ../config.toml);
in
{
  testUsername = {
    expr = manifest.user.username;
    expected = "lamy210";
  };

  testMacbookAirHome = {
    expr = "/Users/" + manifest.user.username;
    expected = "/Users/lamy210";
  };

  testLinuxHome = {
    expr = "/home/" + manifest.user.username;
    expected = "/home/lamy210";
  };

  testSchemaVersion = {
    expr = manifest.schema;
    expected = 1;
  };
}
