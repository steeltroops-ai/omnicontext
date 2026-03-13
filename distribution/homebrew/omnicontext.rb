class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  version "1.2.3"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "2463f6f18cff093d7a8ac2683128406317c676970243e61c11b6769e92f37f44"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "e3454e06fa4a45c34457d7b102acb544a26a642dd71e036f2378ccaee3ebd004"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "135f5a02c5951a99ce62cf83745714ad24a98db36f883e212b4fecec0d79c01f"
  end

  # Pre-built binary — no compilation needed
  bottle :unneeded

  def install
    bin.install "omnicontext"
    bin.install "omnicontext-mcp"
    bin.install "omnicontext-daemon" if File.exist?("omnicontext-daemon")
  end

  def post_install
    # Auto-configure MCP for all detected AI clients
    system "#{bin}/omnicontext", "setup", "--all" rescue nil
  end

  def caveats
    <<~EOS
      To configure OmniContext for all your AI coding clients, run:
        omnicontext setup --all

      Quick start:
        cd /path/to/your/project
        omnicontext index .
        omnicontext search "your query"
    EOS
  end

  test do
    assert_match "omnicontext", shell_output("#{bin}/omnicontext --version")
  end
end
