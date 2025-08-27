#!/bin/bash

# Build script for AKD Watch Docker images
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Change to the repository root
cd "$(dirname "$0")/.."

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed or not in PATH"
    exit 1
fi

# Parse command line arguments
IMAGES_TO_BUILD=()
TAG="latest"
PUSH=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --tag)
            TAG="$2"
            shift 2
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --all)
            IMAGES_TO_BUILD=("aio" "auditor" "web")
            shift
            ;;
        aio|auditor|web)
            IMAGES_TO_BUILD+=("$1")
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS] [IMAGES...]"
            echo ""
            echo "Build Docker images for AKD Watch components"
            echo ""
            echo "IMAGES:"
            echo "  aio      Build the all-in-one image"
            echo "  auditor  Build the auditor-only image"
            echo "  web      Build the web-only image"
            echo ""
            echo "OPTIONS:"
            echo "  --all    Build all images"
            echo "  --tag    Tag for the images (default: latest)"
            echo "  --push   Push images to registry after building"
            echo "  --help   Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 --all                    # Build all images"
            echo "  $0 aio web                  # Build AIO and web images"
            echo "  $0 --tag v1.0.0 --all      # Build all with tag v1.0.0"
            echo "  $0 --tag v1.0.0 --push aio # Build and push AIO image"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Default to building all images if none specified
if [ ${#IMAGES_TO_BUILD[@]} -eq 0 ]; then
    IMAGES_TO_BUILD=("aio" "auditor" "web")
fi

# Build function
build_image() {
    local image_type=$1
    local dockerfile="docker/Dockerfile.$image_type"
    local image_name="akd-watch/$image_type:$TAG"
    
    print_status "Building $image_type image: $image_name"
    
    if ! docker build -f "$dockerfile" -t "$image_name" .; then
        print_error "Failed to build $image_type image"
        return 1
    fi
    
    print_status "Successfully built $image_name"
    
    if [ "$PUSH" = true ]; then
        print_status "Pushing $image_name to registry"
        if ! docker push "$image_name"; then
            print_error "Failed to push $image_name"
            return 1
        fi
        print_status "Successfully pushed $image_name"
    fi
}

# Build each requested image
for image in "${IMAGES_TO_BUILD[@]}"; do
    build_image "$image"
done

print_status "Build process completed!"

# Show built images
print_status "Built images:"
for image in "${IMAGES_TO_BUILD[@]}"; do
    echo "  akd-watch/$image:$TAG"
done

if [ "$PUSH" = false ]; then
    print_warning "Images were not pushed to registry. Use --push flag to push them."
fi
