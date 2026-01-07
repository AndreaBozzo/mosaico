#!/usr/bin/env bash

# This script runs integration tests for Mosaico
# It starts mosaicod as a background service and runs pytest integration tests against it
#
# Prerequisites:
#   - mosaicod must be built (cargo build)
#   - PostgreSQL must be running (via docker compose or GitHub Actions service)
#   - Python dependencies must be installed (poetry install)
#
# Usage:
#   ./scripts/integration_tests.sh

set -euo pipefail

# Output file for the mosaicod background process
MOSAICOD_OUTPUT="/tmp/mosaicod_e2e_testing.out"
# Directory containing the source code for the SDK
PYTHON_SDK_DIR="mosaico-sdk-py"
# Directory containing the mosaicod source code
MOSAICOD_DIR="mosaicod"
# This directory will be used to configure mosaicod store and will be deleted at the end of the process
TEST_DIRECTORY="/tmp/__mosaico_auto_testing__"
# Log level for mosaicod
RUST_LOG="${RUST_LOG:-mosaico=trace}"
# Database URL
MOSAICO_REPOSITORY_DB_URL="${MOSAICO_REPOSITORY_DB_URL:-postgresql://postgres:password@localhost:6543/mosaico}"
# Useful for rust crashes
RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

FILE_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_DIR=$(readlink -f "${FILE_DIR}/..")
DATABASE_URL="${MOSAICO_REPOSITORY_DB_URL}"
MOSAICOD_PATH="${PROJECT_DIR}/${MOSAICOD_DIR}"
PYTHON_SDK_PATH="${PROJECT_DIR}/${PYTHON_SDK_DIR}"

export DATABASE_URL
export RUST_LOG
export MOSAICO_REPOSITORY_DB_URL
export RUST_BACKTRACE

# Colors (with fallback for non-interactive terminals)
if [ -t 1 ]; then
    RED=$(tput setaf 1)
    GREEN=$(tput setaf 2)
    BLUE=$(tput setaf 4)
    RESET=$(tput sgr0)
    BOLD=$(tput bold)
    DIM=$(tput dim)
else
    RED=""
    GREEN=""
    BLUE=""
    RESET=""
    BOLD=""
    DIM=""
fi

MOSAICOD_PID=""

cleanup() {
    if [ -n "$MOSAICOD_PID" ]; then
        kill "$MOSAICOD_PID" 2>/dev/null || true
        wait "$MOSAICOD_PID" 2>/dev/null || true
        echo "${DIM}mosaicod ($MOSAICOD_PID) terminated.${RESET}"
    fi
    rm -rf "${TEST_DIRECTORY}" 2>/dev/null || true
}

trap cleanup EXIT

echo "${GREEN}${BOLD}=== Integration Tests ===${RESET}"

# Create test directory
mkdir -p "${TEST_DIRECTORY}"

# Start mosaicod
echo "${BLUE}Starting mosaicod...${RESET}"
cd "${MOSAICOD_PATH}"
./target/debug/mosaicod run --port 6276 --local-store "${TEST_DIRECTORY}" > "${MOSAICOD_OUTPUT}" 2>&1 &
MOSAICOD_PID=$!
echo "mosaicod started with PID: ${BOLD}${MOSAICOD_PID}${RESET}"
echo "Output: ${DIM}${MOSAICOD_OUTPUT}${RESET}"

# Wait for mosaicod to be ready
echo "${BLUE}Waiting for mosaicod to be ready...${RESET}"
sleep 5

# Run integration tests
echo "${BLUE}Running pytest integration tests...${RESET}"
cd "${PYTHON_SDK_PATH}"
poetry run pytest ./src/testing -k integration

echo "${GREEN}${BOLD}=== Integration Tests Completed ===${RESET}"
