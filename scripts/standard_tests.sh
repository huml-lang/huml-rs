#!/bin/bash

# Script to run the standard HUML test suite
# This script initializes git submodules if needed and runs the standard tests

set -e

echo "🧪 HUML Standard Test Suite Runner"
echo "=================================="

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: This script must be run from the project root directory"
    exit 1
fi

# Check if git submodules are initialized
if [ ! -f "tests/README.md" ]; then
    echo "📦 Initializing git submodules..."
    git submodule init
    git submodule update
    echo "✅ Submodules initialized"
else
    echo "✅ Git submodules already initialized"
fi

# Update submodules to latest
echo "🔄 Updating submodules to latest..."
git submodule update --remote

echo ""
echo "🏃 Running standard tests..."
echo ""

# Run the standard tests with output
cargo test standard_tests -- --nocapture

echo ""
echo "📊 Test Summary:"
echo "==============="
echo "✅ Document parsing test should pass"
echo "⚠️  Some assertion tests may fail (this is expected)"
echo ""
echo "ℹ️  Failing assertion tests indicate areas where the parser"
echo "   can be improved to better comply with the HUML specification."
echo ""
echo "📋 To see detailed results, check the output above."
echo "🔧 To run all tests: cargo test"
echo "🎯 To run only assertion tests: cargo test test_standard_assertions -- --nocapture"
echo "📄 To run only document tests: cargo test test_standard_documents -- --nocapture"
