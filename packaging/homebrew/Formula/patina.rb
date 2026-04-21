class Patina < Formula
  desc "Patina AI + Mother CLI"
  homepage "https://github.com/NicabarNimble/patina"
  version "0.64.3"
  license "MIT"

  depends_on macos: :sonoma

  on_macos do
    url "https://github.com/NicabarNimble/patina/releases/download/v#{version}/patina-aarch64-apple-darwin.tar.gz"
    sha256 "6e5ca81f8ace8c354666e0fcd23f88db66376e2f99dd0aad9d32b0f961aa0b7e"
  end

  on_linux do
    url "https://github.com/NicabarNimble/patina/releases/download/v#{version}/patina-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "3850a589879d0e238f515daf713b1f644742afa37357b209ffb6cb6a36693de0"
  end

  def install
    if OS.mac? && Hardware::CPU.intel?
      odie "Patina requires macOS 14+ on Apple Silicon"
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
