# GitHub-hosted Ubuntu ARM64 evaluation fixture. Not a real user account.
# No installation or Home Manager switch is performed by this canary.
{
  username = "runner";
  homeDirectory = "/home/runner";
  system = "aarch64-linux";
  hostname = "schneeforge-ci-arm64";
}
