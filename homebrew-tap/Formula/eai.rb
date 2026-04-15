class Eai < Formula
  desc "Natural language to shell commands"
  homepage "https://github.com/GITHUB_REPOSITORY"
  version "VERSION_PLACEHOLDER"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/GITHUB_REPOSITORY/releases/download/v#{version}/eai-darwin-arm64.tar.gz"
      sha256 "SHA_DARWIN_ARM64"
    else
      url "https://github.com/GITHUB_REPOSITORY/releases/download/v#{version}/eai-darwin-amd64.tar.gz"
      sha256 "SHA_DARWIN_AMD64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/GITHUB_REPOSITORY/releases/download/v#{version}/eai-linux-arm64.tar.gz"
      sha256 "SHA_LINUX_ARM64"
    else
      url "https://github.com/GITHUB_REPOSITORY/releases/download/v#{version}/eai-linux-amd64.tar.gz"
      sha256 "SHA_LINUX_AMD64"
    end
  end

  def install
    bin.install "eai"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/eai --version")
  end
end
