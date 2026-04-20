class Patina < Formula
  desc "Patina AI + Mother CLI"
  homepage "https://github.com/NicabarNimble/patina"
  version "0.62.1"
  license "MIT"

  on_macos do
    url "https://github.com/NicabarNimble/patina/releases/download/v#{version}/patina-aarch64-apple-darwin.tar.gz"
    sha256 "REPLACE_WITH_MACOS_ARM64_SHA256"
  end

  on_linux do
    url "https://github.com/NicabarNimble/patina/releases/download/v#{version}/patina-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "REPLACE_WITH_LINUX_X64_SHA256"
  end

  def install
    if OS.mac? && Hardware::CPU.intel?
      odie "Patina Homebrew distribution currently supports Apple Silicon macOS only"
    end

    bin.install "patina"
  end

  service do
    run [opt_bin/"patina", "mother", "start"]
    keep_alive true
    log_path var/"log/patina-mother.log"
    error_log_path var/"log/patina-mother.error.log"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/patina --version")
    assert_match "mother", shell_output("#{bin}/patina mother --help")
  end
end
