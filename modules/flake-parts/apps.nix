_: {
  perSystem =
    { config, pkgs, ... }:
    let
      schneeforge = config.packages.schneeforge;

      app =
        subcommand:
        pkgs.writeShellApplication {
          name = subcommand;
          runtimeInputs = [ schneeforge ];
          text = "exec schneeforge ${subcommand}";
        };
    in
    {
      apps = {
        doctor = {
          type = "app";
          program = "${app "doctor"}/bin/doctor";
          meta.description = "Diagnose system / Nix / host compatibility";
        };
        apply = {
          type = "app";
          program = "${app "apply"}/bin/apply";
          meta.description = "Detect host and apply configuration (switch)";
        };
        status = {
          type = "app";
          program = "${app "status"}/bin/status";
          meta.description = "Show current host and configuration status";
        };
        rollback = {
          type = "app";
          program = "${app "rollback"}/bin/rollback";
          meta.description = "Rollback to previous generation";
        };
        verify = {
          type = "app";
          program = "${app "verify"}/bin/verify";
          meta.description = "Verify environment after install";
        };
      };
    };
}
