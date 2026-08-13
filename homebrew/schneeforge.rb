class Schneeforge < Formula
  desc "Declarative Developer Workstation Manager (Nix + Home Manager + nix-darwin)"
  homepage "https://github.com/Lamy210/nix_setting"
  url "https://github.com/Lamy210/nix_setting/releases/download/v0.1.0/schneeforge-aarch64-darwin"
  version "0.1.0"
  sha256 "d7029c195866a9ba679d515a0d2bafac2b5b13a228fe33d8ce71d226978e5880"
  license "MIT"

  depends_on macos: :sonoma

  def install
    bin.install "schneeforge-aarch64-darwin" => "schneeforge"
  end

  test do
    assert_match "Declarative Developer Workstation Manager", shell_output("#{bin}/schneeforge --help")
  end
end
