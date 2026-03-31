class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  version "1.5.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "55b8af5eb18464464f4696af5b1c4fb134d9f89f3c9e2e0927004d064d70c061"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "59621f594400ae038c1f3a8cb0d82c3a04cc758173a56b577ea717cd48aa813c"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "8702f43b153e33c8924e949c2d383a51bdb5c8631b32dfe76efbbad842b616fb"
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
