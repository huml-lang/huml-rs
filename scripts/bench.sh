#!/bin/bash

# HUML Parser Benchmark Utility Script
# This script provides convenient ways to run different benchmark scenarios

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "HUML Parser Benchmark Utility"
    echo ""
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  all              Run all benchmarks"
    echo "  quick            Run quick benchmarks (components only)"
    echo "  full             Run full document parsing benchmarks"
    echo "  collections      Run collection parsing benchmarks"
    echo "  strings          Run string parsing benchmarks"
    echo "  edge-cases       Run edge case benchmarks"
    echo "  memory           Run memory usage benchmarks"
    echo "  sizes            Run different document size benchmarks"
    echo "  compare          Compare with baseline (if exists)"
    echo "  save-baseline    Save current results as baseline"
    echo "  report           Generate HTML report"
    echo ""
    echo "Options:"
    echo "  --quiet          Suppress verbose output"
    echo "  --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 all           # Run all benchmarks"
    echo "  $0 quick         # Run quick component benchmarks"
    echo "  $0 full --quiet  # Run full document benchmarks quietly"
    echo "  $0 report        # Generate HTML report"
}

# Function to check if we're in the right directory
check_directory() {
    if [ ! -f "Cargo.toml" ] || [ ! -f "test.huml" ]; then
        print_error "This script must be run from the huml-rs project root directory"
        exit 1
    fi
}

# Function to run benchmarks with specific filter
run_benchmark() {
    local filter="$1"
    local description="$2"

    print_info "Running $description benchmarks..."

    if [ "$QUIET" = true ]; then
        cargo bench --quiet "$filter"
    else
        cargo bench "$filter"
    fi

    if [ $? -eq 0 ]; then
        print_success "$description benchmarks completed successfully"
    else
        print_error "$description benchmarks failed"
        exit 1
    fi
}

# Function to generate HTML report
generate_report() {
    print_info "Generating HTML benchmark report..."

    if command -v criterion-html &> /dev/null; then
        criterion-html target/criterion/
        print_success "HTML report generated in target/criterion/"
    else
        print_warning "criterion-html not found. Installing..."
        cargo install criterion-plot
        if [ $? -eq 0 ]; then
            criterion-html target/criterion/
            print_success "HTML report generated in target/criterion/"
        else
            print_error "Failed to install criterion-plot"
            exit 1
        fi
    fi
}

# Function to save baseline
save_baseline() {
    print_info "Saving current benchmark results as baseline..."

    if [ -d "target/criterion" ]; then
        cp -r target/criterion target/criterion-baseline
        print_success "Baseline saved to target/criterion-baseline"
    else
        print_error "No benchmark results found. Run benchmarks first."
        exit 1
    fi
}

# Function to compare with baseline
compare_baseline() {
    print_info "Comparing current results with baseline..."

    if [ ! -d "target/criterion-baseline" ]; then
        print_error "No baseline found. Run 'save-baseline' first."
        exit 1
    fi

    # Run current benchmarks
    run_benchmark "" "comparison"

    # Simple comparison (you could enhance this with more sophisticated comparison)
    print_info "Baseline comparison completed. Check target/criterion/ for detailed results."
}

# Parse command line arguments
QUIET=false
COMMAND=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --quiet)
            QUIET=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        all|quick|full|collections|strings|edge-cases|memory|sizes|compare|save-baseline|report)
            COMMAND="$1"
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Check if we have a command
if [ -z "$COMMAND" ]; then
    print_error "No command specified"
    show_usage
    exit 1
fi

# Check directory
check_directory

# Execute command
case $COMMAND in
    all)
        print_info "Running all benchmarks..."
        run_benchmark "" "all"
        ;;
    quick)
        run_benchmark "parse_components" "component parsing"
        ;;
    full)
        run_benchmark "parse_full_huml_document" "full document parsing"
        ;;
    collections)
        run_benchmark "parse_collections" "collection parsing"
        ;;
    strings)
        run_benchmark "parse_multiline_strings" "string parsing"
        ;;
    edge-cases)
        run_benchmark "parse_edge_cases" "edge case"
        ;;
    memory)
        run_benchmark "memory_usage" "memory usage"
        ;;
    sizes)
        run_benchmark "different_sizes" "document size"
        ;;
    compare)
        compare_baseline
        ;;
    save-baseline)
        save_baseline
        ;;
    report)
        generate_report
        ;;
    *)
        print_error "Unknown command: $COMMAND"
        show_usage
        exit 1
        ;;
esac

print_success "Benchmark operation completed successfully!"
