class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  version "1.3.2"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "ae16592b6e2326c921c4e4d4a72e34bc396c191b9fd055b6f0fdc9a6d48f3a33"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "9e4b31ba4fb533249dfbf5c9175e45fe4a833f8690c7a2b81a658d7b7d3ac817"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "748d96de09507d38bd17df7f6d731f98142de89cd69efcc4f5a9861a38c13489"
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
