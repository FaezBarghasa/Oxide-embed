class OxideEmbed < Formula
  desc "Local-first code graph indexing, cognitive memory, and context hygiene engine"
  homepage "https://github.com/FaezBarghasa/Oxide-embed"
  version "0.4.0"
  license "Apache-2.0 or MIT"

  if OS.mac?
    url "https://github.com/FaezBarghasa/Oxide-embed/releases/download/v#{version}/oxide-embed-v#{version}-universal-apple-darwin.tar.gz"
  end

  def install
    bin.install "bin/oxide-embed"
  end

  service do
    run [opt_bin/"oxide-embed", "watch"]
    keep_alive true
    log_path var/"log/oxide-embed.log"
    error_log_path var/"log/oxide-embed.log"
    environment_variables RUST_LOG: "info"
  end

  test do
    system "#{bin}/oxide-embed", "--version"
  end
end
