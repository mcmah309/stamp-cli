#!/usr/bin/env bash 
set -euo pipefail

copier copy ~/templates/axum_server $1
copier copy ~/templates/devcontainers/rust $1