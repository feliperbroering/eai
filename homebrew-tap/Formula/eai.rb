class Eai < Formula
  desc "Natural language to shell commands"
  homepage "https://github.com/GITHUB_REPOSITORY"
  version "VERSION_PLACEHOLDER"
  license "MIT"

  bottle do
    root_url "https://github.com/GITHUB_REPOSITORY/releases/download/vVERSION_PLACEHOLDER"
    sha256 cellar: :any_skip_relocation, arm64_tahoe:   "BOTTLE_SHA_ARM64_TAHOE"
    sha256 cellar: :any_skip_relocation, arm64_sequoia: "BOTTLE_SHA_ARM64_SEQUOIA"
    sha256 cellar: :any_skip_relocation, arm64_sonoma:  "BOTTLE_SHA_ARM64_SONOMA"
    sha256 cellar: :any_skip_relocation, arm64_ventura: "BOTTLE_SHA_ARM64_VENTURA"
    sha256 cellar: :any_skip_relocation, tahoe:         "BOTTLE_SHA_TAHOE"
    sha256 cellar: :any_skip_relocation, sequoia:       "BOTTLE_SHA_SEQUOIA"
    sha256 cellar: :any_skip_relocation, sonoma:        "BOTTLE_SHA_SONOMA"
    sha256 cellar: :any_skip_relocation, ventura:       "BOTTLE_SHA_VENTURA"
    sha256 cellar: :any_skip_relocation, x86_64_linux:  "BOTTLE_SHA_X86_64_LINUX"
  end

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
