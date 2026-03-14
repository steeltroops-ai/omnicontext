class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  version "1.3.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "57326dd36895a38cae6e031f04e343ffa350ab330161b7cad6051f9ca10b7a8f"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "b15a8478f97cfbfa80fc8523759bc395227efe133837d3bdb7ae5fdc851e1dac"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "83e4d80d1ea748bef9bc41b19c99e383a4aac10d28d8042e79d993bd82c99eed"
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
