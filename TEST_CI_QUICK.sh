#!/bin/bash
# Quick script to test GitHub Actions locally with act

echo "🧪 Testing GitHub Actions Locally"
echo "=================================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if act is installed
if ! command -v act &> /dev/null; then
    echo -e "${RED}❌ 'act' is not installed${NC}"
    echo ""
    echo "Install with:"
    echo "  macOS:   brew install act"
    echo "  Linux:   curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash"
    echo ""
    exit 1
fi

echo -e "${GREEN}✅ act is installed${NC}"
echo ""

# Show available workflows
echo "📋 Available workflows:"
act -l -W .github/workflows/build.yml
echo ""

# Menu
echo "Choose what to test:"
echo ""
echo "  1) Dry run (see what would execute)"
echo "  2) Test linting"
echo "  3) Test suite"
echo "  4) Test Linux build"
echo "  5) Test all jobs"
echo "  6) Custom command"
echo ""
read -p "Enter choice [1-6]: " choice

case $choice in
    1)
        echo ""
        echo -e "${YELLOW}Running dry run...${NC}"
        act -W .github/workflows/build.yml -n
        ;;
    2)
        echo ""
        echo -e "${YELLOW}Testing lint job...${NC}"
        act -W .github/workflows/build.yml -j lint
        ;;
    3)
        echo ""
        echo -e "${YELLOW}Testing test job...${NC}"
        act -W .github/workflows/build.yml -j test
        ;;
    4)
        echo ""
        echo -e "${YELLOW}Testing Linux build (this may take a while)...${NC}"
        act -W .github/workflows/build.yml -j build --matrix platform:ubuntu-22.04
        ;;
    5)
        echo ""
        echo -e "${YELLOW}Testing all jobs...${NC}"
        act -W .github/workflows/build.yml push
        ;;
    6)
        echo ""
        read -p "Enter act command (e.g., -j test -v): " cmd
        echo -e "${YELLOW}Running: act -W .github/workflows/build.yml $cmd${NC}"
        act -W .github/workflows/build.yml $cmd
        ;;
    *)
        echo ""
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✅ Test complete!${NC}"
echo ""
echo "💡 Tips:"
echo "  - Run './TEST_CI_QUICK.sh' anytime to test"
echo "  - See TESTING_CI.md for full documentation"
echo "  - Use 'act -v' for verbose output"
echo ""
