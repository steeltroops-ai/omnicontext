class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents (MCP, semantic search, local-first)"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  # Version and SHA256s are auto-updated by the release workflow (update-manifests job)
  version "0.6.1"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "69b96f23546682d135ec30daf3a787907eb06b48cfa0e440f972eeaeee0d52de"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "f46d6027d6f3554d295e35bf1ebc5f2e14bc5acf7dd53b964da6eeeaf56ccda3"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "e75df3ff05a524bf292bb8f3064c1f82a7a61c0ff9fd357bb722966d3a5791b5"
  end

  def install
    bin.install "omnicontext"
    bin.install "omnicontext-mcp"
    bin.install "omnicontext-daemon" if File.exist?("omnicontext-daemon")
  end

  def caveats
    <<~EOS
      OmniContext requires the Jina AI embedding model (~550 MB) which is
      automatically downloaded on first use:

        omnicontext index /path/to/your/repo

      For MCP integration with AI clients (Claude, Cursor, Windsurf, etc.),
      add the following to your client's MCP config:

        {
          "mcpServers": {
            "omnicontext": {
              "command": "#{bin}/omnicontext-mcp",
              "args": ["--repo", "/path/to/your/repo"]
            }
          }
        }

      Documentation: https://github.com/steeltroops-ai/omnicontext
    EOS
  end

  test do
    assert_match "omnicontext", shell_output("#{bin}/omnicontext --version")
  end
end
