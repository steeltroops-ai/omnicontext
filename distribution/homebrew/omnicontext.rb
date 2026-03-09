class Omnicontext < Formula
  desc "Universal code context engine for AI coding agents"
  homepage "https://github.com/steeltroops-ai/omnicontext"
  version "1.1.1"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "1d660a4946fe1499474084b60317ba93645570aa6189071ac8bc69457dec50dc"
    else
      url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "34fb1aff5a59021c936b288286cc10d440d2fad6b99d4df6d8a14200264633a7"
    end
  end

  on_linux do
    url "https://github.com/steeltroops-ai/omnicontext/releases/download/v#{version}/omnicontext-#{version}-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "868fec7f058d83283a8a961bdb3ab6164b9ae0fcfafd8f4f704912d25efbada6"
  end

  def install
    bin.install "omnicontext"
    bin.install "omnicontext-mcp"
    bin.install "omnicontext-daemon" if File.exist?("omnicontext-daemon")
  end

  test do
    assert_match "omnicontext", shell_output("#{bin}/omnicontext --version")
  end
end
