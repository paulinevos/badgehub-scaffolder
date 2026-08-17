class Badgehub < Formula
  desc "Scaffold and manage BadgeHub apps for event badges"
  homepage "https://github.com/paulinevos/badgehub-scaffolder"
  license "MIT"

  # Every sha256 below is a placeholder. The release workflow overwrites this
  # whole file once the v0.1.0 archives exist; until then the formula is a
  # shape to review, not one to install.
  on_macos do
    on_arm do
      url "https://github.com/paulinevos/badgehub-scaffolder/releases/download/v0.1.0/bh-0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_AARCH64_APPLE_DARWIN_ARCHIVE"
    end

    on_intel do
      url "https://github.com/paulinevos/badgehub-scaffolder/releases/download/v0.1.0/bh-0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_X86_64_APPLE_DARWIN_ARCHIVE"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/paulinevos/badgehub-scaffolder/releases/download/v0.1.0/bh-0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_AARCH64_UNKNOWN_LINUX_GNU_ARCHIVE"
    end

    on_intel do
      url "https://github.com/paulinevos/badgehub-scaffolder/releases/download/v0.1.0/bh-0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_OF_X86_64_UNKNOWN_LINUX_GNU_ARCHIVE"
    end
  end

  def install
    bin.install "bh"
  end

  # Asserting on clap's own scaffolding rather than on any wording from the
  # tool: a reworded help line should not break a release.
  test do
    assert_match "Usage: bh", shell_output("#{bin}/bh --help")
    assert_match version.to_s, shell_output("#{bin}/bh --version")
  end
end
