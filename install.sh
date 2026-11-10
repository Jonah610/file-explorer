
#!/bin/sh
set -eu

REPO="Jonah610/file-explorer"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64|amd64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      aarch64|arm64)
        TARGET="aarch64-unknown-linux-gnu"
        ;;
      *)
        echo "Unsupported Linux architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)
        TARGET="x86_64-apple-darwin"
        ;;
      arm64)
        TARGET="aarch64-apple-darwin"
        ;;
      *)
        echo "Unsupported macOS architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported operating system: $OS"
    echo "Use the Windows installer on Windows."
    exit 1
    ;;
esac

URL="https://github.com/$REPO/releases/latest/download/fe-$TARGET"

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

echo "Downloading fe for $TARGET..."

curl -fL "$URL" -o "$TMP"

mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP" "$INSTALL_DIR/fe"

echo ""
echo "Successfully installed fe!"
echo "Installed to: $INSTALL_DIR/fe"

case ":$PATH:" in
  *":$INSTALL_DIR:"*)
    echo "Run: fe"
    ;;
  *)
    echo ""
    echo "Add this to your shell configuration:"
    echo "export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac
