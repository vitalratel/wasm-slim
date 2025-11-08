#!/bin/bash
# Complete CI validation using act (GitHub Actions local runner)
# This runs the actual GitHub Actions workflows locally

set -e

echo "🚀 Running complete CI validation with act..."
echo ""

# Check if act is installed
if ! command -v act &> /dev/null; then
    echo "❌ Error: 'act' is not installed"
    echo ""
    echo "Install act:"
    echo "  Linux:  curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash"
    echo "  macOS:  brew install act"
    echo "  Manual: https://github.com/nektos/act"
    exit 1
fi

# Check if Docker or Podman is running
CONTAINER_RUNTIME=""

if command -v podman &> /dev/null; then
    # Podman is available
    if podman info &> /dev/null; then
        CONTAINER_RUNTIME="podman"

        # Check if podman-docker compatibility is set up
        if ! command -v docker &> /dev/null; then
            echo "💡 Tip: Create docker alias for podman:"
            echo "   alias docker=podman"
            echo "   OR install: sudo dnf install podman-docker"
            echo ""
        fi

        # act works with podman via docker socket emulation
        if [ ! -S "/run/user/$UID/podman/podman.sock" ]; then
            echo "⚠️  Starting podman socket for act compatibility..."
            systemctl --user start podman.socket || true
        fi

        # Set Docker host to use podman socket
        export DOCKER_HOST="unix:///run/user/$UID/podman/podman.sock"
    else
        echo "❌ Error: Podman is not running"
        echo "   Start podman: systemctl --user start podman.socket"
        exit 1
    fi
elif command -v docker &> /dev/null; then
    # Docker is available
    if ! docker info &> /dev/null; then
        echo "❌ Error: Docker is not running"
        echo "   Start Docker and try again"
        exit 1
    fi
    CONTAINER_RUNTIME="docker"
else
    echo "❌ Error: Neither Docker nor Podman is installed"
    echo ""
    echo "Install one of:"
    echo "  Docker:  https://docs.docker.com/get-docker/"
    echo "  Podman:  sudo dnf install podman (Fedora/RHEL)"
    exit 1
fi

echo "✅ Using container runtime: $CONTAINER_RUNTIME"
echo ""

echo "📋 Available workflows:"
act -l
echo ""

# Ask user which workflows to run
echo "Select workflows to run:"
echo "  1) All workflows (slowest, most thorough)"
echo "  2) Tests only (recommended)"
echo "  3) Tests + Clippy + Coverage"
echo "  4) Custom selection"
echo ""
read -p "Enter choice [1-4] (default: 2): " choice
choice=${choice:-2}

case $choice in
    1)
        echo ""
        echo "🏃 Running ALL workflows..."
        echo "⏱️  This will take 15-30 minutes..."
        echo ""
        act -j test || exit 1
        act -j clippy || exit 1
        act -j coverage || exit 1
        act -j benchmarks || exit 1
        ;;
    2)
        echo ""
        echo "🏃 Running test workflow..."
        echo "⏱️  This will take 5-10 minutes..."
        echo ""
        act -j test || exit 1
        ;;
    3)
        echo ""
        echo "🏃 Running tests, clippy, and coverage..."
        echo "⏱️  This will take 10-15 minutes..."
        echo ""
        act -j test || exit 1
        act -j clippy || exit 1
        act -j coverage || exit 1
        ;;
    4)
        echo ""
        echo "Available jobs:"
        act -l
        echo ""
        read -p "Enter job name (e.g., 'test'): " job_name
        echo "🏃 Running job: $job_name"
        act -j "$job_name" || exit 1
        ;;
    *)
        echo "❌ Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Complete CI validation passed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "💡 Your code is ready to push to GitHub!"
