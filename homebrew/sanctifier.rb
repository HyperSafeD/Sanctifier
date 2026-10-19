class Sanctifier < Formula
  desc "Security copilot for Stellar Soroban smart contracts"
  homepage "https://github.com/HyperSafeD/Sanctifier"
  license "MIT OR Apache-2.0"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/HyperSafeD/Sanctifier/releases/download/v#{version}/sanctifier-macos-arm64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    else
      url "https://github.com/HyperSafeD/Sanctifier/releases/download/v#{version}/sanctifier-macos-amd64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    url "https://github.com/HyperSafeD/Sanctifier/releases/download/v#{version}/sanctifier-linux-amd64-musl"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  end

  def install
    bin.install "sanctifier-macos-arm64" => "sanctifier" if OS.mac? && Hardware::CPU.arm?
    bin.install "sanctifier-macos-amd64" => "sanctifier" if OS.mac? && Hardware::CPU.intel?
    bin.install "sanctifier-linux-amd64-musl" => "sanctifier" if OS.linux?
  end

  test do
    system "#{bin}/sanctifier", "--version"
  end
end
