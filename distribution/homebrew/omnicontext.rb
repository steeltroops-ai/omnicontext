class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  version "1.2.1"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "88735ce64829c62d1439388420536d53ebff0058a9dd1dd80d7471ff50038d54"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "c29d25806b743e3bbeab69bb39262e77a999afef31e92775f65db3b90d82a4a1"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "d5dac74de1314dba34a8247b33bb5f6a943d113d63e2467e9a8feb0eb54aa3a5"
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
